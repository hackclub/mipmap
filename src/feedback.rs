use crate::{
    accounts::session::User,
    admin::{self, Admin},
    dashboard, env,
    error::{ApiError, reject},
};
use actix_web::{HttpResponse, get, post, rt::time::sleep, web};
use chrono::{DateTime, Utc};
use hc_helper::airtable::{AirtableTable, create_records};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::{PgExecutor, PgPool};
use std::{
    sync::{LazyLock, Mutex, PoisonError},
    time::Duration,
};

const TEXT_LIMIT: usize = 2000;
const SURVEY_MIPS: i32 = 10;
const PUSH_BACKOFF: Duration = Duration::from_secs(60);

async fn survey_state(db: impl PgExecutor<'_>, user_id: i32) -> sqlx::Result<(bool, bool)> {
    sqlx::query_as(
        "SELECT u.onboarded_at IS NOT NULL
             AND NOT EXISTS (
                 SELECT 1 FROM feedback f
                 WHERE f.user_id = u.id AND f.deleted_at IS NULL AND f.created_at > now() - interval '3 days'
             )
             AND (u.survey_forced OR (
                 u.dashboard_visits >= 3
                 AND (u.survey_dismissed_at IS NULL OR u.survey_dismissed_at < now() - interval '5 days')
                 AND (SELECT COALESCE(sum(p.total_seconds), 0) FROM project p
                      WHERE p.user_id = u.id AND p.deleted_at IS NULL) >= 3600
             )), u.survey_forced
         FROM \"user\" u WHERE u.id = $1",
    )
    .bind(user_id)
    .fetch_one(db)
    .await
}

pub async fn survey_due(db: impl PgExecutor<'_>, user_id: i32) -> sqlx::Result<bool> {
    survey_state(db, user_id).await.map(|(due, _)| due)
}

#[derive(Deserialize)]
struct FeedbackRequest {
    score: i16,
    source: Option<String>,
    positives: Option<String>,
    improvements: Option<String>,
}

fn clean(text: Option<String>) -> Result<Option<String>, ApiError> {
    let text = text.map(|text| text.trim().to_string()).filter(|text| !text.is_empty());
    if text.as_ref().is_some_and(|text| text.chars().count() > TEXT_LIMIT) {
        return Err(reject(format!("Answers can be at most {TEXT_LIMIT} characters")));
    }
    Ok(text)
}

#[post("/feedback")]
pub async fn create_feedback(
    User(user_id): User,
    pool: web::Data<PgPool>,
    body: web::Json<FeedbackRequest>,
) -> Result<HttpResponse, ApiError> {
    let body = body.into_inner();
    if !(1..=10).contains(&body.score) {
        return Err(reject("Choose a score from 1 to 10"));
    }
    let mut tx = pool.begin().await?;
    sqlx::query("SELECT 1 FROM \"user\" WHERE id = $1 FOR UPDATE")
        .bind(user_id)
        .execute(&mut *tx)
        .await?;
    if !survey_due(&mut *tx, user_id).await? {
        return Err(reject("Steve's survey isn't open right now"));
    }
    sqlx::query(
        "INSERT INTO feedback (user_id, score, source, positives, improvements)
         VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(user_id)
    .bind(body.score)
    .bind(clean(body.source)?)
    .bind(clean(body.positives)?)
    .bind(clean(body.improvements)?)
    .execute(&mut *tx)
    .await?;
    sqlx::query("UPDATE \"user\" SET mips = mips + $2, dashboard_visits = 0, survey_forced = false WHERE id = $1")
        .bind(user_id)
        .bind(SURVEY_MIPS)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    dashboard::respond(&pool, user_id).await
}

#[post("/survey/visit")]
pub async fn visit_survey(User(user_id): User, pool: web::Data<PgPool>) -> Result<HttpResponse, ApiError> {
    let mut tx = pool.begin().await?;
    sqlx::query("UPDATE \"user\" SET dashboard_visits = dashboard_visits + 1 WHERE id = $1")
        .bind(user_id)
        .execute(&mut *tx)
        .await?;
    let (due, forced) = survey_state(&mut *tx, user_id).await?;
    let show = due && (forced || rand::random::<bool>());
    tx.commit().await?;
    Ok(HttpResponse::Ok().json(json!({ "show": show })))
}

#[post("/survey/dismiss")]
pub async fn dismiss_survey(User(user_id): User, pool: web::Data<PgPool>) -> Result<HttpResponse, ApiError> {
    sqlx::query("UPDATE \"user\" SET survey_dismissed_at = now(), survey_forced = false WHERE id = $1")
        .bind(user_id)
        .execute(pool.get_ref())
        .await?;
    Ok(HttpResponse::NoContent().finish())
}

pub fn admin_routes(cfg: &mut web::ServiceConfig) {
    cfg.service((force_survey, get_push, start_push, stop_push));
}

#[post("/survey/force")]
async fn force_survey(Admin(admin_id): Admin, pool: web::Data<PgPool>) -> Result<HttpResponse, ApiError> {
    let mut tx = pool.begin().await?;
    let users = sqlx::query("UPDATE \"user\" SET survey_forced = true WHERE onboarded_at IS NOT NULL")
        .execute(&mut *tx)
        .await?
        .rows_affected();
    admin::record_read(&mut *tx, admin_id, "force_survey", format!("Showed Steve's survey to {users} users")).await?;
    tx.commit().await?;
    Ok(HttpResponse::Ok().json(json!({ "users": users })))
}

#[derive(Clone, Default, Serialize)]
struct Push {
    configured: bool,
    running: bool,
    stopping: bool,
    sent: i64,
    total: i64,
    pending: i64,
    retries: u32,
    error: Option<String>,
}

static PUSH: LazyLock<Mutex<Push>> = LazyLock::new(Default::default);

fn push() -> std::sync::MutexGuard<'static, Push> {
    PUSH.lock().unwrap_or_else(PoisonError::into_inner)
}

fn unified() -> Option<(AirtableTable, String)> {
    let table = AirtableTable {
        token: env::var("AIRTABLE_TOKEN")?,
        base_id: env::var("AIRTABLE_UNIFIED_BASE_ID")?,
        table_id_or_name: env::var("AIRTABLE_UNIFIED_NPS_TABLE").unwrap_or_else(|| "NPS".to_string()),
    };
    Some((table, env::var("AIRTABLE_UNIFIED_YSWS_RECORD_ID")?))
}

async fn push_status(pool: &PgPool) -> Result<Push, ApiError> {
    let pending = sqlx::query_scalar(
        "SELECT count(*) FROM feedback WHERE unified_record_id IS NULL AND deleted_at IS NULL",
    )
    .fetch_one(pool)
    .await?;
    let mut status = push().clone();
    status.configured = unified().is_some();
    status.pending = pending;
    Ok(status)
}

#[get("/feedback/push")]
async fn get_push(_: Admin, pool: web::Data<PgPool>) -> Result<HttpResponse, ApiError> {
    Ok(HttpResponse::Ok().json(push_status(&pool).await?))
}

#[post("/feedback/push")]
async fn start_push(Admin(admin_id): Admin, pool: web::Data<PgPool>) -> Result<HttpResponse, ApiError> {
    let Some((table, ysws)) = unified() else {
        return Err(reject("Unified isn't configured"));
    };
    let status = push_status(&pool).await?;
    if !status.running {
        *push() = Push { running: true, total: status.pending, ..Default::default() };
        admin::record_read(
            pool.get_ref(),
            admin_id,
            "push_feedback",
            format!("Started pushing {} feedback responses to Unified", status.pending),
        )
        .await?;
        let pool = pool.get_ref().clone();
        actix_web::rt::spawn(async move {
            let result = run_push(&pool, table, &ysws).await;
            let mut status = push();
            status.running = false;
            status.stopping = false;
            if let Err(e) = result {
                log::error!("Pushing feedback to Unified failed: {e:#}");
                status.error = Some(format!("{e:#}"));
            }
        });
    }
    Ok(HttpResponse::Ok().json(push_status(&pool).await?))
}

#[post("/feedback/push/stop")]
async fn stop_push(_: Admin, pool: web::Data<PgPool>) -> Result<HttpResponse, ApiError> {
    {
        let mut status = push();
        if status.running {
            status.stopping = true;
        }
    }
    Ok(HttpResponse::Ok().json(push_status(&pool).await?))
}

#[derive(sqlx::FromRow)]
struct Unsent {
    id: i32,
    score: i16,
    positives: Option<String>,
    improvements: Option<String>,
    created_at: DateTime<Utc>,
}

#[derive(Serialize)]
struct NpsFields<'a> {
    #[serde(rename = "On a scale from 1-10, how likely are you to recommend this YSWS to a friend?")]
    score: i16,
    #[serde(rename = "YSWS")]
    ysws: [&'a str; 1],
    #[serde(rename = "What are we doing well?", skip_serializing_if = "Option::is_none")]
    positives: Option<&'a str>,
    #[serde(rename = "How can we improve?", skip_serializing_if = "Option::is_none")]
    improvements: Option<&'a str>,
    #[serde(rename = "Override Created At")]
    created_at: DateTime<Utc>,
    #[serde(rename = "Tags (Comma Separated)")]
    tags: &'static str,
}

async fn run_push(pool: &PgPool, table: AirtableTable, ysws: &str) -> anyhow::Result<()> {
    loop {
        if push().stopping {
            return Ok(());
        }
        let row: Option<Unsent> = sqlx::query_as(
            "SELECT id, score, positives, improvements, created_at FROM feedback
             WHERE unified_record_id IS NULL AND deleted_at IS NULL ORDER BY id LIMIT 1",
        )
        .fetch_optional(pool)
        .await?;
        let Some(row) = row else {
            return Ok(());
        };
        let fields = NpsFields {
            score: row.score,
            ysws: [ysws],
            positives: row.positives.as_deref(),
            improvements: row.improvements.as_deref(),
            created_at: row.created_at,
            tags: "mipmap",
        };
        let record_id = loop {
            match create_records(table.clone(), vec![&fields]).await {
                Ok(mut ids) => break ids.pop().ok_or_else(|| anyhow::anyhow!("Airtable returned no record"))?,
                Err(e) => {
                    log::warn!("Pushing feedback #{} to Unified failed, retrying in a minute: {e:#}", row.id);
                    let mut status = push();
                    status.retries += 1;
                    status.error = Some(format!("{e:#}"));
                }
            }
            sleep(PUSH_BACKOFF).await;
            if push().stopping {
                return Ok(());
            }
        };
        sqlx::query("UPDATE feedback SET unified_record_id = $2 WHERE id = $1")
            .bind(row.id)
            .bind(record_id)
            .execute(pool)
            .await?;
        let mut status = push();
        status.sent += 1;
        status.error = None;
    }
}
