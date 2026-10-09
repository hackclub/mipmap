use crate::{error::reject, http};
use actix_governor::{KeyExtractor, SimpleKeyExtractionError};
use actix_web::{
    HttpRequest,
    body::MessageBody,
    dev::{ServiceRequest, ServiceResponse},
    http::{Method, header},
    middleware::Next,
};
use reqwest::Url;
use std::net::{IpAddr, Ipv6Addr};

#[derive(Clone)]
pub struct ClientIpKeyExtractor;

impl KeyExtractor for ClientIpKeyExtractor {
    type Key = IpAddr;
    type KeyExtractionError = SimpleKeyExtractionError<&'static str>;

    fn extract(&self, req: &ServiceRequest) -> Result<Self::Key, Self::KeyExtractionError> {
        client_ip(req.request())
            .ok_or_else(|| SimpleKeyExtractionError::new("Could not extract client IP address"))
    }
}

pub fn client_ip(req: &HttpRequest) -> Option<IpAddr> {
    let peer = req.peer_addr().map(|addr| addr.ip());
    peer.filter(|ip| http::is_public(*ip))
        .or_else(|| {
            req.headers()
                .get("x-forwarded-for")
                .and_then(|value| value.to_str().ok())
                .and_then(|list| list.rsplit(',').next())
                .and_then(|addr| addr.trim().parse::<IpAddr>().ok())
        })
        .or(peer)
        .map(|ip| match ip.to_canonical() {
            IpAddr::V6(ip) => IpAddr::V6(Ipv6Addr::from(u128::from(ip) & !0 << 64)),
            ip => ip,
        })
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
    use actix_governor::{Governor, GovernorConfigBuilder};
    use actix_web::{App, HttpResponse, http::StatusCode, test, web};
    use std::net::{Ipv4Addr, SocketAddr};

    #[actix_web::test]
    async fn rate_limit_rejects_requests_after_burst_is_exhausted() {
        let rate_limit = GovernorConfigBuilder::default()
            .key_extractor(ClientIpKeyExtractor)
            .seconds_per_request(60)
            .burst_size(2)
            .use_headers()
            .finish()
            .unwrap();
        let app = test::init_service(
            App::new().service(
                web::scope("/api")
                    .wrap(Governor::new(&rate_limit))
                    .route("/hello", web::get().to(HttpResponse::Ok)),
            ),
        )
        .await;
        let peer = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 12345);

        for _ in 0..2 {
            let request = test::TestRequest::get()
                .uri("/api/hello")
                .peer_addr(peer)
                .to_request();
            let response = test::call_service(&app, request).await;
            assert_eq!(response.status(), StatusCode::OK);
        }

        let request = test::TestRequest::get()
            .uri("/api/hello")
            .peer_addr(peer)
            .to_request();
        let response = test::call_service(&app, request).await;

        assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
        assert!(response.headers().contains_key("retry-after"));
    }

    #[actix_web::test]
    async fn rate_limit_is_tracked_per_forwarded_client() {
        let rate_limit = GovernorConfigBuilder::default()
            .key_extractor(ClientIpKeyExtractor)
            .seconds_per_request(60)
            .burst_size(1)
            .finish()
            .unwrap();
        let app = test::init_service(
            App::new().service(
                web::scope("/api")
                    .wrap(Governor::new(&rate_limit))
                    .route("/hello", web::get().to(HttpResponse::Ok)),
            ),
        )
        .await;
        let proxy = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 12345);

        for (headers, expected) in [
            (vec![("x-forwarded-for", "203.0.113.1")], StatusCode::OK),
            (vec![("x-forwarded-for", "203.0.113.2")], StatusCode::OK),
            (vec![("x-forwarded-for", "198.51.100.7, 203.0.113.1")], StatusCode::TOO_MANY_REQUESTS),
            (
                vec![("cf-connecting-ip", "203.0.113.3"), ("x-forwarded-for", "203.0.113.1")],
                StatusCode::TOO_MANY_REQUESTS,
            ),
        ] {
            let mut request = test::TestRequest::get().uri("/api/hello").peer_addr(proxy);
            for header in headers {
                request = request.insert_header(header);
            }
            let request = request.to_request();
            let response = test::call_service(&app, request).await;
            assert_eq!(response.status(), expected);
        }
    }
}
