use actix_web::{HttpRequest, HttpResponse, get};
use hc_helper::get_auth_token;
use serde::Serialize;

#[derive(Serialize)]
struct Profile {
    first_name: String,
    fragments: u32,
}

#[get("/profile")]
pub async fn get_profile(req: HttpRequest) -> HttpResponse {
    let _token = get_auth_token!(req);
    HttpResponse::Ok().json(Profile {
        first_name: "Orpheus".to_string(),
        fragments: 1250,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{App, cookie::Cookie, http::StatusCode, test, web};
    use serde_json::{Value, json};

    #[actix_web::test]
    async fn returns_profile() {
        let app =
            test::init_service(App::new().service(web::scope("/api").service(get_profile))).await;

        let req = test::TestRequest::get()
            .uri("/api/profile")
            .cookie(Cookie::new("auth-token", "profile-test-user"))
            .to_request();
        let res = test::call_service(&app, req).await;
        assert_eq!(res.status(), StatusCode::OK);
        let body: Value = test::read_body_json(res).await;
        assert_eq!(body, json!({ "first_name": "Orpheus", "fragments": 1250 }));
    }
}
