use actix_files::{Files, NamedFile};
use actix_multipart::form::MultipartFormConfig;
use actix_web::{
    App, HttpRequest, HttpResponse, HttpServer,
    dev::{ServiceRequest, ServiceResponse, fn_service},
    get,
    http::header,
    middleware::{DefaultHeaders, from_fn},
    web,
};

mod accounts;
mod admin;
mod dashboard;
mod database;
mod env;
mod error;
mod feedback;
mod http;
mod locations;
mod middleware;
mod mirror;
mod projects;
mod slack;
mod snapshots;
mod stats;
mod surveys;
mod tree;
mod uploads;

const FRONTEND_DIR: &str = "frontend/build";
const UPLOAD_LIMIT: usize = 10 * 1024 * 1024;
const CSP: &str = "default-src 'self'; script-src 'self' 'unsafe-inline' https://plausible.io; \
    style-src 'self' 'unsafe-inline'; font-src 'self'; \
    img-src 'self' data: blob: https:; connect-src 'self' https://plausible.io; object-src 'none'; \
    base-uri 'self'; form-action 'self'; frame-ancestors 'none'; frame-src https://www.youtube-nocookie.com";

const NOT_FOUND: &str = "<!doctype html>
<html lang=\"en\">
<head>
<meta charset=\"utf-8\">
<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">
<title>Not found · Mipmap</title>
<link rel=\"stylesheet\" href=\"/fonts/fonts.css\">
<style>
body { margin: 0; min-height: 100vh; display: grid; place-items: center; background-color: #3d3029; color: #ead7c2;
    background-image: radial-gradient(#5f4838 1.5px, transparent 1.5px); background-size: 24px 24px;
    font-family: 'Sansation', sans-serif; text-align: center; padding: 16px; box-sizing: border-box; }
h1 { margin: 0 0 8px; font-family: 'Phudu', sans-serif; color: #d6ab70; }
a { color: #c8d5f3; }
a:hover, a:focus { color: #ead7c2; }
</style>
</head>
<body>
<main>
<h1>Page not found</h1>
<p>There's nothing at this address.</p>
<p><a href=\"/\">Go to the Mipmap home page</a></p>
</main>
</body>
</html>
";

fn not_found_page() -> HttpResponse {
    HttpResponse::NotFound().content_type("text/html; charset=utf-8").body(NOT_FOUND)
}

async fn html(req: &HttpRequest, page: &str) -> HttpResponse {
    match NamedFile::open_async(format!("{FRONTEND_DIR}/{page}.html")).await {
        Ok(file) => file.into_response(req),
        Err(_) => not_found_page(),
    }
}

#[get("/docs")]
async fn docs(req: HttpRequest) -> HttpResponse {
    html(&req, "docs").await
}

async fn trailing_slash(req: HttpRequest) -> HttpResponse {
    let path = req.path().trim_end_matches('/');
    let location = match req.query_string() {
        "" => path.to_string(),
        query => format!("{path}?{query}"),
    };
    HttpResponse::PermanentRedirect().insert_header((header::LOCATION, location)).finish()
}

async fn not_found(req: ServiceRequest) -> Result<ServiceResponse, actix_web::Error> {
    Ok(req.into_response(not_found_page()))
}

fn startup_error(message: &str) -> std::io::Error {
    log::error!("{message}");
    std::io::Error::other(message.to_string())
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenvy::dotenv().ok();
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    if env::var("BASE_URL").is_none_or(|base_url| base_url.contains('/')) {
        return Err(startup_error(
            "BASE_URL must be set to the site's host, like mipmap.hackclub.com (no https:// or path)",
        ));
    }
    accounts::tokens::check().map_err(|e| startup_error(&format!("{e}")))?;

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
    projects::ship::spawn(database_pool.clone());
    stats::spawn(database_pool.clone());
    slack::spawn(database_pool.clone());
    tree::spawn_images();
    uploads::warn_unconfigured();
    let database_pool = web::Data::new(database_pool);

    log::info!("listening on http://0.0.0.0:8080");

    HttpServer::new(move || {
        App::new()
            .app_data(database_pool.clone())
            .wrap(from_fn(middleware::cache_control))
            .wrap(from_fn(middleware::cloudflare_only))
            .wrap(
                DefaultHeaders::new()
                    .add(("X-Content-Type-Options", "nosniff"))
                    .add(("X-Frame-Options", "DENY"))
                    .add(("Content-Security-Policy", CSP))
                    .add(("Permissions-Policy", "camera=(), microphone=(), geolocation=()"))
                    .add(("Referrer-Policy", "strict-origin-when-cross-origin"))
                    .add(("Strict-Transport-Security", "max-age=31536000; includeSubDomains")),
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
                    .configure(admin::api)
                    .configure(slack::routes)
                    .configure(surveys::routes)
                    .service((snapshots::create_snapshot, snapshots::held_image))
                    .service((feedback::create_feedback, feedback::visit_survey, feedback::dismiss_survey))
                    .service((accounts::session::logout, accounts::session::logout_all)),
            )
            .configure(accounts::routes)
            .service(slack::events)
            .service(web::scope("/dashboard/item-images").service(tree::item_image))
            .configure(dashboard::pages)
            .configure(stats::pages)
            .configure(admin::pages)
            .configure(surveys::pages)
            .configure(slack::pages)
            .service(docs)
            .service(web::resource(["/docs/", "/stats/"]).route(web::get().to(trailing_slash)))
            .service(
                Files::new("/", FRONTEND_DIR)
                    .index_file("index.html")
                    .path_filter(|path, _| path.extension().is_none_or(|extension| extension != "html"))
                    .default_handler(fn_service(not_found)),
            )
    })
    .bind(("0.0.0.0", 8080))?
    .run()
    .await
}
