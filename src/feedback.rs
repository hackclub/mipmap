use crate::{
    accounts::session::User,
    dashboard,
    error::{ApiError, reject},
};
use actix_web::{HttpResponse, post, web};
use serde::Deserialize;
use serde_json::json;
use sqlx::{PgExecutor, PgPool};

const TEXT_LIMIT: usize = 2000;
const SURVEY_MIPS: i32 = 15;

pub async fn survey_due(db: impl PgExecutor<'_>, user_id: i32) -> sqlx::Result<bool> {
    sqlx::query_scalar(
        "SELECT u.onboarded_at IS NOT NULL AND u.dashboard_visits >= 3
             AND (u.survey_dismissed_at IS NULL OR u.survey_dismissed_at < now() - interval '5 days')
             AND NOT EXISTS (
                 SELECT 1 FROM feedback f
                 WHERE f.user_id = u.id AND f.deleted_at IS NULL AND f.created_at > now() - interval '5 days'
             )
             AND (SELECT COALESCE(sum(p.total_seconds), 0) FROM project p
                  WHERE p.user_id = u.id AND p.deleted_at IS NULL) >= 3600
         FROM \"user\" u WHERE u.id = $1",
    )
    .bind(user_id)
    .fetch_one(db)
    .await
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
    sqlx::query("UPDATE \"user\" SET mips = mips + $2, dashboard_visits = 0 WHERE id = $1")
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
    let show = survey_due(&mut *tx, user_id).await? && rand::random::<bool>();
    tx.commit().await?;
    Ok(HttpResponse::Ok().json(json!({ "show": show })))
}

#[post("/survey/dismiss")]
pub async fn dismiss_survey(User(user_id): User, pool: web::Data<PgPool>) -> Result<HttpResponse, ApiError> {
    sqlx::query("UPDATE \"user\" SET survey_dismissed_at = now() WHERE id = $1")
        .bind(user_id)
        .execute(pool.get_ref())
        .await?;
    Ok(HttpResponse::NoContent().finish())
}
