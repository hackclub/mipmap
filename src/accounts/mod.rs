use actix_web::{HttpRequest, HttpResponse, web};
use std::{cell::Cell, rc::Rc};

pub mod hackatime;
mod login;
pub mod profile;
pub mod session;
pub mod tokens;

pub const PATHS: [&str; 4] = ["/login", "/callback", "/hackatime/login", "/hackatime/callback"];

pub fn routes(cfg: &mut web::ServiceConfig) {
    cfg.service((login::start, login::callback, hackatime::start, hackatime::callback));
}

pub fn redirect(location: &str) -> HttpResponse {
    HttpResponse::Found()
        .append_header(("Location", location))
        .finish()
}

fn callback_url(req: &HttpRequest, path: &str) -> String {
    format!(
        "{}://{}{path}",
        req.connection_info().scheme(),
        hc_helper::keys::base_url()
    )
}

fn token_sink() -> (Rc<Cell<Option<String>>>, impl AsyncFnOnce(String)) {
    let token = Rc::new(Cell::new(None));
    let sink = token.clone();
    (token, async move |value| sink.set(Some(value)))
}
