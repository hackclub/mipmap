use actix_web::{HttpRequest, HttpResponse, get, web::Query};
use hc_helper::auth::login::{self, CallbackArgs, Scopes};

fn callback_redirect_url(req: &HttpRequest) -> String {
    format!(
        "{}://{}/callback",
        req.connection_info().scheme(),
        hc_helper::keys::base_url()
    )
}

fn scopes() -> Scopes {
    Scopes {
        openid: false,
        profile: false,
        email: false,
        name: false,
        slack_id: true,
        verification_status: true,
        basic_info: true,
        addresses: true,
    }
}

#[get("/login")]
pub async fn start(req: HttpRequest) -> HttpResponse {
    login::handle_login(&req, scopes(), callback_redirect_url(&req)).await
}

#[get("/callback")]
pub async fn callback(req: HttpRequest, query: Query<CallbackArgs>) -> HttpResponse {
    login::handle_callback::<fn(String) -> std::future::Ready<()>>(
        &req,
        query,
        callback_redirect_url(&req),
        "/".to_string(),
        None,
    )
    .await
}
