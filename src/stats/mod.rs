use crate::{error::ApiError, middleware, projects::review, tree};
use actix_web::{HttpRequest, HttpResponse, get, post, web, web::Bytes};
use chrono::{DateTime, NaiveDate, Utc};
use serde::Serialize;
use sqlx::PgPool;
use std::{
    collections::HashMap,
    net::IpAddr,
    sync::{LazyLock, PoisonError},
    time::{Duration, Instant},
};
use tokio::sync::Mutex;

mod sync;

pub use sync::spawn;

const CACHE_FOR: Duration = Duration::from_secs(30);

const VISITS_PER_IP: u32 = 50;
const VISITOR_IPS: usize = 100_000;

static CACHE: LazyLock<Mutex<Option<(Instant, Bytes)>>> = LazyLock::new(Default::default);
static VISITS: LazyLock<std::sync::Mutex<(NaiveDate, HashMap<IpAddr, u32>)>> =
    LazyLock::new(Default::default);

pub fn api(cfg: &mut web::ServiceConfig) {
    cfg.service((get_stats, record_visit));
}

pub fn pages(cfg: &mut web::ServiceConfig) {
    cfg.service(page);
}

#[derive(Serialize, sqlx::FromRow)]
struct Day {
    date: NaiveDate,
    signed_up: i64,
    active: i64,
}

#[derive(Serialize, sqlx::FromRow)]
struct Hour {
    hour: DateTime<Utc>,
    signed_up: i64,
    logged_in: i64,
}

#[derive(Serialize, sqlx::FromRow)]
struct Totals {
    hours_building: f64,
    projects_created: i64,
    projects_shipped: i64,
    hours_approved: f64,
    mips_awarded: i64,
    snapshots_shared: i64,
    successful_referrals: i64,
    orders_placed: i64,
    logged_in_last_day: i64,
    awaiting_review: i64,
    average_wait_hours: Option<f64>,
    average_approval_hours: Option<f64>,
}

#[derive(Serialize, sqlx::FromRow)]
struct Funnel {
    visited: i64,
    signed_in: i64,
    linked_hackatime: i64,
    created_project: i64,
    logged_time: i64,
    shipped: i64,
    redeemed: i64,
}

#[derive(Serialize)]
struct Funnels {
    day: Funnel,
    week: Funnel,
    all: Funnel,
}

#[derive(Serialize, sqlx::FromRow)]
struct OnboardingTotals {
    users: i64,
    started: i64,
    finished: i64,
    median_minutes: Option<f64>,
    stickers: i64,
}

#[derive(Serialize, sqlx::FromRow)]
struct OnboardingStep {
    step: String,
    users: i64,
}

#[derive(Serialize)]
struct Onboarding {
    #[serde(flatten)]
    totals: OnboardingTotals,
    steps: Vec<OnboardingStep>,
}

#[derive(Serialize)]
struct Onboardings {
    day: Onboarding,
    week: Onboarding,
    all: Onboarding,
}

#[derive(Serialize)]
struct Stats {
    days: Vec<Day>,
    hours: Vec<Hour>,
    totals: Totals,
    funnel: Funnels,
    onboarding: Onboardings,
}

async fn onboarding(pool: &PgPool, hours: Option<i32>) -> sqlx::Result<Onboarding> {
    let totals = sqlx::query_as(
        "SELECT count(*) AS users,
             count(*) FILTER (WHERE onboarding_started_at IS NOT NULL) AS started,
             count(*) FILTER (WHERE onboarded_at IS NOT NULL) AS finished,
             (percentile_cont(0.5) WITHIN GROUP (
                  ORDER BY extract(epoch FROM onboarded_at - onboarding_started_at)
              ) FILTER (WHERE onboarded_at IS NOT NULL AND onboarding_started_at IS NOT NULL))::float8
                 / 60 AS median_minutes,
             count(*) FILTER (WHERE EXISTS (
                 SELECT 1 FROM \"order\"
                 WHERE user_id = \"user\".id AND node_id = $2 AND deleted_at IS NULL
             )) AS stickers
         FROM \"user\"
         WHERE deleted_at IS NULL AND ($1::int IS NULL OR signup_at >= now() - make_interval(hours => $1))",
    )
    .bind(hours)
    .bind(tree::STICKER as i32)
    .fetch_one(pool)
    .await?;
    let steps = sqlx::query_as(
        "SELECT onboarding_step AS step, count(*) AS users FROM \"user\"
         WHERE deleted_at IS NULL AND ($1::int IS NULL OR signup_at >= now() - make_interval(hours => $1))
             AND onboarding_started_at IS NOT NULL AND onboarding_step IS NOT NULL
         GROUP BY 1",
    )
    .bind(hours)
    .fetch_all(pool)
    .await?;
    Ok(Onboarding { totals, steps })
}

async fn funnel(pool: &PgPool, hours: Option<i32>) -> sqlx::Result<Funnel> {
    sqlx::query_as(
        "SELECT
             (SELECT count(*) FROM visitor
              WHERE $1::int IS NULL OR visited_at >= now() - make_interval(hours => $1)) AS visited,
             count(*) AS signed_in,
             count(*) FILTER (WHERE hackatime_user_id IS NOT NULL) AS linked_hackatime,
             count(*) FILTER (WHERE EXISTS (
                 SELECT 1 FROM project WHERE user_id = \"user\".id AND deleted_at IS NULL
             )) AS created_project,
             count(*) FILTER (WHERE EXISTS (SELECT 1 FROM active_day WHERE user_id = \"user\".id))
                 AS logged_time,
             count(*) FILTER (WHERE EXISTS (
                 SELECT 1 FROM submission JOIN project ON project.id = submission.project_id
                 WHERE project.user_id = \"user\".id AND submission.deleted_at IS NULL
             )) AS shipped,
             count(*) FILTER (WHERE EXISTS (
                 SELECT 1 FROM \"order\"
                 WHERE user_id = \"user\".id AND deleted_at IS NULL AND node_id <> $2
             )) AS redeemed
         FROM \"user\"
         WHERE deleted_at IS NULL
             AND ($1::int IS NULL OR signup_at >= now() - make_interval(hours => $1))",
    )
    .bind(hours)
    .bind(tree::STICKER as i32)
    .fetch_one(pool)
    .await
}

async fn build(pool: &PgPool) -> anyhow::Result<Bytes> {
    let days = sqlx::query_as(
        "WITH signups AS (
             SELECT (signup_at AT TIME ZONE 'UTC')::date AS day, count(*) AS users
             FROM \"user\" WHERE deleted_at IS NULL GROUP BY 1
         ), active AS (
             SELECT day, count(*) AS users FROM active_day GROUP BY day
         ), range AS (
             SELECT generate_series(
                 LEAST((SELECT min(day) FROM signups), (now() AT TIME ZONE 'UTC')::date)::timestamp,
                 (now() AT TIME ZONE 'UTC')::date::timestamp,
                 interval '1 day'
             )::date AS day
         )
         SELECT range.day AS date,
             (sum(COALESCE(signups.users, 0)) OVER (ORDER BY range.day))::bigint AS signed_up,
             COALESCE(active.users, 0) AS active
         FROM range
         LEFT JOIN signups USING (day)
         LEFT JOIN active USING (day)
         ORDER BY range.day",
    )
    .fetch_all(pool)
    .await?;
    let hours = sqlx::query_as(
        "WITH signups AS (
             SELECT date_trunc('hour', signup_at AT TIME ZONE 'UTC') AS hour, count(*) AS users
             FROM \"user\" WHERE deleted_at IS NULL GROUP BY 1
         ), logins AS (
             SELECT date_trunc('hour', login.logged_in_at AT TIME ZONE 'UTC') AS hour,
                 count(DISTINCT login.user_id) AS users
             FROM login JOIN \"user\" ON \"user\".id = login.user_id
             WHERE \"user\".deleted_at IS NULL GROUP BY 1
         ), range AS (
             SELECT generate_series(
                 LEAST((SELECT min(hour) FROM signups), date_trunc('hour', now() AT TIME ZONE 'UTC')),
                 date_trunc('hour', now() AT TIME ZONE 'UTC'),
                 interval '1 hour'
             ) AS hour
         )
         SELECT range.hour AT TIME ZONE 'UTC' AS hour,
             (sum(COALESCE(signups.users, 0)) OVER (ORDER BY range.hour))::bigint AS signed_up,
             COALESCE(logins.users, 0) AS logged_in
         FROM range
         LEFT JOIN signups USING (hour)
         LEFT JOIN logins USING (hour)
         ORDER BY range.hour",
    )
    .fetch_all(pool)
    .await?;
    let totals = sqlx::query_as(
        "SELECT
             (SELECT COALESCE(sum(total_seconds), 0) FROM project WHERE deleted_at IS NULL)::float8
                 / 3600 AS hours_building,
             (SELECT count(*) FROM project WHERE deleted_at IS NULL) AS projects_created,
             (SELECT count(DISTINCT project_id) FROM submission WHERE deleted_at IS NULL)
                 AS projects_shipped,
             (SELECT COALESCE(sum(hours), 0) FROM (
                  SELECT CASE WHEN bool_or(tracked_hours IS NULL) THEN max(hours) ELSE sum(hours) END AS hours
                  FROM submission WHERE deleted_at IS NULL AND NOT rejected AND review_status = $1
                  GROUP BY project_id, tracked_hours IS NULL
              ) approved) AS hours_approved,
             (SELECT COALESCE(sum(mips), 0) FROM submission)
                 + (SELECT count(*) FROM \"user\" WHERE referral_paid) * $2 AS mips_awarded,
             (SELECT count(*) FROM snapshot WHERE deleted_at IS NULL) AS snapshots_shared,
             (SELECT count(*) FROM \"user\" WHERE referral_paid) AS successful_referrals,
             (SELECT count(*) FROM \"order\" WHERE deleted_at IS NULL AND node_id <> $3) AS orders_placed,
             (SELECT count(DISTINCT login.user_id) FROM login JOIN \"user\" ON \"user\".id = login.user_id
              WHERE \"user\".deleted_at IS NULL AND login.logged_in_at > now() - interval '1 day')
                 AS logged_in_last_day,
             (SELECT count(*) FROM submission
              WHERE deleted_at IS NULL AND NOT rejected AND review_status IS DISTINCT FROM $1)
                 AS awaiting_review,
             (SELECT extract(epoch FROM avg(now() - created_at))::float8 / 3600 FROM submission
              WHERE deleted_at IS NULL AND NOT rejected AND review_status IS DISTINCT FROM $1)
                 AS average_wait_hours,
             (SELECT extract(epoch FROM avg(approved_at - created_at))::float8 / 3600 FROM submission
              WHERE deleted_at IS NULL AND approved_at IS NOT NULL)
                 AS average_approval_hours",
    )
    .bind(review::APPROVED)
    .bind(review::REFERRER_MIPS + review::REFEREE_MIPS)
    .bind(tree::STICKER as i32)
    .fetch_one(pool)
    .await?;
    let funnel = Funnels {
        day: funnel(pool, Some(24)).await?,
        week: funnel(pool, Some(24 * 7)).await?,
        all: funnel(pool, None).await?,
    };
    let onboarding = Onboardings {
        day: onboarding(pool, Some(24)).await?,
        week: onboarding(pool, Some(24 * 7)).await?,
        all: onboarding(pool, None).await?,
    };
    Ok(serde_json::to_vec(&Stats { days, hours, totals, funnel, onboarding })?.into())
}

#[get("/stats")]
async fn get_stats(pool: web::Data<PgPool>) -> Result<HttpResponse, ApiError> {
    let mut cache = CACHE.lock().await;
    let body = match &*cache {
        Some((built, body)) if built.elapsed() < CACHE_FOR => body.clone(),
        _ => {
            let body = build(&pool).await?;
            *cache = Some((Instant::now(), body.clone()));
            body
        }
    };
    Ok(HttpResponse::Ok().content_type("application/json").body(body))
}

fn allow_visit(ip: IpAddr) -> bool {
    let today = Utc::now().date_naive();
    let mut visits = VISITS.lock().unwrap_or_else(PoisonError::into_inner);
    if visits.0 != today {
        *visits = (today, HashMap::new());
    }
    if visits.1.len() >= VISITOR_IPS && !visits.1.contains_key(&ip) {
        return false;
    }
    let count = visits.1.entry(ip).or_default();
    *count = count.saturating_add(1);
    *count <= VISITS_PER_IP
}

#[post("/visit")]
async fn record_visit(req: HttpRequest, pool: web::Data<PgPool>) -> Result<HttpResponse, ApiError> {
    if middleware::client_ip(&req).is_some_and(allow_visit) {
        sqlx::query("INSERT INTO visitor DEFAULT VALUES")
            .execute(pool.get_ref())
            .await?;
    }
    Ok(HttpResponse::NoContent().finish())
}

#[get("/stats")]
async fn page(req: HttpRequest) -> HttpResponse {
    crate::html(&req, "stats").await
}
