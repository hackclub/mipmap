use super::{ensure_owned, review, unknown_project};
use crate::{
    accounts::session::User,
    dashboard,
    error::{ApiError, reject_with},
};
use actix_web::{HttpResponse, http::StatusCode, post, web, web::Bytes};
use chrono::{DateTime, SubsecRound, Utc};
use futures_util::stream;
use hc_helper::airtable::AirtableTable;
use serde_json::{Value, json};
use sqlx::PgPool;
use std::{
    collections::HashSet,
    convert::Infallible,
    sync::{LazyLock, Mutex, PoisonError},
};
use tokio::sync::mpsc::{UnboundedSender, unbounded_channel};

mod checks;
pub mod history;
mod queue;
mod repos;

pub use queue::spawn;

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

const FAILED: &str = "Fix the failed checks and try again";

static SHIPPING: LazyLock<Mutex<HashSet<i32>>> = LazyLock::new(Default::default);

pub struct Shipping(i32);

impl Shipping {
    pub fn start(project_id: i32) -> Option<Self> {
        SHIPPING
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .insert(project_id)
            .then(|| Shipping(project_id))
    }
}

impl Drop for Shipping {
    fn drop(&mut self) {
        SHIPPING
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .remove(&self.0);
    }
}

struct Events(UnboundedSender<Bytes>);

impl Events {
    fn send(&self, line: Value) {
        let _ = self.0.send(Bytes::from(format!("{line}\n")));
    }

    fn failed(&self, id: &str, error: ApiError) {
        let message = match error {
            ApiError::Rejected(_, message) => message,
            ApiError::Internal(e) => {
                log::error!("ship check {id} failed: {e:#}");
                "Something went wrong, please try again".into()
            }
        };
        self.send(json!({ "check": id, "status": "failed", "message": message }));
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
                self.failed(id, error);
                None
            }
        }
    }

    async fn advise<T>(
        &self,
        id: &str,
        label: &str,
        unverified: &str,
        check: impl Future<Output = Result<(T, bool), ApiError>>,
    ) -> Option<T> {
        self.send(json!({ "check": id, "label": label, "status": "running" }));
        match check.await {
            Ok((value, true)) => {
                self.send(json!({ "check": id, "status": "passed" }));
                Some(value)
            }
            Ok((value, false)) => {
                self.send(json!({ "check": id, "status": "unverified", "message": unverified }));
                Some(value)
            }
            Err(error) => {
                self.failed(id, error);
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
    let shipped_at = Utc::now().trunc_subsecs(3);
    let table = review::table()
        .ok_or_else(|| reject_with(StatusCode::SERVICE_UNAVAILABLE, "Shipping isn't open yet"))?;
    ensure_owned(&pool, user_id, *id).await?;
    let shipping = Shipping::start(*id)
        .ok_or_else(|| reject_with(StatusCode::CONFLICT, super::BUSY))?;
    let project: Draft = sqlx::query_as(
        "SELECT id, name, description, COALESCE(repo_url, '') AS repo, COALESCE(demo_url, '') AS demo,
             screenshot_url AS screenshot, tier, hackatime_projects AS hackatime,
             COALESCE((SELECT NOT rejected AND review_status IS DISTINCT FROM $3 FROM submission
                       WHERE project_id = project.id AND deleted_at IS NULL
                       ORDER BY created_at DESC, id DESC LIMIT 1), false) AS reviewing
         FROM project WHERE id = $1 AND user_id = $2 AND deleted_at IS NULL",
    )
    .bind(*id)
    .bind(user_id)
    .bind(review::APPROVED)
    .fetch_optional(pool.get_ref())
    .await?
    .ok_or_else(unknown_project)?;
    if project.reviewing {
        return Err(reject_with(StatusCode::CONFLICT, "This project is still being reviewed"));
    }

    let (tx, rx) = unbounded_channel();
    let pool = pool.get_ref().clone();
    actix_web::rt::spawn(async move {
        let _shipping = shipping;
        let events = Events(tx);
        let line = match run(&pool, user_id, &project, table, shipped_at, &events).await {
            Ok(submitted) => match dashboard::build(&pool, user_id, false).await {
                Ok(dashboard) => json!({ "dashboard": dashboard, "waiting": !submitted }),
                Err(e) => {
                    log::error!("building dashboard after ship failed: {e:#}");
                    json!({ "dashboard": null, "waiting": !submitted })
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
    shipped_at: DateTime<Utc>,
    events: &Events,
) -> Result<bool, &'static str> {
    let details = events
        .check("details", "Project details", async { checks::details(project) })
        .await;
    let account = events
        .check(
            "account",
            "Hack Club Auth account has address, birthday, ID verified, etc.",
            checks::account(pool, user_id),
        )
        .await;
    let hackatime = events
        .check("hackatime", "Hackatime", checks::hackatime(pool, &table, user_id, project))
        .await;
    if account.is_none() || hackatime.is_none() {
        return Err(FAILED);
    }
    let repo = events
        .advise(
            "readme",
            "Repository has a README",
            "Couldn't verify a README. You can still ship, but ensure your README is detailed",
            checks::readme(&project.repo),
        )
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

    let (Some(()), Some(account), Some(hackatime), Some(repo), Some(_claim), Some(())) =
        (details, account, hackatime, repo, unique, demo)
    else {
        return Err(FAILED);
    };

    events
        .check(
            "submit",
            "Sending for review",
            checks::submit(pool, project, &repo, table, account, hackatime, shipped_at),
        )
        .await
        .ok_or("Submission failed. Try again once or twice, then bug me about it")
}
