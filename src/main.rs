use actix_files::{Files, NamedFile};
use actix_multipart::form::MultipartFormConfig;
use actix_web::{
    App, HttpRequest, HttpResponse, HttpServer, get,
    middleware::{DefaultHeaders, from_fn},
    web,
};

mod accounts;
mod dashboard;
mod database;
mod env;
mod error;
mod feedback;
mod http;
mod middleware;
mod mirror;
mod projects;
mod snapshots;
mod stats;
mod tree;
mod uploads;

const FRONTEND_DIR: &str = "frontend/build";
const UPLOAD_LIMIT: usize = 10 * 1024 * 1024;
const CSP: &str = "default-src 'self'; script-src 'self' 'unsafe-inline' https://plausible.io; \
    style-src 'self' 'unsafe-inline' https://fonts.googleapis.com; font-src 'self' https://fonts.gstatic.com; \
    img-src 'self' data: blob: https:; connect-src 'self' https://plausible.io; object-src 'none'; \
    base-uri 'self'; form-action 'self'; frame-ancestors 'none'; frame-src https://www.youtube-nocookie.com";

async fn html(req: &HttpRequest, page: &str) -> HttpResponse {
    match NamedFile::open_async(format!("{FRONTEND_DIR}/{page}.html")).await {
        Ok(file) => file.into_response(req),
        Err(_) => HttpResponse::NotFound().finish(),
    }
}

#[get("/docs")]
async fn docs(req: HttpRequest) -> HttpResponse {
    html(&req, "docs").await
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
    tree::backfill_prices(&database_pool)
        .await
        .map_err(std::io::Error::other)?;
    snapshots::backfill_name_keys(&database_pool)
        .await
        .map_err(std::io::Error::other)?;
    hc_helper::airtable::spawn_airtable_queue_handler().map_err(std::io::Error::other)?;
    mirror::spawn(database_pool.clone());
    projects::review::spawn(database_pool.clone());
    stats::spawn(database_pool.clone());
    tree::spawn_images();
    uploads::warn_unconfigured();
    let database_pool = web::Data::new(database_pool);

    log::info!("listening on http://0.0.0.0:8080");

    HttpServer::new(move || {
        App::new()
            .app_data(database_pool.clone())
            .wrap(from_fn(middleware::cloudflare_only))
            .wrap(
                DefaultHeaders::new()
                    .add(("X-Content-Type-Options", "nosniff"))
                    .add(("X-Frame-Options", "DENY"))
                    .add(("Content-Security-Policy", CSP))
                    .add(("Permissions-Policy", "camera=(), microphone=(), geolocation=()"))
                    .add(("Referrer-Policy", "strict-origin-when-cross-origin"))
                    .add(("Strict-Transport-Security", "max-age=31536000")),
            )
            .service(
                web::scope("/api")
                    .app_data(web::PayloadConfig::new(UPLOAD_LIMIT))
                    .app_data(MultipartFormConfig::default().memory_limit(UPLOAD_LIMIT))
                    .wrap(from_fn(middleware::same_origin))
                    .configure(dashboard::api)
                    .configure(tree::routes)
                    .configure(projects::routes)
                    .configure(stats::api)
                    .service(snapshots::create_snapshot)
                    .service(feedback::create_feedback)
                    .service(accounts::session::logout),
            )
            .configure(accounts::routes)
            .service(web::scope("/dashboard/item-images").service(tree::item_image))
            .configure(dashboard::pages)
            .configure(stats::pages)
            .service(docs)
            .service(Files::new("/", FRONTEND_DIR).index_file("index.html"))
    })
    .bind(("0.0.0.0", 8080))?
    .run()
    .await
}
