use crate::{error::reject, http};
use actix_web::{
    HttpRequest,
    body::MessageBody,
    dev::{ServiceRequest, ServiceResponse},
    error::ErrorForbidden,
    http::{
        Method,
        header::{self, HeaderMap},
    },
    middleware::Next,
};
use reqwest::Url;
use std::{
    net::{IpAddr, Ipv6Addr, SocketAddr},
    sync::LazyLock,
};

static CLOUDFLARE_ONLY: LazyLock<bool> = LazyLock::new(|| {
    !crate::env::var("CLOUDFLARE_ONLY").is_some_and(|value| value.eq_ignore_ascii_case("N"))
});

const CLOUDFLARE_RANGES: [&str; 22] = [
    "173.245.48.0/20",
    "103.21.244.0/22",
    "103.22.200.0/22",
    "103.31.4.0/22",
    "141.101.64.0/18",
    "108.162.192.0/18",
    "190.93.240.0/20",
    "188.114.96.0/20",
    "197.234.240.0/22",
    "198.41.128.0/17",
    "162.158.0.0/15",
    "104.16.0.0/13",
    "104.24.0.0/14",
    "172.64.0.0/13",
    "131.0.72.0/22",
    "2400:cb00::/32",
    "2606:4700::/32",
    "2803:f800::/32",
    "2405:b500::/32",
    "2405:8100::/32",
    "2a06:98c0::/29",
    "2c0f:f248::/32",
];

fn in_range(ip: IpAddr, range: &str) -> bool {
    let Some((net, len)) = range.split_once('/') else {
        return false;
    };
    let Ok(len) = len.parse::<u32>() else {
        return false;
    };
    match (ip.to_canonical(), net.parse()) {
        (IpAddr::V4(ip), Ok(IpAddr::V4(net))) => {
            (u32::from(ip) ^ u32::from(net))
                .checked_shr(32 - len)
                .unwrap_or(0)
                == 0
        }
        (IpAddr::V6(ip), Ok(IpAddr::V6(net))) => {
            (u128::from(ip) ^ u128::from(net))
                .checked_shr(128 - len)
                .unwrap_or(0)
                == 0
        }
        _ => false,
    }
}

fn is_cloudflare(ip: IpAddr) -> bool {
    CLOUDFLARE_RANGES.iter().any(|range| in_range(ip, range))
}

fn parse_addr(addr: &str) -> Option<IpAddr> {
    let addr = addr.trim();
    addr.parse::<IpAddr>()
        .ok()
        .or_else(|| addr.parse::<SocketAddr>().ok().map(|addr| addr.ip()))
        .or_else(|| addr.strip_prefix('[')?.strip_suffix(']')?.parse().ok())
}

fn forwarded_for(req: &HttpRequest) -> Option<Option<IpAddr>> {
    let mut values = req.headers().get_all("x-forwarded-for").peekable();
    values.peek()?;
    Some(
        values
            .map(|value| value.to_str().ok())
            .collect::<Option<Vec<_>>>()
            .and_then(|values| parse_addr(values.join(",").rsplit(',').next()?)),
    )
}

enum Edge {
    Known(IpAddr),
    Missing,
    Unreadable,
}

fn edge(req: &HttpRequest) -> Edge {
    match req.peer_addr().map(|addr| addr.ip()).filter(|ip| http::is_public(*ip)) {
        Some(ip) => Edge::Known(ip),
        None => match forwarded_for(req) {
            None => Edge::Missing,
            Some(Some(ip)) => Edge::Known(ip),
            Some(None) => Edge::Unreadable,
        },
    }
}

fn edge_ip(req: &HttpRequest) -> Option<IpAddr> {
    match edge(req) {
        Edge::Known(ip) => Some(ip),
        Edge::Missing | Edge::Unreadable => None,
    }
}

pub fn client_ip(req: &HttpRequest) -> Option<IpAddr> {
    let edge = edge_ip(req);
    edge.filter(|ip| is_cloudflare(*ip))
        .and_then(|_| {
            req.headers()
                .get("cf-connecting-ip")
                .and_then(|value| value.to_str().ok())
                .and_then(|addr| addr.trim().parse::<IpAddr>().ok())
        })
        .or(edge)
        .or_else(|| req.peer_addr().map(|addr| addr.ip()))
        .map(|ip| match ip.to_canonical() {
            IpAddr::V6(ip) => IpAddr::V6(Ipv6Addr::from(u128::from(ip) & !0 << 64)),
            ip => ip,
        })
}

pub async fn cloudflare_only(
    req: ServiceRequest,
    next: Next<impl MessageBody>,
) -> Result<ServiceResponse<impl MessageBody>, actix_web::Error> {
    let blocked = match edge(req.request()) {
        Edge::Known(ip) => !is_cloudflare(ip),
        Edge::Missing => false,
        Edge::Unreadable => true,
    };
    if *CLOUDFLARE_ONLY && blocked {
        return Err(ErrorForbidden("Forbidden"));
    }
    next.call(req).await
}

pub async fn cache_control(
    req: ServiceRequest,
    next: Next<impl MessageBody>,
) -> Result<ServiceResponse<impl MessageBody>, actix_web::Error> {
    let immutable = req.path().starts_with("/_app/immutable/");
    let mut res = next.call(req).await?;
    let immutable = immutable && res.status().is_success();
    let headers = res.headers_mut();
    if !headers.contains_key(header::CACHE_CONTROL) {
        let html = headers
            .get(header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .is_some_and(|value| value.starts_with("text/html"));
        let value = match (immutable, html) {
            (true, _) => Some("public, max-age=31536000, immutable"),
            (false, true) => Some("no-cache"),
            (false, false) => None,
        };
        if let Some(value) = value {
            headers.insert(header::CACHE_CONTROL, header::HeaderValue::from_static(value));
        }
    }
    Ok(res)
}

fn allowed_origin(origin: &str, base_url: &str) -> bool {
    Url::parse(origin).is_ok_and(|origin| {
        let host = origin.host_str().unwrap_or_default();
        let secure = match origin.scheme() {
            "https" => true,
            "http" => matches!(host, "localhost" | "127.0.0.1" | "[::1]"),
            _ => false,
        };
        let host = match origin.port() {
            Some(port) => format!("{host}:{port}"),
            None => host.to_string(),
        };
        secure && host.eq_ignore_ascii_case(base_url)
    })
}

fn cross_site(headers: &HeaderMap, base_url: &str) -> bool {
    match headers.get(header::ORIGIN) {
        Some(origin) => !origin.to_str().is_ok_and(|origin| allowed_origin(origin, base_url)),
        None => headers
            .get("sec-fetch-site")
            .is_some_and(|site| matches!(site.as_bytes(), b"cross-site" | b"same-site")),
    }
}

pub async fn same_origin(
    req: ServiceRequest,
    next: Next<impl MessageBody>,
) -> Result<ServiceResponse<impl MessageBody>, actix_web::Error> {
    if !matches!(*req.method(), Method::GET | Method::HEAD) && cross_site(req.headers(), &hc_helper::keys::base_url()) {
        return Err(reject("Cross-site request blocked. Reload the page and try again").into());
    }
    next.call(req).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{App, HttpResponse, http::StatusCode, test, web};
    use std::net::{Ipv4Addr, SocketAddr};

    #[actix_web::test]
    async fn only_cloudflare_edges_are_allowed() {
        let app = test::init_service(
            App::new()
                .wrap(actix_web::middleware::from_fn(cloudflare_only))
                .route("/", web::get().to(HttpResponse::Ok)),
        )
        .await;
        let proxy = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 12345);

        for (forwarded, expected) in [
            (&[][..], StatusCode::OK),
            (&["203.0.113.1, 162.158.1.2"][..], StatusCode::OK),
            (&["2606:4700::1"][..], StatusCode::OK),
            (&["203.0.113.1, 162.158.1.2:443"][..], StatusCode::OK),
            (&["[2606:4700::1]:443"][..], StatusCode::OK),
            (&["[2606:4700::1]"][..], StatusCode::OK),
            (&["203.0.113.1", "162.158.1.2"][..], StatusCode::OK),
            (&["162.158.1.2", "203.0.113.1"][..], StatusCode::FORBIDDEN),
            (&["203.0.113.1"][..], StatusCode::FORBIDDEN),
            (&["162.158.1.2, 203.0.113.1"][..], StatusCode::FORBIDDEN),
            (&["unknown"][..], StatusCode::FORBIDDEN),
            (&["162.158.1.2, "][..], StatusCode::FORBIDDEN),
        ] {
            let mut request = test::TestRequest::get().uri("/").peer_addr(proxy);
            for value in forwarded {
                request = request.append_header(("x-forwarded-for", *value));
            }
            let status = match test::try_call_service(&app, request.to_request()).await {
                Ok(response) => response.status(),
                Err(e) => e.as_response_error().status_code(),
            };
            assert_eq!(status, expected, "{forwarded:?}");
        }
    }

    #[actix_web::test]
    async fn origins_must_match_scheme_and_host() {
        assert!(allowed_origin("https://mipmap.hackclub.com", "mipmap.hackclub.com"));
        assert!(!allowed_origin("http://mipmap.hackclub.com", "mipmap.hackclub.com"));
        assert!(!allowed_origin("https://evil.com", "mipmap.hackclub.com"));
        assert!(!allowed_origin("null", "mipmap.hackclub.com"));
        assert!(allowed_origin("http://localhost:8080", "localhost:8080"));
        assert!(allowed_origin("http://[::1]:8080", "[::1]:8080"));
    }

    #[actix_web::test]
    async fn missing_origin_uses_fetch_metadata() {
        let headers = |pairs: &[(&'static str, &'static str)]| {
            let mut headers = HeaderMap::new();
            for (name, value) in pairs {
                headers.insert(header::HeaderName::from_static(name), header::HeaderValue::from_static(value));
            }
            headers
        };
        let base = "mipmap.hackclub.com";
        assert!(!cross_site(&headers(&[]), base));
        assert!(!cross_site(&headers(&[("sec-fetch-site", "same-origin")]), base));
        assert!(!cross_site(&headers(&[("sec-fetch-site", "none")]), base));
        assert!(cross_site(&headers(&[("sec-fetch-site", "cross-site")]), base));
        assert!(cross_site(&headers(&[("sec-fetch-site", "same-site")]), base));
        assert!(cross_site(&headers(&[("origin", "http://mipmap.hackclub.com")]), base));
        assert!(!cross_site(&headers(&[("origin", "https://mipmap.hackclub.com")]), base));
    }

    #[actix_web::test]
    async fn client_ip_trusts_cf_connecting_ip_only_from_cloudflare() {
        let proxy = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 12345);
        let request = |forwarded| {
            test::TestRequest::get()
                .peer_addr(proxy)
                .insert_header(("x-forwarded-for", forwarded))
                .insert_header(("cf-connecting-ip", "198.51.100.7"))
                .to_http_request()
        };

        assert_eq!(
            client_ip(&request("162.158.1.2")),
            "198.51.100.7".parse().ok()
        );
        assert_eq!(
            client_ip(&request("203.0.113.1")),
            "203.0.113.1".parse().ok()
        );
    }
}
