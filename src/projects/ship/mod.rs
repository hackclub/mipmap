use super::{review, unknown_project};
use crate::{
    accounts::session::User,
    dashboard,
    error::{ApiError, reject_with},
};
use actix_web::{HttpResponse, http::StatusCode, post, web, web::Bytes};
use futures_util::stream;
use hc_helper::airtable::AirtableTable;
use serde_json::{Value, json};
use sqlx::PgPool;
use std::{
    collections::HashSet,
    convert::Infallible,
    sync::{LazyLock, Mutex},
};
use tokio::sync::mpsc::{UnboundedSender, unbounded_channel};

mod checks;
mod repos;

#[derive(sqlx::FromRow)]
struct Draft {
    id: i32,
    name: String,
    description: String,
    repo: String,
    demo: String,
    screenshot: Option<String>,
    tier: Option<String>,
    hackatime: Vec<String>,
    reviewing: bool,
}

static SHIPPING: LazyLock<Mutex<HashSet<i32>>> = LazyLock::new(Default::default);

pub struct Shipping(i32);

impl Shipping {
    pub fn start(project_id: i32) -> Option<Self> {
        SHIPPING
            .lock()
            .unwrap()
            .insert(project_id)
            .then_some(Shipping(project_id))
    }
}

impl Drop for Shipping {
    fn drop(&mut self) {
        SHIPPING.lock().unwrap().remove(&self.0);
    }
}

struct Events(UnboundedSender<Bytes>);

impl Events {
    fn send(&self, line: Value) {
        let _ = self.0.send(Bytes::from(format!("{line}\n")));
    }

    async fn check<T>(
        &self,
        id: &str,
        label: &str,
        check: impl Future<Output = Result<T, ApiError>>,
    ) -> Option<T> {
        self.send(json!({ "check": id, "label": label, "status": "running" }));
        match check.await {
            Ok(value) => {
                self.send(json!({ "check": id, "status": "passed" }));
                Some(value)
            }
            Err(error) => {
                let message = match error {
                    ApiError::Rejected(_, message) => message,
                    ApiError::Internal(e) => {
                        log::error!("ship check {id} failed: {e:#}");
                        "Something went wrong, please try again.".into()
                    }
                };
                self.send(json!({ "check": id, "status": "failed", "message": message }));
                None
            }
        }
    }
}

#[post("/projects/{id}/ship")]
pub async fn ship_project(
    User(user_id): User,
    pool: web::Data<PgPool>,
    id: web::Path<i32>,
) -> Result<HttpResponse, ApiError> {
    let table = review::table()
        .ok_or_else(|| reject_with(StatusCode::SERVICE_UNAVAILABLE, "Shipping isn't open yet."))?;
    let shipping = Shipping::start(*id)
        .ok_or_else(|| reject_with(StatusCode::CONFLICT, "This project is already being shipped."))?;
    let project: Draft = sqlx::query_as(
        "SELECT id, name, description, COALESCE(repo_url, '') AS repo, COALESCE(demo_url, '') AS demo,
             screenshot_url AS screenshot, tier, hackatime_projects AS hackatime,
             COALESCE((SELECT NOT rejected AND review_status IS DISTINCT FROM $3 FROM submissions
                       WHERE project_id = projects.id AND deleted_at IS NULL
                       ORDER BY created_at DESC, id DESC LIMIT 1), false) AS reviewing
         FROM projects WHERE id = $1 AND user_id = $2 AND deleted_at IS NULL",
    )
    .bind(*id)
    .bind(user_id)
    .bind(review::APPROVED)
    .fetch_optional(pool.get_ref())
    .await?
    .ok_or_else(unknown_project)?;
    if project.reviewing {
        return Err(reject_with(StatusCode::CONFLICT, "This project is still being reviewed."));
    }

    let (tx, rx) = unbounded_channel();
    let pool = pool.get_ref().clone();
    actix_web::rt::spawn(async move {
        let _shipping = shipping;
        let events = Events(tx);
        let line = match run(&pool, user_id, &project, table, &events).await {
            Ok(()) => match dashboard::build(&pool, user_id).await {
                Ok(dashboard) => json!({ "dashboard": dashboard }),
                Err(e) => {
                    log::error!("building dashboard after ship failed: {e:#}");
                    json!({ "dashboard": null })
                }
            },
            Err(message) => json!({ "error": message }),
        };
        events.send(line);
    });
    Ok(HttpResponse::Ok()
        .content_type("application/x-ndjson")
        .streaming(stream::unfold(rx, |mut rx| async move {
            rx.recv().await.map(|line| (Ok::<_, Infallible>(line), rx))
        })))
}

async fn run(
    pool: &PgPool,
    user_id: i32,
    project: &Draft,
    table: AirtableTable,
    events: &Events,
) -> Result<(), &'static str> {
    let details = events
        .check("details", "Project details", async { checks::details(project) })
        .await;
    let account = events
        .check("account", "Hack Club account", checks::account(pool, user_id))
        .await;
    let hackatime = events
        .check("hackatime", "Hackatime", checks::hackatime(pool, user_id, project))
        .await;
    let repo = events
        .check("readme", "Repository has a README", checks::readme(&project.repo))
        .await;
    let unique = events
        .check(
            "duplicate",
            "Repository not shipped by another project",
            checks::not_shipped(pool, project.id, &project.repo),
        )
        .await;
    let demo = events
        .check("demo", "Demo link works", checks::demo(&project.demo))
        .await;

    let (Some(()), Some(hca_token), Some(hackatime), Some(repo), Some(()), Some(())) =
        (details, account, hackatime, repo, unique, demo)
    else {
        return Err("Fix the failed checks and try again.");
    };

    events
        .check(
            "submit",
            "Sending for review",
            checks::submit(pool, project, &repo, table, &hca_token, hackatime),
        )
        .await
        .ok_or("Submission failed. Try again once or twice, then bug me about it.")
}
