use crate::{
    accounts::{
        profile,
        tokens::{self, Provider},
    },
    env,
    projects::review,
};
use anyhow::Context;
use chrono::{DateTime, Utc};
use hc_helper::{
    airtable::AirtableTable,
    auth::data::{forget_auth_token, get_auth_data_with_token},
    submission::create_unified_with_token,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{PgPool, types::Json};

#[derive(Serialize, Deserialize)]
pub struct Queued {
    pub demo: String,
    pub screenshot: String,
    pub description: String,
    pub hackatime: Vec<String>,
    pub hours: f64,
    pub hackatime_id: String,
    pub github_username: Option<String>,
}

pub async fn send(
    table: AirtableTable,
    hca_token: &str,
    hackatime_token: &str,
    code_url: String,
    tier: &str,
    shipped_at: DateTime<Utc>,
    queued: Queued,
) -> anyhow::Result<String> {
    create_unified_with_token(
        hca_token,
        hackatime_token,
        table,
        code_url,
        queued.demo,
        queued.screenshot,
        queued.description,
        queued.hackatime,
        review::Extras {
            hours: queued.hours,
            hackatime_id: queued.hackatime_id,
            github_username: queued.github_username,
            tier: review::tier_label(tier)?,
            shipped_at,
        },
    )
    .await
}

#[derive(sqlx::FromRow)]
struct Waiting {
    id: i32,
    user_id: i32,
    code_url: String,
    tier: Option<String>,
    created_at: DateTime<Utc>,
    queued: Json<Value>,
}

pub fn spawn(pool: PgPool) {
    let Some(table) = review::table() else {
        return;
    };
    actix_web::rt::spawn(async move {
        let mut interval = env::interval_secs("ID_VERIFICATION_CHECK_SECS", 60 * 60);
        loop {
            interval.tick().await;
            if let Err(e) = release(&pool, &table).await {
                log::error!("Submitting ships waiting on ID verification failed: {e:#}");
            }
        }
    });
}

async fn release(pool: &PgPool, table: &AirtableTable) -> anyhow::Result<()> {
    let stuck: Vec<String> = sqlx::query_scalar(
        "SELECT id::text FROM submission WHERE record_id IS NULL AND sending_at IS NOT NULL AND deleted_at IS NULL",
    )
    .fetch_all(pool)
    .await?;
    if !stuck.is_empty() {
        log::error!(
            "Ships {} were being sent for review but never saved a record id. If a ship's record is in the review table, set its submission.record_id, otherwise clear its submission.sending_at to resend it",
            stuck.join(", ")
        );
    }
    let waiting: Vec<Waiting> = sqlx::query_as(
        "SELECT s.id, p.user_id, s.code_url, s.tier, s.created_at, s.queued
         FROM submission s JOIN project p ON p.id = s.project_id JOIN \"user\" u ON u.id = p.user_id
         WHERE s.record_id IS NULL AND s.queued IS NOT NULL AND s.sending_at IS NULL
         AND s.deleted_at IS NULL AND NOT s.rejected
         AND p.deleted_at IS NULL AND u.deleted_at IS NULL AND NOT u.is_banned
         ORDER BY p.user_id, s.created_at, s.id",
    )
    .fetch_all(pool)
    .await?;
    let mut checked: Option<(i32, Option<String>)> = None;
    for ship in waiting {
        if checked.as_ref().is_none_or(|(user_id, _)| *user_id != ship.user_id) {
            let token = verified(pool, ship.user_id).await.unwrap_or_else(|e| {
                log::warn!("Checking user {}'s ID verification failed: {e:#}", ship.user_id);
                None
            });
            checked = Some((ship.user_id, token));
        }
        let Some((_, Some(hca_token))) = &checked else {
            continue;
        };
        if let Err(e) = submit(pool, table, hca_token, ship).await {
            log::error!("{e:#}");
        }
    }
    Ok(())
}

async fn verified(pool: &PgPool, user_id: i32) -> anyhow::Result<Option<String>> {
    let Some(token) = tokens::load(pool, user_id, Provider::Hca).await? else {
        return Ok(None);
    };
    forget_auth_token(&token);
    let data = get_auth_data_with_token(&token).await?;
    profile::sync(pool, &data).await?;
    Ok(profile::ship_blocked(Some(&data.verification_status), data.ysws_eligible)
        .is_none()
        .then_some(token))
}

async fn submit(pool: &PgPool, table: &AirtableTable, hca_token: &str, ship: Waiting) -> anyhow::Result<()> {
    let id = ship.id;
    let queued: Queued = serde_json::from_value(ship.queued.0)
        .with_context(|| format!("Ship {id} has unreadable queued data, skipping it"))?;
    let hackatime_token = tokens::load(pool, ship.user_id, Provider::Hackatime)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Ship {id} is verified but its user has no Hackatime token"))?;
    let claimed = sqlx::query(
        "UPDATE submission SET sending_at = now() WHERE id = $1 AND record_id IS NULL AND sending_at IS NULL",
    )
    .bind(id)
    .execute(pool)
    .await?
    .rows_affected();
    if claimed == 0 {
        return Ok(());
    }
    let sent = send(
        table.clone(),
        hca_token,
        &hackatime_token,
        ship.code_url,
        ship.tier.as_deref().unwrap_or_default(),
        ship.created_at,
        queued,
    )
    .await;
    let record_id = match sent {
        Ok(record_id) => record_id,
        Err(e) => {
            log::error!("Submitting ship {id} after ID verification failed: {e:#}");
            sqlx::query("UPDATE submission SET sending_at = NULL WHERE id = $1")
                .bind(id)
                .execute(pool)
                .await?;
            return Ok(());
        }
    };
    sqlx::query("UPDATE submission SET record_id = $2, queued = NULL WHERE id = $1")
        .bind(id)
        .bind(&record_id)
        .execute(pool)
        .await
        .map_err(|e| {
            anyhow::anyhow!("Review record {record_id} was created for ship {id} but saving it failed: {e}")
        })?;
    log::info!("Submitted ship {id} for review now that its ID verification came through");
    Ok(())
}
