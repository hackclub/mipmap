use actix_files::NamedFile;
use actix_web::{HttpRequest, HttpResponse, get};
use hc_helper::get_auth_token;

#[get("/dashboard")]
pub async fn page(req: HttpRequest) -> HttpResponse {
    get_auth_token!(req);
    match NamedFile::open_async(format!("{}/dashboard.html", crate::FRONTEND_DIR)).await {
        Ok(file) => file.into_response(&req),
        Err(_) => HttpResponse::NotFound().finish(),
    }
}

#[get("/dashboard/tree.json")]
pub async fn tree() -> HttpResponse {
    HttpResponse::Ok()
        .content_type("application/json")
        .body(crate::tree::TREE_JSON)
}

#[get("/dashboard/layout.json")]
pub async fn layout() -> HttpResponse {
    HttpResponse::Ok()
        .content_type("application/json")
        .body(crate::layout::LAYOUT_JSON.as_str())
}

#[get("/dashboard/fulfillment-rounds.json")]
pub async fn fulfillment_rounds() -> HttpResponse {
    HttpResponse::Ok()
        .content_type("application/json")
        .body(include_str!(
            "../frontend/src/lib/assets/fulfillment_rounds.json"
        ))
}

#[get("/dashboard/hackatime-projects.json")]
pub async fn hackatime_projects() -> HttpResponse {
    HttpResponse::Ok().json(serde_json::json!([
        { "name": "mipmap", "seconds": 287_400 },
        { "name": "dotfiles", "seconds": 41_700 },
        { "name": "raytracer", "seconds": 126_900 },
        { "name": "blog", "seconds": 18_300 },
        { "name": "discord-bot", "seconds": 9_600 },
        { "name": "chess-engine", "seconds": 213_000 },
        { "name": "pong", "seconds": 1_380 },
        { "name": "portfolio", "seconds": 5_220 }
    ]))
}
