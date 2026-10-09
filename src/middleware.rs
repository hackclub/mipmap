use crate::{error::reject, http};
use actix_web::{
    HttpRequest,
    body::MessageBody,
    dev::{ServiceRequest, ServiceResponse},
    error::ErrorForbidden,
    http::{Method, header},
    middleware::Next,
};
use reqwest::Url;
use std::{
    net::{IpAddr, Ipv6Addr},
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

fn edge_ip(req: &HttpRequest) -> Option<IpAddr> {
    req.peer_addr()
        .map(|addr| addr.ip())
        .filter(|ip| http::is_public(*ip))
        .or_else(|| {
            req.headers()
                .get("x-forwarded-for")
                .and_then(|value| value.to_str().ok())
                .and_then(|list| list.rsplit(',').next())
                .and_then(|addr| addr.trim().parse::<IpAddr>().ok())
        })
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
    if *CLOUDFLARE_ONLY && edge_ip(req.request()).is_some_and(|ip| !is_cloudflare(ip)) {
        return Err(ErrorForbidden("Forbidden"));
    }
    next.call(req).await
}

pub async fn same_origin(
    req: ServiceRequest,
    next: Next<impl MessageBody>,
) -> Result<ServiceResponse<impl MessageBody>, actix_web::Error> {
    let cross_site = !matches!(*req.method(), Method::GET | Method::HEAD)
        && req.headers().get(header::ORIGIN).is_some_and(|origin| {
            origin
                .to_str()
                .ok()
                .and_then(|origin| Url::parse(origin).ok())
                .is_none_or(|origin| {
                    let host = origin.host_str().unwrap_or_default();
                    let host = match origin.port() {
                        Some(port) => format!("{host}:{port}"),
                        None => host.to_string(),
                    };
                    !host.eq_ignore_ascii_case(&hc_helper::keys::base_url())
                })
        });
    if cross_site {
        return Err(reject("Cross-site request blocked").into());
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
            (None, StatusCode::OK),
            (Some("203.0.113.1, 162.158.1.2"), StatusCode::OK),
            (Some("2606:4700::1"), StatusCode::OK),
            (Some("203.0.113.1"), StatusCode::FORBIDDEN),
            (Some("162.158.1.2, 203.0.113.1"), StatusCode::FORBIDDEN),
        ] {
            let mut request = test::TestRequest::get().uri("/").peer_addr(proxy);
            if let Some(forwarded) = forwarded {
                request = request.insert_header(("x-forwarded-for", forwarded));
            }
            let status = match test::try_call_service(&app, request.to_request()).await {
                Ok(response) => response.status(),
                Err(e) => e.as_response_error().status_code(),
            };
            assert_eq!(status, expected, "{forwarded:?}");
        }
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
