use crate::tree::{Kind, TREE};
use actix_web::{HttpRequest, HttpResponse, get, post, web};
use hc_helper::get_auth_token;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Mutex,
};

#[derive(Clone, Default, Serialize)]
struct Progress {
    unlocked: BTreeSet<u32>,
    purchased: BTreeSet<u32>,
}

static PROGRESS: Mutex<BTreeMap<String, Progress>> = Mutex::new(BTreeMap::new());

fn progress(token: &str) -> Progress {
    PROGRESS
        .lock()
        .unwrap()
        .get(token)
        .cloned()
        .unwrap_or_default()
}

fn unlock(token: &str, id: u32) -> Result<Progress, &'static str> {
    let node = TREE
        .node(id)
        .filter(|node| node.kind != Kind::Origin)
        .ok_or("unknown node")?;
    let mut progress = PROGRESS.lock().unwrap();
    let state = progress.entry(token.to_string()).or_default();
    if state.unlocked.contains(&id) {
        return Err("already unlocked");
    }
    let reachable = node.requires.iter().any(|req| {
        state.unlocked.contains(req) || TREE.node(*req).is_some_and(|n| n.kind == Kind::Origin)
    });
    if !reachable {
        return Err("requirements not met");
    }
    state.unlocked.insert(id);
    Ok(state.clone())
}

fn purchase(token: &str, id: u32) -> Result<Progress, &'static str> {
    TREE.node(id)
        .filter(|node| node.kind == Kind::Item)
        .ok_or("unknown item")?;
    let mut progress = PROGRESS.lock().unwrap();
    let state = progress.entry(token.to_string()).or_default();
    if !state.unlocked.contains(&id) {
        return Err("item not unlocked");
    }
    if state.purchased.contains(&id) {
        return Err("item already purchased");
    }
    let tokens = state
        .unlocked
        .iter()
        .filter(|id| TREE.node(**id).is_some_and(|node| node.kind == Kind::Token))
        .count();
    if state.purchased.len() >= tokens {
        return Err("no tokens available");
    }
    state.purchased.insert(id);
    Ok(state.clone())
}

#[derive(Deserialize)]
struct UnlockRequest {
    id: u32,
}

#[get("/progress")]
pub async fn get_progress(req: HttpRequest) -> HttpResponse {
    let token = get_auth_token!(req);
    HttpResponse::Ok().json(progress(&token))
}

#[post("/unlock-item")]
pub async fn unlock_item(req: HttpRequest, body: web::Json<UnlockRequest>) -> HttpResponse {
    let token = get_auth_token!(req);
    match unlock(&token, body.id) {
        Ok(state) => HttpResponse::Ok().json(state),
        Err(error) => HttpResponse::BadRequest().json(json!({ "error": error })),
    }
}

#[post("/purchase-item")]
pub async fn purchase_item(req: HttpRequest, body: web::Json<UnlockRequest>) -> HttpResponse {
    let token = get_auth_token!(req);
    match purchase(&token, body.id) {
        Ok(state) => HttpResponse::Ok().json(state),
        Err(error) => HttpResponse::BadRequest().json(json!({ "error": error })),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{App, cookie::Cookie, http::StatusCode, test};
    use serde_json::Value;

    #[actix_web::test]
    async fn unlock_flow() {
        let app = test::init_service(
            App::new().service(
                web::scope("/api")
                    .service(get_progress)
                    .service(unlock_item),
            ),
        )
        .await;

        let post = |id: u32| {
            test::TestRequest::post()
                .uri("/api/unlock-item")
                .cookie(Cookie::new("auth-token", "test-user"))
                .set_json(json!({ "id": id }))
                .to_request()
        };

        let res = test::call_service(&app, post(16)).await;
        assert_eq!(res.status(), StatusCode::BAD_REQUEST);

        let res = test::call_service(&app, post(5)).await;
        assert_eq!(res.status(), StatusCode::OK);
        let body: Value = test::read_body_json(res).await;
        assert_eq!(body, json!({ "unlocked": [5], "purchased": [] }));

        let res = test::call_service(&app, post(5)).await;
        assert_eq!(res.status(), StatusCode::BAD_REQUEST);
        let body: Value = test::read_body_json(res).await;
        assert_eq!(body, json!({ "error": "already unlocked" }));

        let res = test::call_service(&app, post(16)).await;
        assert_eq!(res.status(), StatusCode::OK);

        let res = test::call_service(&app, post(52)).await;
        assert_eq!(res.status(), StatusCode::BAD_REQUEST);

        let req = test::TestRequest::get()
            .uri("/api/progress")
            .cookie(Cookie::new("auth-token", "test-user"))
            .to_request();
        let body: Value = test::call_and_read_body_json(&app, req).await;
        assert_eq!(body, json!({ "unlocked": [5, 16], "purchased": [] }));
    }
}
