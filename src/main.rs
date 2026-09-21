use actix_files::{Files, NamedFile};
use actix_web::{App, HttpServer, Responder, get, web};
use serde::Serialize;

mod login;

const FRONTEND_DIR: &str = "frontend/build";

#[derive(Serialize)]
struct Hello {
    message: String,
}

#[get("/hello")]
async fn hello() -> impl Responder {
    web::Json(Hello {
        message: "hello from actix".to_string(),
    })
}

async fn spa_fallback() -> std::io::Result<NamedFile> {
    NamedFile::open(format!("{FRONTEND_DIR}/index.html"))
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    log::info!("listening on http://127.0.0.1:8080");

    HttpServer::new(|| {
        App::new()
            .service(web::scope("/api").service(hello))
            .service(login::start)
            .service(login::callback)
            .service(Files::new("/", FRONTEND_DIR).index_file("index.html"))
            .default_service(web::to(spa_fallback))
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}
