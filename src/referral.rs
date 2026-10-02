use actix_web::{HttpRequest, HttpResponse, get};
use hc_helper::get_auth_token;
use serde_json::json;
use std::hash::{DefaultHasher, Hash, Hasher};

fn code(token: &str) -> String {
    let mut hasher = DefaultHasher::new();
    token.hash(&mut hasher);
    format!("{:016x}", hasher.finish())[..12].to_string()
}

#[get("/referral")]
pub async fn get_referral(req: HttpRequest) -> HttpResponse {
    let token = get_auth_token!(req);
    HttpResponse::Ok().json(json!({ "code": code(&token) }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{App, cookie::Cookie, http::StatusCode, test, web};
    use serde_json::Value;

    #[actix_web::test]
    async fn code_is_deterministic() {
        let app =
            test::init_service(App::new().service(web::scope("/api").service(get_referral))).await;

        let mut codes = Vec::new();
        for _ in 0..2 {
            let req = test::TestRequest::get()
                .uri("/api/referral")
                .cookie(Cookie::new("auth-token", "referral-test-user"))
                .to_request();
            let res = test::call_service(&app, req).await;
            assert_eq!(res.status(), StatusCode::OK);
            let body: Value = test::read_body_json(res).await;
            codes.push(body["code"].as_str().unwrap().to_string());
        }

        assert_eq!(codes[0], codes[1]);
        assert_eq!(codes[0].len(), 12);
        assert!(
            codes[0]
                .chars()
                .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
        );
    }
}
