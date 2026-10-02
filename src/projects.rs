use actix_web::{HttpRequest, HttpResponse, get, post, put, web};
use hc_helper::get_auth_token;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{collections::BTreeMap, sync::Mutex};

static PROJECTS: Mutex<BTreeMap<String, Vec<Project>>> = Mutex::new(BTreeMap::new());

#[derive(Serialize, Deserialize, Clone, Copy)]
#[serde(rename_all = "lowercase")]
enum Tier {
    Quebec,
    Romeo,
    Sierra,
}

#[derive(Serialize, Clone)]
struct Project {
    id: u32,
    name: String,
    description: String,
    repo: String,
    demo: String,
    tier: Option<Tier>,
    hackatime: Vec<String>,
    screenshot: Option<String>,
}

#[derive(Deserialize)]
struct ProjectFields {
    name: String,
    description: String,
    repo: String,
    demo: String,
    tier: Option<Tier>,
    hackatime: Vec<String>,
}

#[derive(Deserialize)]
struct CreateRequest {
    name: String,
}

fn list(token: &str) -> Vec<Project> {
    PROJECTS
        .lock()
        .unwrap()
        .get(token)
        .cloned()
        .unwrap_or_default()
}

fn create(token: &str, name: String) -> Project {
    let mut projects = PROJECTS.lock().unwrap();
    let list = projects.entry(token.to_string()).or_default();
    let project = Project {
        id: list.len() as u32 + 1,
        name,
        description: String::new(),
        repo: String::new(),
        demo: String::new(),
        tier: None,
        hackatime: Vec::new(),
        screenshot: None,
    };
    list.push(project.clone());
    project
}

fn update(token: &str, id: u32, fields: ProjectFields) -> Result<Project, &'static str> {
    let mut projects = PROJECTS.lock().unwrap();
    let project = projects
        .get_mut(token)
        .and_then(|list| list.iter_mut().find(|p| p.id == id))
        .ok_or("unknown project")?;
    project.name = fields.name;
    project.description = fields.description;
    project.repo = fields.repo;
    project.demo = fields.demo;
    project.tier = fields.tier;
    project.hackatime = fields.hackatime;
    Ok(project.clone())
}

#[get("/projects")]
pub async fn list_projects(req: HttpRequest) -> HttpResponse {
    let token = get_auth_token!(req);
    HttpResponse::Ok().json(list(&token))
}

#[post("/projects")]
pub async fn create_project(req: HttpRequest, body: web::Json<CreateRequest>) -> HttpResponse {
    let token = get_auth_token!(req);
    let name = body.name.trim();
    if name.is_empty() {
        return HttpResponse::BadRequest().json(json!({ "error": "name required" }));
    }
    HttpResponse::Ok().json(create(&token, name.to_string()))
}

#[post("/projects/{id}/screenshot")]
pub async fn upload_screenshot(
    req: HttpRequest,
    _id: web::Path<u32>,
    _body: web::Bytes,
) -> HttpResponse {
    let _token = get_auth_token!(req);
    todo!()
}

#[put("/projects/{id}")]
pub async fn update_project(
    req: HttpRequest,
    id: web::Path<u32>,
    body: web::Json<ProjectFields>,
) -> HttpResponse {
    let token = get_auth_token!(req);
    let mut fields = body.into_inner();
    fields.name = fields.name.trim().to_string();
    if fields.name.is_empty() {
        return HttpResponse::BadRequest().json(json!({ "error": "name required" }));
    }
    match update(&token, *id, fields) {
        Ok(project) => HttpResponse::Ok().json(project),
        Err(error) => HttpResponse::NotFound().json(json!({ "error": error })),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{App, cookie::Cookie, http::StatusCode, test};
    use serde_json::Value;

    #[actix_web::test]
    async fn projects_flow() {
        let app = test::init_service(
            App::new().service(
                web::scope("/api")
                    .service(list_projects)
                    .service(create_project)
                    .service(upload_screenshot)
                    .service(update_project),
            ),
        )
        .await;

        let cookie = || Cookie::new("auth-token", "projects-test-user");
        let create = |name: &str| {
            test::TestRequest::post()
                .uri("/api/projects")
                .cookie(cookie())
                .set_json(json!({ "name": name }))
                .to_request()
        };
        let put = |id: u32, name: &str| {
            test::TestRequest::put()
                .uri(&format!("/api/projects/{id}"))
                .cookie(cookie())
                .set_json(json!({
                    "name": name,
                    "description": "d",
                    "repo": "r",
                    "demo": "m",
                    "tier": "romeo",
                    "hackatime": ["mipmap"],
                }))
                .to_request()
        };

        let res = test::call_service(&app, create("  ")).await;
        assert_eq!(res.status(), StatusCode::BAD_REQUEST);
        let body: Value = test::read_body_json(res).await;
        assert_eq!(body, json!({ "error": "name required" }));

        let res = test::call_service(&app, create(" first ")).await;
        assert_eq!(res.status(), StatusCode::OK);
        let body: Value = test::read_body_json(res).await;
        assert_eq!(
            body,
            json!({
                "id": 1,
                "name": "first",
                "description": "",
                "repo": "",
                "demo": "",
                "tier": null,
                "hackatime": [],
                "screenshot": null,
            })
        );

        let req = test::TestRequest::get()
            .uri("/api/projects")
            .cookie(cookie())
            .to_request();
        let body: Value = test::call_and_read_body_json(&app, req).await;
        assert_eq!(body.as_array().unwrap().len(), 1);

        let res = test::call_service(&app, put(1, " renamed ")).await;
        assert_eq!(res.status(), StatusCode::OK);
        let body: Value = test::read_body_json(res).await;
        assert_eq!(
            body,
            json!({
                "id": 1,
                "name": "renamed",
                "description": "d",
                "repo": "r",
                "demo": "m",
                "tier": "romeo",
                "hackatime": ["mipmap"],
                "screenshot": null,
            })
        );

        let res = test::call_service(&app, put(1, " ")).await;
        assert_eq!(res.status(), StatusCode::BAD_REQUEST);

        let res = test::call_service(&app, put(99, "x")).await;
        assert_eq!(res.status(), StatusCode::NOT_FOUND);
        let body: Value = test::read_body_json(res).await;
        assert_eq!(body, json!({ "error": "unknown project" }));

        let req = test::TestRequest::get()
            .uri("/api/projects")
            .cookie(cookie())
            .to_request();
        let body: Value = test::call_and_read_body_json(&app, req).await;
        assert_eq!(body[0]["name"], "renamed");
    }
}
