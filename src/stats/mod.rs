use crate::{error::ApiError, middleware, projects::review};
use actix_web::{HttpRequest, HttpResponse, get, post, web, web::Bytes};
use chrono::{NaiveDate, Utc};
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

const CACHE_FOR: Duration = Duration::from_secs(60 * 60);

const VISITS_PER_IP: u32 = 50;

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
struct Totals {
    hours_building: f64,
    projects_shipped: i64,
    hours_approved: f64,
    mips_awarded: i64,
    snapshots_shared: i64,
    successful_referrals: i64,
    orders_placed: i64,
}

#[derive(Serialize, sqlx::FromRow)]
struct Funnel {
    visited: i64,
    signed_in: i64,
    linked_hackatime: i64,
    logged_time: i64,
    shipped: i64,
    redeemed: i64,
}

#[derive(Serialize)]
struct Stats {
    days: Vec<Day>,
    totals: Totals,
    funnel: Funnel,
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
    let totals = sqlx::query_as(
        "SELECT
             (SELECT COALESCE(sum(total_seconds), 0) FROM project WHERE deleted_at IS NULL)::float8
                 / 3600 AS hours_building,
             (SELECT count(DISTINCT project_id) FROM submission WHERE deleted_at IS NULL)
                 AS projects_shipped,
             (SELECT COALESCE(sum(hours), 0) FROM (
                  SELECT max(hours) AS hours FROM submission
                  WHERE deleted_at IS NULL AND NOT rejected AND review_status = $1
                  GROUP BY project_id
              ) approved) AS hours_approved,
             (SELECT COALESCE(sum(mips), 0) FROM submission)
                 + (SELECT count(*) FROM \"user\" WHERE referral_paid) * $2 AS mips_awarded,
             (SELECT count(*) FROM snapshot WHERE deleted_at IS NULL) AS snapshots_shared,
             (SELECT count(*) FROM \"user\" WHERE referral_paid) AS successful_referrals,
             (SELECT count(*) FROM \"order\" WHERE deleted_at IS NULL) AS orders_placed",
    )
    .bind(review::APPROVED)
    .bind(review::REFERRAL_MIPS)
    .fetch_one(pool)
    .await?;
    let funnel = sqlx::query_as(
        "SELECT
             (SELECT count(*) FROM visitor) AS visited,
             count(*) AS signed_in,
             count(*) FILTER (WHERE hackatime_user_id IS NOT NULL) AS linked_hackatime,
             count(*) FILTER (WHERE EXISTS (SELECT 1 FROM active_day WHERE user_id = \"user\".id))
                 AS logged_time,
             count(*) FILTER (WHERE EXISTS (
                 SELECT 1 FROM submission JOIN project ON project.id = submission.project_id
                 WHERE project.user_id = \"user\".id AND submission.deleted_at IS NULL
             )) AS shipped,
             count(*) FILTER (WHERE EXISTS (
                 SELECT 1 FROM \"order\" WHERE user_id = \"user\".id AND deleted_at IS NULL
             )) AS redeemed
         FROM \"user\" WHERE deleted_at IS NULL",
    )
    .fetch_one(pool)
    .await?;
    Ok(serde_json::to_vec(&Stats { days, totals, funnel })?.into())
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
