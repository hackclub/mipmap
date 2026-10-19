use crate::{accounts::session::User, error::ApiError};
use actix_web::{HttpResponse, get, post, web};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

pub fn routes(cfg: &mut web::ServiceConfig) {
    cfg.service((get_preferences, set_preferences));
}

#[derive(Serialize, Deserialize, sqlx::FromRow)]
struct Preferences {
    enabled: bool,
    announcements: bool,
    challenges: bool,
    fulfillment_rounds: bool,
    snapshot_reminders: bool,
    ship_updates: bool,
    inactivity_reminders: bool,
}

#[derive(Serialize, sqlx::FromRow)]
struct Settings {
    slack_linked: bool,
    #[serde(flatten)]
    #[sqlx(flatten)]
    preferences: Preferences,
}

async fn fetch(pool: &PgPool, user_id: i32) -> sqlx::Result<Settings> {
    sqlx::query_as(
        "SELECT u.slack_id IS NOT NULL AS slack_linked, p.enabled, p.announcements, p.challenges,
             p.fulfillment_rounds, p.snapshot_reminders, p.ship_updates, p.inactivity_reminders
         FROM \"user\" u JOIN slack_preference p ON p.user_id = u.id WHERE u.id = $1",
    )
    .bind(user_id)
    .fetch_one(pool)
    .await
}

#[get("/slack/preferences")]
async fn get_preferences(User(user_id): User, pool: web::Data<PgPool>) -> Result<HttpResponse, ApiError> {
    Ok(HttpResponse::Ok().json(fetch(&pool, user_id).await?))
}

#[post("/slack/preferences")]
async fn set_preferences(
    User(user_id): User,
    pool: web::Data<PgPool>,
    body: web::Json<Preferences>,
) -> Result<HttpResponse, ApiError> {
    sqlx::query(
        "UPDATE slack_preference SET enabled = $2, announcements = $3, challenges = $4, fulfillment_rounds = $5,
             snapshot_reminders = $6, ship_updates = $7, inactivity_reminders = $8
         WHERE user_id = $1",
    )
    .bind(user_id)
    .bind(body.enabled)
    .bind(body.announcements)
    .bind(body.challenges)
    .bind(body.fulfillment_rounds)
    .bind(body.snapshot_reminders)
    .bind(body.ship_updates)
    .bind(body.inactivity_reminders)
    .execute(pool.get_ref())
    .await?;
    Ok(HttpResponse::Ok().json(fetch(&pool, user_id).await?))
}
