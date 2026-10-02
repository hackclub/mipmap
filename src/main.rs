use actix_files::Files;
use actix_governor::{Governor, GovernorConfigBuilder, KeyExtractor, SimpleKeyExtractionError};
use actix_web::{App, HttpServer, Responder, dev::ServiceRequest, guard, web};
use serde::Serialize;
use std::{
    net::{IpAddr, SocketAddr},
    time::Duration,
};

mod airtable_sync;
mod dashboard;
mod database;
mod layout;
mod login;
mod profile;
mod progress;
mod projects;
mod referral;
mod snapshots;
mod tokens;
mod tree;

const FRONTEND_DIR: &str = "frontend/build";

#[derive(Serialize)]
struct Hello {
    message: String,
}

#[derive(Clone)]
struct ClientIpKeyExtractor;

impl KeyExtractor for ClientIpKeyExtractor {
    type Key = IpAddr;
    type KeyExtractionError = SimpleKeyExtractionError<&'static str>;

    fn extract(&self, req: &ServiceRequest) -> Result<Self::Key, Self::KeyExtractionError> {
        req.connection_info()
            .realip_remote_addr()
            .and_then(|addr| {
                addr.parse::<IpAddr>()
                    .ok()
                    .or_else(|| addr.parse::<SocketAddr>().ok().map(|socket| socket.ip()))
            })
            .ok_or_else(|| SimpleKeyExtractionError::new("Could not extract client IP address"))
    }
}

async fn hello() -> impl Responder {
    web::Json(Hello {
        message: "hello from actix".to_string(),
    })
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenvy::dotenv().ok();
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let database_url = std::env::var("DATABASE_URL").map_err(|_| {
        std::io::Error::new(std::io::ErrorKind::NotFound, "DATABASE_URL must be set")
    })?;
    let database_pool = database::init(&database_url)
        .await
        .map_err(std::io::Error::other)?;
    log::info!("PostgreSQL ready");
    airtable_sync::spawn(database_pool.clone()).map_err(std::io::Error::other)?;
    let database_pool = web::Data::new(database_pool);

    log::info!("listening on http://0.0.0.0:8080");

    let rate_limit = GovernorConfigBuilder::default()
        .key_extractor(ClientIpKeyExtractor)
        .seconds_per_request(1)
        .burst_size(10)
        .use_headers()
        .finish()
        .expect("rate limit configuration must be valid");

    let limiter = rate_limit.limiter();
    actix_web::rt::spawn(async move {
        let mut interval = actix_web::rt::time::interval(Duration::from_secs(60));
        loop {
            interval.tick().await;
            limiter.retain_recent();
            limiter.shrink_to_fit();
        }
    });

    HttpServer::new(move || {
        App::new()
            .app_data(database_pool.clone())
            .service(
                web::scope("/api")
                    .wrap(Governor::new(&rate_limit))
                    .route("/hello", web::get().to(hello))
                    .service(progress::get_progress)
                    .service(progress::unlock_item)
                    .service(progress::purchase_item)
                    .service(projects::list_projects)
                    .service(projects::create_project)
                    .service(projects::upload_screenshot)
                    .service(projects::update_project)
                    .service(snapshots::my_snapshots)
                    .service(snapshots::status)
                    .service(snapshots::list_snapshots)
                    .service(snapshots::create_snapshot)
                    .service(referral::get_referral)
                    .service(profile::get_profile),
            )
            .service(
                web::scope("")
                    .guard(guard::fn_guard(|ctx| {
                        matches!(ctx.head().uri.path(), "/login" | "/callback")
                    }))
                    .wrap(Governor::new(&rate_limit))
                    .service(login::start)
                    .service(login::callback),
            )
            .service(dashboard::page)
            .service(dashboard::tree)
            .service(dashboard::layout)
            .service(dashboard::fulfillment_rounds)
            .service(dashboard::hackatime_projects)
            .service(Files::new("/", FRONTEND_DIR).index_file("index.html"))
    })
    .bind(("0.0.0.0", 8080))?
    .run()
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{http::StatusCode, test};
    use std::net::Ipv4Addr;

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
                    .route("/hello", web::get().to(hello)),
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
                    .route("/hello", web::get().to(hello)),
            ),
        )
        .await;
        let proxy = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 12345);

        for (client, expected) in [
            ("203.0.113.1", StatusCode::OK),
            ("203.0.113.2", StatusCode::OK),
            ("203.0.113.1", StatusCode::TOO_MANY_REQUESTS),
        ] {
            let request = test::TestRequest::get()
                .uri("/api/hello")
                .peer_addr(proxy)
                .insert_header(("x-forwarded-for", client))
                .to_request();
            let response = test::call_service(&app, request).await;
            assert_eq!(response.status(), expected);
        }
    }
}
