use crate::{env, snapshots};
use hc_helper::{
    airtable::{AirtableTable, field_names, get_records},
    submission::UNIFIED_FIELDS,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{PgConnection, PgPool};
use std::collections::BTreeSet;

pub const APPROVED: &str = "2–Submitted";
pub const REFERRER_MIPS: i32 = 100;
pub const REFEREE_MIPS: i32 = 20;

#[derive(Serialize)]
pub struct Extras {
    #[serde(rename = "Optional - Override Hours Spent")]
    pub hours: f64,
    #[serde(rename = "Justification - Submitter Hackatime ID")]
    pub hackatime_id: String,
    #[serde(rename = "GitHub Username", skip_serializing_if = "Option::is_none")]
    pub github_username: Option<String>,
}

#[derive(Deserialize)]
struct Review {
    #[serde(rename = "Automation - Status")]
    status: Option<Value>,
    #[serde(rename = "Optional - Override Hours Spent")]
    hours: Option<f64>,
    #[serde(rename = "Code URL")]
    code_url: Option<String>,
    #[serde(rename = "Rejected", default)]
    rejected: bool,
}

#[derive(sqlx::FromRow)]
struct Submission {
    id: i32,
    record_id: String,
    mips_per_hour: f64,
    review_status: Option<String>,
    rejected: bool,
    hours: Option<f64>,
    multiplier: Option<f64>,
    mips: i32,
    deleted: bool,
}

const EXTRA_FIELDS: &[&str] = &[
    "Optional - Override Hours Spent",
    "Justification - Submitter Hackatime ID",
    "GitHub Username",
    "Automation - Status",
    "Rejected",
];

pub fn table() -> Option<AirtableTable> {
    Some(AirtableTable {
        token: env::var("AIRTABLE_TOKEN")?,
        base_id: env::var("AIRTABLE_REVIEW_BASE_ID")?,
        table_id_or_name: env::var("AIRTABLE_REVIEW_TABLE")?,
    })
}

pub fn mips_per_hour(tier: Option<&str>) -> anyhow::Result<f64> {
    let key = match tier {
        Some("quebec") => "MIPS_PER_HOUR_QUEBEC",
        Some("romeo") => "MIPS_PER_HOUR_ROMEO",
        Some("sierra") => "MIPS_PER_HOUR_SIERRA",
        _ => anyhow::bail!("project has no tier"),
    };
    env::var(key)
        .and_then(|rate| rate.parse().ok())
        .filter(|rate: &f64| rate.is_finite() && *rate >= 0.0)
        .ok_or_else(|| anyhow::anyhow!("{key} is not set to a valid number"))
}

pub fn spawn(pool: PgPool) {
    let Some(table) = table() else {
        log::warn!(
            "AIRTABLE_TOKEN, AIRTABLE_REVIEW_BASE_ID or AIRTABLE_REVIEW_TABLE not set, shipping disabled"
        );
        return;
    };
    actix_web::rt::spawn(async move {
        match field_names(table.clone()).await {
            Ok(fields) => {
                let missing: Vec<&str> = UNIFIED_FIELDS
                    .iter()
                    .chain(EXTRA_FIELDS)
                    .copied()
                    .filter(|field| !fields.iter().any(|name| name == field))
                    .collect();
                if !missing.is_empty() {
                    log::warn!(
                        "Review Airtable table is missing fields: {}",
                        missing.join(", ")
                    );
                }
            }
            Err(e) => log::warn!(
                "Couldn't check the review Airtable table's fields (the token needs schema.bases:read): {e:#}"
            ),
        }
        let mut interval = env::interval_secs("REVIEW_SYNC_INTERVAL_SECS", 120);
        loop {
            interval.tick().await;
            if let Err(e) = sync(&pool, &table).await {
                log::error!("Review sync failed: {e:#}");
            }
        }
    });
}

async fn sync(pool: &PgPool, table: &AirtableTable) -> anyhow::Result<()> {
    let ids: Vec<String> = sqlx::query_scalar("SELECT record_id FROM submission")
        .fetch_all(pool)
        .await?;
    if ids.is_empty() {
        return Ok(());
    }
    let reviews = get_records::<Review>(table.clone(), &ids).await?;
    apply(pool, &ids, reviews).await
}

async fn apply(pool: &PgPool, ids: &[String], reviews: Vec<(String, Review)>) -> anyhow::Result<()> {
    if reviews.is_empty() {
        log::warn!(
            "None of the {} submissions are in the review table, skipping review sync",
            ids.len()
        );
        return Ok(());
    }
    let mut found = Vec::new();
    let mut statuses = Vec::new();
    let mut hours = Vec::new();
    let mut code_urls = Vec::new();
    let mut rejected = Vec::new();
    for (record_id, review) in reviews {
        found.push(record_id);
        statuses.push(review.status.as_ref().and_then(Value::as_str).map(str::to_string));
        hours.push(review.hours);
        code_urls.push(review.code_url.unwrap_or_default());
        rejected.push(review.rejected);
    }

    let mut tx = pool.begin().await?;
    let mut changed: BTreeSet<i32> = sqlx::query_scalar(
        "UPDATE submission s
         SET review_status = r.status, hours = r.hours, code_url = r.code_url, rejected = r.rejected,
             approved_at = CASE WHEN r.status = $6 AND NOT r.rejected THEN COALESCE(s.approved_at, now()) END,
             deleted_at = NULL
         FROM UNNEST($1::text[], $2::text[], $3::float8[], $4::text[], $5::bool[])
             AS r (record_id, status, hours, code_url, rejected)
         WHERE s.record_id = r.record_id
         AND (s.review_status, s.hours, s.code_url, s.rejected, s.deleted_at)
             IS DISTINCT FROM (r.status, r.hours, r.code_url, r.rejected, NULL::timestamptz)
         RETURNING s.project_id",
    )
    .bind(&found)
    .bind(&statuses)
    .bind(&hours)
    .bind(&code_urls)
    .bind(&rejected)
    .bind(APPROVED)
    .fetch_all(&mut *tx)
    .await?
    .into_iter()
    .collect();
    changed.extend(
        sqlx::query_scalar::<_, i32>(
            "UPDATE submission SET deleted_at = now()
             WHERE record_id = ANY($1) AND NOT record_id = ANY($2) AND deleted_at IS NULL
             RETURNING project_id",
        )
        .bind(ids)
        .bind(&found)
        .fetch_all(&mut *tx)
        .await?,
    );
    for project_id in changed {
        reconcile(&mut tx, project_id).await?;
    }
    tx.commit().await?;
    Ok(())
}

async fn reconcile(conn: &mut PgConnection, project_id: i32) -> anyhow::Result<()> {
    let user_id: i32 = sqlx::query_scalar("SELECT user_id FROM project WHERE id = $1")
        .bind(project_id)
        .fetch_one(&mut *conn)
        .await?;
    let submissions: Vec<Submission> = sqlx::query_as(
        "SELECT id, record_id, mips_per_hour, review_status, rejected, hours, multiplier, mips,
             deleted_at IS NOT NULL AS deleted
         FROM submission WHERE project_id = $1 ORDER BY created_at, id FOR UPDATE",
    )
    .bind(project_id)
    .fetch_all(&mut *conn)
    .await?;
    let shared: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM snapshot WHERE user_id = $1 AND deleted_at IS NULL",
    )
    .bind(user_id)
    .fetch_one(&mut *conn)
    .await?;

    let mut paid_hours = 0.0_f64;
    let mut delta = 0;
    for submission in submissions {
        let approved = !submission.deleted
            && !submission.rejected
            && submission.review_status.as_deref() == Some(APPROVED);
        let (mips, multiplier) = match (approved, submission.hours) {
            (true, Some(hours)) => {
                let multiplier = submission
                    .multiplier
                    .unwrap_or_else(|| snapshots::multiplier(shared));
                let mips = ((hours - paid_hours).max(0.0)
                    * submission.mips_per_hour
                    * multiplier)
                    .round() as i32;
                paid_hours = paid_hours.max(hours);
                (mips, Some(multiplier))
            }
            (true, None) => {
                log::warn!(
                    "Review {} was approved without hours, so it earns nothing until hours are set",
                    submission.record_id
                );
                (0, submission.multiplier)
            }
            (false, _) => (0, submission.multiplier),
        };
        if mips != submission.mips || multiplier != submission.multiplier {
            sqlx::query("UPDATE submission SET mips = $2, multiplier = $3 WHERE id = $1")
                .bind(submission.id)
                .bind(mips)
                .bind(multiplier)
                .execute(&mut *conn)
                .await?;
            delta += mips - submission.mips;
        }
    }
    if delta != 0 {
        sqlx::query("UPDATE \"user\" SET mips = mips + $2 WHERE id = $1")
            .bind(user_id)
            .bind(delta)
            .execute(&mut *conn)
            .await?;
        log::info!("Project {project_id}'s Mips changed by {delta} to match the review table");
    }
    let referrer: Option<i32> = sqlx::query_scalar(
        "UPDATE \"user\" SET referral_paid = true
         WHERE id = $1 AND referred_by IS NOT NULL AND NOT referral_paid AND EXISTS (
             SELECT 1 FROM submission JOIN project ON project.id = submission.project_id
             WHERE project.user_id = $1 AND submission.deleted_at IS NULL
             AND NOT submission.rejected AND submission.review_status = $2
         )
         RETURNING referred_by",
    )
    .bind(user_id)
    .bind(APPROVED)
    .fetch_optional(&mut *conn)
    .await?;
    if let Some(referrer) = referrer {
        sqlx::query("UPDATE \"user\" SET mips = mips + CASE WHEN id = $1 THEN $3::int ELSE $4::int END WHERE id IN ($1, $2)")
            .bind(referrer)
            .bind(user_id)
            .bind(REFERRER_MIPS)
            .bind(REFEREE_MIPS)
            .execute(&mut *conn)
            .await?;
    }
    Ok(())
}
