use crate::{accounts::profile, env, projects::ship::history, snapshots, tree};
use chrono::{DateTime, Utc};
use hc_helper::airtable::{AirtableTable, field_names, fields, get_records_with_fields};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{PgConnection, PgExecutor, PgPool};
use std::{
    collections::{BTreeSet, HashMap, HashSet, hash_map::Entry},
    sync::{LazyLock, Mutex, PoisonError},
    time::{Duration, Instant},
};

pub const APPROVED: &str = "2–Submitted";
pub const REFERRER_MIPS: i32 = 100;
pub const REFEREE_MIPS: i32 = 20;
const MAX_HOURS: f64 = 10_000.0;
const FULL_SYNC_EVERY: Duration = Duration::from_secs(60 * 60);
const REFERRAL_RETRY_EVERY: Duration = Duration::from_secs(60 * 60);
const REFERRAL_BACKOFF_MAX: Duration = Duration::from_secs(7 * 24 * 60 * 60);
const TIERS: [(&str, &str); 3] = [("quebec", "Quebec"), ("romeo", "Romeo"), ("sierra", "Sierra")];

static REPORTED: LazyLock<Mutex<HashSet<String>>> = LazyLock::new(Default::default);

const READ_FIELDS: &[&str] = &[
    "Automation - Status",
    "Optional - Override Hours Spent",
    "Code URL",
    "Rejected",
    "Submission Tier",
    "Rejection Reason",
];

#[derive(Serialize)]
pub struct Extras {
    #[serde(rename = "Optional - Override Hours Spent")]
    pub hours: f64,
    #[serde(rename = "Justification - Submitter Hackatime ID")]
    pub hackatime_id: String,
    #[serde(rename = "GitHub Username", skip_serializing_if = "Option::is_none")]
    pub github_username: Option<String>,
    #[serde(rename = "Submission Tier")]
    pub tier: &'static str,
    #[serde(rename = "Shipped At")]
    pub shipped_at: DateTime<Utc>,
}

#[derive(Deserialize)]
struct Review {
    #[serde(rename = "Automation - Status")]
    status: Option<Value>,
    #[serde(rename = "Optional - Override Hours Spent")]
    hours: Option<Value>,
    #[serde(rename = "Code URL")]
    code_url: Option<Value>,
    #[serde(rename = "Rejected")]
    rejected: Option<Value>,
    #[serde(rename = "Submission Tier")]
    tier: Option<Value>,
    #[serde(rename = "Rejection Reason")]
    reason: Option<Value>,
}

impl Review {
    fn hours(&self) -> Option<Option<f64>> {
        let hours = match &self.hours {
            None | Some(Value::Null) => Some(None),
            Some(Value::Number(hours)) => hours.as_f64().map(Some),
            Some(Value::String(hours)) if hours.trim().is_empty() => Some(None),
            Some(Value::String(hours)) => hours
                .trim()
                .parse()
                .ok()
                .map(Some),
            _ => None,
        }?;
        match hours {
            Some(hours) if !(0.0..=MAX_HOURS).contains(&hours) => None,
            hours => Some(hours),
        }
    }

    fn rejected(&self) -> Option<bool> {
        match &self.rejected {
            None | Some(Value::Null) => Some(false),
            Some(Value::Bool(rejected)) => Some(*rejected),
            _ => None,
        }
    }

    fn status(&self) -> Option<Option<&str>> {
        match &self.status {
            None | Some(Value::Null) => Some(None),
            Some(Value::String(status)) => Some(Some(status)),
            _ => None,
        }
    }

    fn code_url(&self) -> Option<&str> {
        match &self.code_url {
            None | Some(Value::Null) => Some(""),
            Some(Value::String(url)) => Some(url),
            _ => None,
        }
    }

    fn reason(&self) -> Option<String> {
        match &self.reason {
            Some(Value::String(reason)) if !reason.trim().is_empty() => Some(reason.trim().to_string()),
            _ => None,
        }
    }

    fn tier(&self) -> Option<Option<&'static str>> {
        match &self.tier {
            None | Some(Value::Null) => Some(None),
            Some(Value::String(label)) if label.trim().is_empty() => Some(None),
            Some(Value::String(label)) => TIERS
                .iter()
                .find(|(_, name)| name.eq_ignore_ascii_case(label.trim()))
                .map(|(tier, _)| Some(*tier)),
            _ => None,
        }
    }
}

#[derive(sqlx::FromRow)]
struct Submission {
    id: i32,
    record_id: Option<String>,
    mips_per_hour: f64,
    review_status: Option<String>,
    rejected: bool,
    hours: Option<f64>,
    tracked_hours: Option<f64>,
    multiplier: Option<f64>,
    mips: i32,
    bonus_percent: f64,
    bonus_mips: i32,
    deleted: bool,
}

const TEXT: &[&str] = &["singleLineText", "multilineText", "richText"];
const URL: &[&str] = &["url", "singleLineText", "multilineText", "richText"];
const EMAIL: &[&str] = &["email", "singleLineText", "multilineText", "richText"];
const STATUS: &[&str] = &["formula", "singleSelect", "singleLineText", "multilineText", "richText"];

const FIELDS: &[(&str, &[&str])] = &[
    ("Code URL", URL),
    ("Playable URL", URL),
    ("First Name", TEXT),
    ("Last Name", TEXT),
    ("Email", EMAIL),
    ("Screenshot", &["multipleAttachments"]),
    ("Description", TEXT),
    ("Address (Line 1)", TEXT),
    ("Address (Line 2)", TEXT),
    ("City", TEXT),
    ("State / Province", TEXT),
    ("ZIP / Postal Code", TEXT),
    ("Country", TEXT),
    ("Birthday", &["date"]),
    ("Justification - Hackatime Project Name(s) + Date Range(s)", TEXT),
    ("Optional - Override Hours Spent", &["number"]),
    ("Justification - Submitter Hackatime ID", TEXT),
    ("GitHub Username", TEXT),
    ("Automation - Status", STATUS),
    ("Rejected", &["checkbox"]),
    ("Submission Tier", &["singleSelect"]),
    ("Shipped At", &["dateTime"]),
    ("Rejection Reason", TEXT),
];

pub fn table() -> Option<AirtableTable> {
    Some(AirtableTable {
        token: env::var("AIRTABLE_TOKEN")?,
        base_id: env::var("AIRTABLE_REVIEW_BASE_ID")?,
        table_id_or_name: env::var("AIRTABLE_REVIEW_TABLE")?,
    })
}

pub fn tier_label(tier: &str) -> anyhow::Result<&'static str> {
    TIERS
        .iter()
        .find(|(key, _)| *key == tier)
        .map(|(_, label)| *label)
        .ok_or_else(|| anyhow::anyhow!("unknown tier {tier}"))
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
    let referrals = pool.clone();
    actix_web::rt::spawn(async move {
        match field_problems(&table).await {
            Ok(problems) if !problems.is_empty() => log::error!(
                "Review Airtable table fields aren't set up right: {}",
                problems.join("; ")
            ),
            Ok(_) => {}
            Err(e) => log::warn!(
                "Couldn't check the review Airtable table's fields (the token needs schema.bases:read): {e:#}"
            ),
        }
        let mut interval = env::interval_secs("REVIEW_SYNC_INTERVAL_SECS", 120);
        let mut synced_all: Option<Instant> = None;
        loop {
            interval.tick().await;
            let all = synced_all.is_none_or(|at| at.elapsed() >= FULL_SYNC_EVERY);
            match sync(&pool, &table, all).await {
                Ok(true) if all => synced_all = Some(Instant::now()),
                Ok(_) => {}
                Err(e) => log::error!("Review sync failed: {e:#}"),
            }
        }
    });
    actix_web::rt::spawn(async move {
        let mut interval = actix_web::rt::time::interval(REFERRAL_RETRY_EVERY);
        let mut failures: HashMap<i32, (u32, Instant)> = HashMap::new();
        loop {
            interval.tick().await;
            if let Err(e) = retry_referrals(&referrals, &mut failures).await {
                log::error!("Retrying referral bonuses failed: {e:#}");
            }
        }
    });
}

async fn missing_fields(table: &AirtableTable) -> anyhow::Result<Vec<&'static str>> {
    let fields = field_names(table.clone()).await?;
    Ok(FIELDS
        .iter()
        .map(|(field, _)| *field)
        .filter(|field| !fields.iter().any(|name| name == field))
        .collect())
}

async fn field_problems(table: &AirtableTable) -> anyhow::Result<Vec<String>> {
    let fields = fields(table.clone()).await?;
    let mut problems = Vec::new();
    for (name, kinds) in FIELDS {
        let Some(field) = fields.iter().find(|field| field.name == *name) else {
            problems.push(format!("{name} is missing"));
            continue;
        };
        if !kinds.contains(&field.kind.as_str()) {
            problems.push(format!("{name} is {} but should be {}", field.kind, kinds.join(" or ")));
        } else if field.kind == "singleSelect" && *name == "Submission Tier" {
            let choices = field.options["choices"].as_array();
            let missing: Vec<&str> = TIERS
                .iter()
                .map(|(_, label)| *label)
                .filter(|label| {
                    !choices.is_some_and(|choices| choices.iter().any(|choice| choice["name"] == *label))
                })
                .collect();
            if !missing.is_empty() {
                problems.push(format!("{name} is missing the options {}", missing.join(", ")));
            }
        }
    }
    Ok(problems)
}

async fn sync(pool: &PgPool, table: &AirtableTable, all: bool) -> anyhow::Result<bool> {
    let ids: Vec<String> = sqlx::query_scalar(
        "SELECT record_id FROM submission
         WHERE record_id IS NOT NULL AND CASE WHEN $1 THEN deleted_at IS NULL OR deleted_at > now() - interval '30 days'
             ELSE updated_at > now() - interval '7 days'
                 OR (deleted_at IS NULL AND NOT rejected AND review_status IS DISTINCT FROM $2)
         END",
    )
    .bind(all)
    .bind(APPROVED)
    .fetch_all(pool)
    .await?;
    if ids.is_empty() {
        return Ok(true);
    }
    let fields = missing_fields(table).await;
    if let Ok(missing) = &fields
        && !missing.is_empty()
    {
        log::error!(
            "Review Airtable table is missing fields: {}, skipping review sync",
            missing.join(", ")
        );
        return Ok(false);
    }
    let reviews = get_records_with_fields::<Review>(table.clone(), &ids, READ_FIELDS).await?;
    if let Err(e) = &fields
        && reviews.is_empty()
    {
        log::warn!(
            "None of the {} submissions are in the review table and its fields couldn't be checked, skipping review sync: {e:#}",
            ids.len()
        );
        return Ok(false);
    }
    let users = apply(pool, table, &ids, reviews).await?;
    pay_referrals(pool, users).await;
    Ok(true)
}

async fn apply(
    pool: &PgPool,
    table: &AirtableTable,
    ids: &[String],
    reviews: Vec<(String, Review)>,
) -> anyhow::Result<Vec<i32>> {
    let present: Vec<String> = reviews.iter().map(|(record_id, _)| record_id.clone()).collect();
    let mut found = Vec::new();
    let mut statuses = Vec::new();
    let mut hours = Vec::new();
    let mut code_urls = Vec::new();
    let mut rejected = Vec::new();
    let mut tiers = Vec::new();
    let mut rates = Vec::new();
    let mut reasons = Vec::new();
    let mut superseded: HashSet<String> = sqlx::query_scalar(
        "SELECT s.record_id FROM submission s JOIN project p ON p.id = s.project_id
         WHERE s.record_id = ANY($1) AND (s.deleted_at IS NOT NULL OR s.rejected)
         AND (p.deleted_at IS NOT NULL OR EXISTS (
             SELECT 1 FROM submission n WHERE n.project_id = s.project_id AND n.deleted_at IS NULL AND n.id > s.id
         ))",
    )
    .bind(&present)
    .fetch_all(pool)
    .await?
    .into_iter()
    .collect();
    let candidates: Vec<(String, i32, i32)> = sqlx::query_as(
        "SELECT s.record_id, s.id, p.user_id FROM submission s JOIN project p ON p.id = s.project_id
         WHERE s.record_id = ANY($1) AND NOT s.record_id = ANY($2) AND (s.deleted_at IS NOT NULL OR s.rejected)
         ORDER BY p.user_id",
    )
    .bind(&present)
    .bind(superseded.iter().cloned().collect::<Vec<_>>())
    .fetch_all(pool)
    .await?;
    let mut shipped: HashMap<i32, Vec<history::Shipped>> = HashMap::new();
    for (record_id, id, user_id) in candidates {
        let ships = match shipped.entry(user_id) {
            Entry::Occupied(entry) => entry.into_mut(),
            Entry::Vacant(entry) => entry.insert(history::shipped(pool, table, user_id).await?),
        };
        let Some(names) = ships.iter().find(|ship| ship.id == id).map(|ship| &ship.names) else {
            continue;
        };
        if ships.iter().any(|ship| ship.id > id && ship.live && !ship.names.is_disjoint(names)) {
            superseded.insert(record_id);
        }
    }
    for (record_id, review) in reviews {
        if superseded.contains(&record_id) {
            if review.status() == Some(Some(APPROVED))
                && review.rejected() == Some(false)
                && REPORTED.lock().unwrap_or_else(PoisonError::into_inner).insert(record_id.clone())
            {
                log::warn!(
                    "Review {record_id} changed after its project reshipped or was deleted, leaving it deleted or rejected"
                );
            }
            continue;
        }
        let (Some(record_status), Some(record_hours), Some(record_code_url), Some(record_rejected), Some(record_tier)) =
            (review.status(), review.hours(), review.code_url(), review.rejected(), review.tier())
        else {
            log::error!(
                "Review {record_id} has an unreadable status ({:?}), hours ({:?}), code URL ({:?}), rejected ({:?}) or tier ({:?}), skipping it",
                review.status,
                review.hours,
                review.code_url,
                review.rejected,
                review.tier
            );
            continue;
        };
        let (record_tier, rate) = match record_tier.map(|tier| mips_per_hour(Some(tier))).transpose() {
            Ok(rate) => (record_tier, rate),
            Err(e) => {
                let tier = record_tier.unwrap_or_default();
                if REPORTED.lock().unwrap_or_else(PoisonError::into_inner).insert(format!("rate:{tier}")) {
                    log::error!("{tier} reviews can't be priced, keeping their current tier and rate: {e:#}");
                }
                (None, None)
            }
        };
        found.push(record_id);
        statuses.push(record_status.map(str::to_string));
        hours.push(record_hours);
        code_urls.push(record_code_url.to_string());
        rejected.push(record_rejected);
        tiers.push(record_tier);
        rates.push(rate);
        reasons.push(review.reason());
    }

    let mut tx = pool.begin().await?;
    let mut changed: BTreeSet<i32> = sqlx::query_scalar(
        "UPDATE submission s
         SET review_status = r.status, hours = r.hours, code_url = r.code_url, rejected = r.rejected,
             approved_at = CASE WHEN r.status = $6 AND NOT r.rejected THEN COALESCE(s.approved_at, now()) END,
             tier = COALESCE(r.tier, s.tier),
             mips_per_hour = CASE WHEN r.tier IS DISTINCT FROM s.tier THEN COALESCE(r.rate, s.mips_per_hour)
                 ELSE s.mips_per_hour END,
             deleted_at = NULL
         FROM UNNEST($1::text[], $2::text[], $3::float8[], $4::text[], $5::bool[], $7::text[], $8::float8[])
             AS r (record_id, status, hours, code_url, rejected, tier, rate)
         WHERE s.record_id = r.record_id
         AND (s.review_status, s.hours, s.code_url, s.rejected, s.deleted_at, s.tier)
             IS DISTINCT FROM (r.status, r.hours, r.code_url, r.rejected, NULL::timestamptz, COALESCE(r.tier, s.tier))
         RETURNING s.project_id",
    )
    .bind(&found)
    .bind(&statuses)
    .bind(&hours)
    .bind(&code_urls)
    .bind(&rejected)
    .bind(APPROVED)
    .bind(&tiers)
    .bind(&rates)
    .fetch_all(&mut *tx)
    .await?
    .into_iter()
    .collect();
    sqlx::query(
        "UPDATE submission s SET rejection_reason = r.reason
         FROM UNNEST($1::text[], $2::text[]) AS r (record_id, reason)
         WHERE s.record_id = r.record_id AND s.rejection_reason IS DISTINCT FROM r.reason",
    )
    .bind(&found)
    .bind(&reasons)
    .execute(&mut *tx)
    .await?;
    let missing: Vec<(i32, String, bool)> = sqlx::query_as(
        "SELECT project_id, record_id, NOT rejected AND review_status IS NOT DISTINCT FROM $3
         FROM submission WHERE record_id = ANY($1) AND NOT record_id = ANY($2) AND deleted_at IS NULL",
    )
    .bind(ids)
    .bind(&present)
    .bind(APPROVED)
    .fetch_all(&mut *tx)
    .await?;
    let mut removed = Vec::new();
    for (project_id, record_id, approved) in missing {
        if !approved {
            log::warn!("Review {record_id} of project {project_id} is gone from the review table");
            changed.insert(project_id);
            removed.push(record_id);
        } else if REPORTED.lock().unwrap_or_else(PoisonError::into_inner).insert(record_id.clone()) {
            log::error!(
                "Approved review {record_id} of project {project_id} is gone from the review table, keeping its Mips"
            );
        }
    }
    sqlx::query("UPDATE submission SET deleted_at = now() WHERE record_id = ANY($1)")
        .bind(&removed)
        .execute(&mut *tx)
        .await?;
    let changed_ids: Vec<i32> = changed.iter().copied().collect();
    sqlx::query(
        "UPDATE project p SET tier = s.tier
         FROM (SELECT DISTINCT ON (project_id) project_id, tier, rejected FROM submission
               WHERE project_id = ANY($1) AND deleted_at IS NULL
               ORDER BY project_id, created_at DESC, id DESC) s
         WHERE p.id = s.project_id AND NOT s.rejected AND s.tier IS NOT NULL AND p.tier IS DISTINCT FROM s.tier",
    )
    .bind(&changed_ids)
    .execute(&mut *tx)
    .await?;
    let mut lowered = BTreeSet::new();
    let mut users = BTreeSet::new();
    for project_id in changed {
        let (user_id, project_lowered) = reconcile(&mut tx, project_id).await?;
        users.insert(user_id);
        lowered.extend(project_lowered);
    }
    refund_overdrawn(&mut tx, lowered).await?;
    tx.commit().await?;
    Ok(users.into_iter().collect())
}

async fn refund_overdrawn(
    conn: &mut PgConnection,
    users: impl IntoIterator<Item = i32>,
) -> anyhow::Result<()> {
    for user_id in users {
        let overdrawn: bool = sqlx::query_scalar("SELECT mips < 0 FROM \"user\" WHERE id = $1")
            .bind(user_id)
            .fetch_one(&mut *conn)
            .await?;
        if overdrawn {
            tree::refund(conn, user_id).await?;
        }
    }
    Ok(())
}

pub async fn rebalance(conn: &mut PgConnection, project_id: i32) -> anyhow::Result<()> {
    let (_, lowered) = reconcile(conn, project_id).await?;
    refund_overdrawn(conn, lowered).await
}

async fn reconcile(conn: &mut PgConnection, project_id: i32) -> anyhow::Result<(i32, Vec<i32>)> {
    let user_id: i32 = sqlx::query_scalar("SELECT user_id FROM project WHERE id = $1")
        .bind(project_id)
        .fetch_one(&mut *conn)
        .await?;
    let submissions: Vec<Submission> = sqlx::query_as(
        "SELECT id, record_id, mips_per_hour, review_status, rejected, hours, tracked_hours, multiplier, mips,
             bonus_percent, bonus_mips, deleted_at IS NOT NULL AS deleted
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

    let mut delta: i32 = 0;
    let mut legacy_hours = 0.0_f64;
    for submission in submissions {
        let approved = !submission.deleted
            && !submission.rejected
            && submission.review_status.as_deref() == Some(APPROVED);
        let (mips, multiplier) = match (approved, submission.hours) {
            (true, Some(hours)) => {
                let multiplier = submission
                    .multiplier
                    .unwrap_or_else(|| snapshots::multiplier(shared));
                let earned = match submission.tracked_hours {
                    Some(_) => hours,
                    None => {
                        let earned = (hours - legacy_hours).max(0.0);
                        legacy_hours = legacy_hours.max(hours);
                        earned
                    }
                };
                let bonus = 1.0 + submission.bonus_percent / 100.0;
                let mips = ((earned * submission.mips_per_hour * multiplier * bonus).round() as i32)
                    .saturating_add(submission.bonus_mips);
                (mips, Some(multiplier))
            }
            (true, None) => {
                log::warn!(
                    "Review {} was approved without hours, so it earns nothing until hours are set",
                    submission.record_id.as_deref().unwrap_or_default()
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
            delta = delta.saturating_add(mips - submission.mips);
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
    let mut lowered = Vec::new();
    if delta < 0 {
        lowered.push(user_id);
    }
    lowered.extend(revoke_referral(conn, user_id).await?);
    Ok((user_id, lowered))
}

async fn has_approved(db: impl PgExecutor<'_>, user_id: i32) -> sqlx::Result<bool> {
    sqlx::query_scalar(
        "SELECT EXISTS (
             SELECT 1 FROM submission JOIN project ON project.id = submission.project_id
             WHERE project.user_id = $1 AND submission.deleted_at IS NULL
             AND NOT submission.rejected AND submission.review_status = $2
         )",
    )
    .bind(user_id)
    .bind(APPROVED)
    .fetch_one(db)
    .await
}

async fn reward(conn: &mut PgConnection, referrer: i32, user_id: i32, sign: i32) -> sqlx::Result<()> {
    sqlx::query("UPDATE \"user\" SET mips = mips + CASE WHEN id = $1 THEN $3::int ELSE $4::int END WHERE id IN ($1, $2)")
        .bind(referrer)
        .bind(user_id)
        .bind(sign * REFERRER_MIPS)
        .bind(sign * REFEREE_MIPS)
        .execute(conn)
        .await?;
    Ok(())
}

async fn revoke_referral(conn: &mut PgConnection, user_id: i32) -> anyhow::Result<Vec<i32>> {
    if has_approved(&mut *conn, user_id).await? {
        return Ok(Vec::new());
    }
    let referrer: Option<i32> = sqlx::query_scalar(
        "UPDATE \"user\" SET referral_paid = false WHERE id = $1 AND referral_paid RETURNING referred_by",
    )
    .bind(user_id)
    .fetch_optional(&mut *conn)
    .await?
    .flatten();
    let Some(referrer) = referrer else {
        return Ok(Vec::new());
    };
    log::warn!("User {user_id} has no approved submission left, revoking the referral bonus");
    reward(conn, referrer, user_id, -1).await?;
    Ok(vec![referrer, user_id])
}

async fn retry_referrals(pool: &PgPool, failures: &mut HashMap<i32, (u32, Instant)>) -> anyhow::Result<()> {
    let users: Vec<i32> = sqlx::query_scalar(
        "SELECT id FROM \"user\" WHERE referred_by IS NOT NULL AND NOT referral_paid
         AND EXISTS (
             SELECT 1 FROM submission JOIN project ON project.id = submission.project_id
             WHERE project.user_id = \"user\".id AND submission.deleted_at IS NULL
             AND NOT submission.rejected AND submission.review_status = $1
         )",
    )
    .bind(APPROVED)
    .fetch_all(pool)
    .await?;
    failures.retain(|user_id, _| users.contains(user_id));
    for user_id in users {
        if failures.get(&user_id).is_some_and(|(_, next)| Instant::now() < *next) {
            continue;
        }
        let paid = pay_referral(pool, user_id).await.unwrap_or_else(|e| {
            log::error!("Paying user {user_id}'s referral bonus failed: {e:#}");
            false
        });
        if paid {
            failures.remove(&user_id);
        } else {
            let count = failures.get(&user_id).map_or(1, |(count, _)| count + 1);
            let wait = REFERRAL_RETRY_EVERY
                .saturating_mul(2_u32.saturating_pow(count.min(16)))
                .min(REFERRAL_BACKOFF_MAX);
            failures.insert(user_id, (count, Instant::now() + wait));
        }
    }
    Ok(())
}

async fn pay_referrals(pool: &PgPool, users: Vec<i32>) {
    for user_id in users {
        if let Err(e) = pay_referral(pool, user_id).await {
            log::error!("Paying user {user_id}'s referral bonus failed: {e:#}");
        }
    }
}

async fn pay_referral(pool: &PgPool, user_id: i32) -> anyhow::Result<bool> {
    let pending: Option<(i32, String, String)> = sqlx::query_as(
        "SELECT referrer.id, referrer.hca_id, \"user\".hca_id
         FROM \"user\" JOIN \"user\" referrer ON referrer.id = \"user\".referred_by
         WHERE \"user\".id = $1 AND NOT \"user\".referral_paid
         AND \"user\".deleted_at IS NULL AND NOT \"user\".is_banned
         AND referrer.deleted_at IS NULL AND NOT referrer.is_banned",
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await?;
    let Some((referrer, referrer_hca_id, hca_id)) = pending else {
        return Ok(true);
    };
    if !has_approved(pool, user_id).await? {
        return Ok(true);
    }
    if !(profile::ysws_eligible(&referrer_hca_id).await? && profile::ysws_eligible(&hca_id).await?) {
        log::info!(
            "User {user_id} or their referrer {referrer} isn't eligible, skipping the referral bonus"
        );
        return Ok(false);
    }
    let mut tx = pool.begin().await?;
    let claimed = sqlx::query(
        "UPDATE \"user\" SET referral_paid = true WHERE id = $1 AND NOT referral_paid",
    )
    .bind(user_id)
    .execute(&mut *tx)
    .await?
    .rows_affected()
        == 1;
    if claimed && has_approved(&mut *tx, user_id).await? {
        reward(&mut tx, referrer, user_id, 1).await?;
        tx.commit().await?;
    }
    Ok(true)
}
