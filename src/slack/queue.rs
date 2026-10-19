use super::{RECIPIENTS, SendError};
use actix_web::rt::time::sleep;
use sqlx::PgPool;
use std::time::Duration;

const SEND_EVERY: Duration = Duration::from_secs(1);
const IDLE: Duration = Duration::from_secs(2);
const UNAVAILABLE_WAIT: Duration = Duration::from_secs(5 * 60);
const RETRY_WAIT: Duration = Duration::from_secs(30);
const MAX_ATTEMPTS: i32 = 5;
const PREFERENCE: &str = "CASE m.kind
    WHEN 'announcement' THEN p.announcements
    WHEN 'challenge' THEN p.challenges
    WHEN 'round' THEN p.fulfillment_rounds
    WHEN 'snapshot' THEN p.snapshot_reminders
    WHEN 'ship' THEN p.ship_updates
    WHEN 'inactivity' THEN p.inactivity_reminders
    ELSE false END";

pub fn spawn(pool: PgPool) {
    actix_web::rt::spawn(async move {
        loop {
            let wait = send_next(&pool).await.unwrap_or_else(|e| {
                log::error!("Slack queue failed: {e:#}");
                RETRY_WAIT
            });
            sleep(wait).await;
        }
    });
}

async fn send_next(pool: &PgPool) -> anyhow::Result<Duration> {
    let mut tx = pool.begin().await?;
    let next: Option<(i32, String, String, i32, bool)> = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "SELECT m.id, m.slack_id, m.text, m.attempts,
             m.kind = 'admin' OR COALESCE(u.slack_id = m.slack_id AND {RECIPIENTS} AND {PREFERENCE}, false)
         FROM slack_message m
         LEFT JOIN \"user\" u ON u.id = m.user_id
         LEFT JOIN slack_preference p ON p.user_id = m.user_id
         WHERE NOT m.incoming AND m.sent_at IS NULL AND m.skipped_at IS NULL AND m.failed_at IS NULL
         ORDER BY m.announcement_id IS NOT NULL, m.id LIMIT 1
         FOR UPDATE OF m SKIP LOCKED"
    )))
    .fetch_optional(&mut *tx)
    .await?;
    let Some((id, slack_id, text, attempts, allowed)) = next else {
        return Ok(IDLE);
    };
    if !allowed {
        sqlx::query("UPDATE slack_message SET skipped_at = now() WHERE id = $1")
            .bind(id)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        return Ok(Duration::ZERO);
    }
    let wait = match super::post(&slack_id, &text).await {
        Ok(ts) => {
            sqlx::query("UPDATE slack_message SET sent_at = now(), ts = $2, attempts = attempts + 1 WHERE id = $1")
                .bind(id)
                .bind(ts)
                .execute(&mut *tx)
                .await?;
            SEND_EVERY
        }
        Err(SendError::RateLimited(secs)) => return Ok(Duration::from_secs(secs)),
        Err(SendError::Unavailable(error)) => {
            log::error!("Slack is unavailable, pausing DMs: {error}");
            return Ok(UNAVAILABLE_WAIT);
        }
        Err(SendError::Down(error)) => {
            log::warn!("Slack didn't respond to message {id}, trying again soon: {error:#}");
            return Ok(RETRY_WAIT);
        }
        Err(SendError::Rejected(error)) => {
            sqlx::query(
                "UPDATE slack_message SET failed_at = now(), error = $2, attempts = attempts + 1 WHERE id = $1",
            )
            .bind(id)
            .bind(error)
            .execute(&mut *tx)
            .await?;
            SEND_EVERY
        }
        Err(SendError::Failed(error)) => {
            log::warn!("Sending Slack message {id} failed: {error:#}");
            sqlx::query(
                "UPDATE slack_message SET attempts = attempts + 1, error = $2,
                     failed_at = CASE WHEN attempts + 1 >= $3 THEN now() END
                 WHERE id = $1",
            )
            .bind(id)
            .bind(format!("{error:#}"))
            .bind(MAX_ATTEMPTS)
            .execute(&mut *tx)
            .await?;
            if attempts + 1 >= MAX_ATTEMPTS { SEND_EVERY } else { RETRY_WAIT }
        }
    };
    tx.commit().await?;
    Ok(wait)
}
