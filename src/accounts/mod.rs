use actix_web::{HttpResponse, web};
use std::{cell::Cell, rc::Rc};

pub mod hackatime;
mod login;
pub mod profile;
pub mod session;
pub mod tokens;

pub use login::login_redirect;

pub fn routes(cfg: &mut web::ServiceConfig) {
    cfg.service((login::start, login::callback, hackatime::start, hackatime::callback));
}

pub fn redirect(location: &str) -> HttpResponse {
    HttpResponse::Found()
        .append_header(("Location", location))
        .finish()
}

pub fn site_url(path: &str) -> String {
    let base_url = hc_helper::keys::base_url();
    let host = match base_url.rsplit_once(':') {
        Some((host, port)) if port.bytes().all(|byte| byte.is_ascii_digit()) => host,
        _ => &base_url,
    };
    let scheme = match host {
        "localhost" | "127.0.0.1" | "[::1]" => "http",
        _ => "https",
    };
    format!("{scheme}://{base_url}{path}")
}

fn token_sink() -> (Rc<Cell<Option<String>>>, impl AsyncFnOnce(String)) {
    let token = Rc::new(Cell::new(None));
    let sink = token.clone();
    (token, async move |value| sink.set(Some(value)))
}
