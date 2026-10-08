use crate::{
    accounts::session::User,
    error::{ApiError, reject, reject_with},
};
use actix_web::{HttpResponse, http::StatusCode, post, web};
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
    let mut tx = pool.begin().await?;
    sqlx::query("SELECT 1 FROM \"user\" WHERE id = $1 FOR UPDATE")
        .bind(user_id)
        .execute(&mut *tx)
        .await?;
    let inserted = sqlx::query(
        "INSERT INTO feedback (user_id, score, source, positives, improvements)
         SELECT $1, $2, $3, $4, $5
         WHERE NOT EXISTS (
             SELECT 1 FROM feedback WHERE user_id = $1 AND created_at > now() - interval '1 day'
         )",
    )
    .bind(user_id)
    .bind(body.score)
    .bind(clean(body.source)?)
    .bind(clean(body.positives)?)
    .bind(clean(body.improvements)?)
    .execute(&mut *tx)
    .await?;
    if inserted.rows_affected() == 0 {
        return Err(reject_with(
            StatusCode::TOO_MANY_REQUESTS,
            "You already sent feedback today. Thanks!",
        ));
    }
    tx.commit().await?;
    Ok(HttpResponse::NoContent().finish())
}
