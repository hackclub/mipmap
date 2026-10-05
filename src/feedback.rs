use crate::{
    accounts::session::User,
    error::{ApiError, reject},
};
use actix_web::{HttpResponse, post, web};
use serde::Deserialize;
use sqlx::PgPool;

const TEXT_LIMIT: usize = 2000;

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
    sqlx::query(
        "INSERT INTO feedback (user_id, score, source, positives, improvements)
         VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(user_id)
    .bind(body.score)
    .bind(clean(body.source)?)
    .bind(clean(body.positives)?)
    .bind(clean(body.improvements)?)
    .execute(pool.get_ref())
    .await?;
    Ok(HttpResponse::NoContent().finish())
}
