use crate::{
    admin::Admin,
    error::{ApiError, reject_with},
    middleware,
    projects::review,
    tree,
};
use actix_web::{
    HttpRequest, HttpResponse, get,
    http::{StatusCode, header},
    post, web,
    web::Bytes,
};
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::PgPool;
use std::{
    collections::HashMap,
    net::{IpAddr, Ipv6Addr},
    sync::{LazyLock, PoisonError},
    time::{Duration, Instant},
};
use tokio::sync::Mutex;

mod sync;

pub use sync::spawn;

const CACHE_FOR: Duration = Duration::from_secs(30);
const RETRY_FIRST: Duration = Duration::from_secs(5);
const RETRY_MAX: Duration = Duration::from_secs(600);

const VISITS_PER_IP: u32 = 50;
const VISITOR_IPS: usize = 100_000;

const PUBLIC_MIN: i64 = 5;

struct Built {
    at: Instant,
    exact: Bytes,
    public: Bytes,
}

#[derive(Default)]
struct Cache {
    built: Option<Built>,
    retry_at: Option<Instant>,
    backoff: Duration,
}

static CACHE: LazyLock<Mutex<Cache>> = LazyLock::new(Default::default);

const ROUNDS_JSON: &str = include_str!("../../frontend/src/lib/assets/fulfillment_rounds.json");

#[derive(Deserialize)]
struct Round {
    ship_by: NaiveDate,
}

static SHIP_BY: LazyLock<Vec<NaiveDate>> = LazyLock::new(|| {
    let rounds: Vec<Round> = serde_json::from_str(ROUNDS_JSON).expect("rounds json must be valid");
    rounds.into_iter().map(|round| round.ship_by).collect()
});

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
    logged_in: i64,
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
    logged_in_last_week: i64,
    logged_in_ever: i64,
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
struct Onboarding {
    users: i64,
    started: i64,
    made_project: i64,
    reached_tech_tree: i64,
    unlocked_sticker: i64,
    stickers: i64,
    finished: i64,
    in_progress: i64,
    #[serde(skip)]
    timed: i64,
    median_minutes: Option<f64>,
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

async fn onboarding(pool: &PgPool, since: Option<DateTime<Utc>>) -> sqlx::Result<Onboarding> {
    sqlx::query_as(
        "SELECT count(*) AS users,
             count(*) FILTER (WHERE onboarding_started_at IS NOT NULL OR onboarded_at IS NOT NULL)
                 AS started,
             count(*) FILTER (WHERE onboarded_at IS NOT NULL
                 OR array_position($3::text[], onboarding_step) >= array_position($3::text[], 'Edit'))
                 AS made_project,
             count(*) FILTER (WHERE onboarded_at IS NOT NULL
                 OR array_position($3::text[], onboarding_step) >= array_position($3::text[], 'Tech Tree'))
                 AS reached_tech_tree,
             count(*) FILTER (WHERE onboarded_at IS NOT NULL
                 OR array_position($3::text[], onboarding_step) >= array_position($3::text[], 'Unlock token'))
                 AS unlocked_sticker,
             count(*) FILTER (WHERE EXISTS (
                 SELECT 1 FROM \"order\"
                 WHERE user_id = \"user\".id AND node_id = $2 AND deleted_at IS NULL
             )) AS stickers,
             count(*) FILTER (WHERE onboarded_at IS NOT NULL) AS finished,
             count(*) FILTER (WHERE onboarded_at IS NULL
                 AND onboarding_started_at >= now() - interval '7 days') AS in_progress,
             count(*) FILTER (WHERE onboarded_at IS NOT NULL AND onboarding_started_at IS NOT NULL)
                 AS timed,
             (percentile_cont(0.5) WITHIN GROUP (
                  ORDER BY extract(epoch FROM onboarded_at - onboarding_started_at)
              ) FILTER (WHERE onboarded_at IS NOT NULL AND onboarding_started_at IS NOT NULL))::float8
                 / 60 AS median_minutes
         FROM \"user\"
         WHERE deleted_at IS NULL AND ($1::timestamptz IS NULL OR signup_at >= $1)",
    )
    .bind(since)
    .bind(tree::STICKER as i32)
    .bind(&*crate::dashboard::ONBOARDING_STEPS)
    .fetch_one(pool)
    .await
}

async fn funnel(pool: &PgPool, since: Option<DateTime<Utc>>) -> sqlx::Result<Funnel> {
    let visited: i64 = match since {
        Some(since) => {
            sqlx::query_scalar("SELECT count(*) FROM visitor WHERE visited_at >= $1")
                .bind(since)
                .fetch_one(pool)
                .await?
        }
        None => sqlx::query_scalar("SELECT count(*) FROM visitor").fetch_one(pool).await?,
    };
    sqlx::query_as(
        "SELECT
             $3::bigint AS visited,
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
         WHERE deleted_at IS NULL AND ($1::timestamptz IS NULL OR signup_at >= $1)",
    )
    .bind(since)
    .bind(tree::STICKER as i32)
    .bind(visited)
    .fetch_one(pool)
    .await
}

async fn build(pool: &PgPool) -> anyhow::Result<Stats> {
    let days = sqlx::query_as(
        "WITH signups AS (
             SELECT (signup_at AT TIME ZONE 'UTC')::date AS day, count(*) AS users
             FROM \"user\" WHERE deleted_at IS NULL GROUP BY 1
         ), active AS (
             SELECT day, count(*) AS users FROM active_day GROUP BY day
         ), logins AS (
             SELECT (login.logged_in_at AT TIME ZONE 'UTC')::date AS day,
                 count(DISTINCT login.user_id) AS users
             FROM login JOIN \"user\" ON \"user\".id = login.user_id
             WHERE \"user\".deleted_at IS NULL GROUP BY 1
         ), range AS (
             SELECT generate_series(
                 LEAST((SELECT min(day) FROM signups), (now() AT TIME ZONE 'UTC')::date)::timestamp,
                 (now() AT TIME ZONE 'UTC')::date::timestamp,
                 interval '1 day'
             )::date AS day
         )
         SELECT range.day AS date,
             (sum(COALESCE(signups.users, 0)) OVER (ORDER BY range.day))::bigint AS signed_up,
             COALESCE(active.users, 0) AS active,
             COALESCE(logins.users, 0) AS logged_in
         FROM range
         LEFT JOIN signups USING (day)
         LEFT JOIN active USING (day)
         LEFT JOIN logins USING (day)
         ORDER BY range.day",
    )
    .fetch_all(pool)
    .await?;
    let hours = sqlx::query_as(
        "WITH range AS (
             SELECT generate_series(
                 date_trunc('hour', now() AT TIME ZONE 'UTC') - interval '23 hours',
                 date_trunc('hour', now() AT TIME ZONE 'UTC'),
                 interval '1 hour'
             ) AT TIME ZONE 'UTC' AS hour
         )
         SELECT hour,
             (SELECT count(*) FROM \"user\"
              WHERE deleted_at IS NULL AND signup_at < hour + interval '1 hour') AS signed_up,
             (SELECT count(DISTINCT login.user_id) FROM login JOIN \"user\" ON \"user\".id = login.user_id
              WHERE \"user\".deleted_at IS NULL
                  AND login.logged_in_at >= hour AND login.logged_in_at < hour + interval '1 hour')
                 AS logged_in
         FROM range
         ORDER BY hour",
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
             (SELECT count(DISTINCT login.user_id) FROM login JOIN \"user\" ON \"user\".id = login.user_id
              WHERE \"user\".deleted_at IS NULL AND login.logged_in_at > now() - interval '7 days')
                 AS logged_in_last_week,
             (SELECT count(DISTINCT login.user_id) FROM login JOIN \"user\" ON \"user\".id = login.user_id
              WHERE \"user\".deleted_at IS NULL)
                 AS logged_in_ever,
             (SELECT count(*) FROM submission
              WHERE record_id IS NOT NULL AND deleted_at IS NULL AND NOT rejected
                  AND review_status IS DISTINCT FROM $1)
                 AS awaiting_review,
             (SELECT extract(epoch FROM avg(now() - start))::float8 / 3600
              FROM (SELECT GREATEST(COALESCE(sending_at, created_at), (
                              SELECT (min(ship_by) + 1)::timestamp AT TIME ZONE 'UTC'
                              FROM unnest($4::date[]) AS ship_by
                              WHERE ship_by >= (created_at AT TIME ZONE 'UTC')::date
                          )) AS start FROM submission
                    WHERE record_id IS NOT NULL AND deleted_at IS NULL AND NOT rejected
                        AND review_status IS DISTINCT FROM $1) queued
              WHERE start <= now())
                 AS average_wait_hours,
             (SELECT extract(epoch FROM avg(GREATEST(approved_at - start, interval '0')))::float8 / 3600
              FROM (SELECT approved_at, GREATEST(COALESCE(sending_at, created_at), (
                              SELECT (min(ship_by) + 1)::timestamp AT TIME ZONE 'UTC'
                              FROM unnest($4::date[]) AS ship_by
                              WHERE ship_by >= (created_at AT TIME ZONE 'UTC')::date
                          )) AS start FROM submission
                    WHERE deleted_at IS NULL AND approved_at IS NOT NULL) approved)
                 AS average_approval_hours",
    )
    .bind(review::APPROVED)
    .bind(review::REFERRER_MIPS + review::REFEREE_MIPS)
    .bind(tree::STICKER as i32)
    .bind(&*SHIP_BY)
    .fetch_one(pool)
    .await?;
    let now = Utc::now();
    let day = Some(now - chrono::Duration::hours(24));
    let week = Some(now - chrono::Duration::days(7));
    let funnel = Funnels {
        day: funnel(pool, day).await?,
        week: funnel(pool, week).await?,
        all: funnel(pool, None).await?,
    };
    let onboarding = Onboardings {
        day: onboarding(pool, day).await?,
        week: onboarding(pool, week).await?,
        all: onboarding(pool, None).await?,
    };
    Ok(Stats { days, hours, totals, funnel, onboarding })
}

fn mask_small(value: &mut Value) {
    match value {
        Value::Object(map) => map.values_mut().for_each(mask_small),
        Value::Array(items) => items.iter_mut().for_each(mask_small),
        Value::Number(number) if number.as_i64().is_some_and(|n| n < PUBLIC_MIN) => *value = Value::Null,
        _ => {}
    }
}

fn public(stats: &Stats) -> serde_json::Result<Value> {
    let mut value = serde_json::to_value(stats)?;
    if stats.onboarding.day.timed < PUBLIC_MIN {
        value["onboarding"]["day"]["median_minutes"] = Value::Null;
    }
    mask_small(&mut value["funnel"]["day"]);
    mask_small(&mut value["onboarding"]["day"]);
    mask_small(&mut value["totals"]["logged_in_last_day"]);
    let signups = match (stats.hours.first(), stats.hours.last()) {
        (Some(first), Some(last)) => last.signed_up - first.signed_up,
        _ => 0,
    };
    let logins = stats.hours.iter().any(|hour| (1..PUBLIC_MIN).contains(&hour.logged_in));
    for hour in value["hours"].as_array_mut().into_iter().flatten() {
        if signups < PUBLIC_MIN {
            hour["signed_up"] = Value::Null;
        }
        if logins {
            hour["logged_in"] = Value::Null;
        }
    }
    Ok(value)
}

fn render(stats: &Stats) -> anyhow::Result<Built> {
    Ok(Built {
        at: Instant::now(),
        exact: serde_json::to_vec(stats)?.into(),
        public: serde_json::to_vec(&public(stats)?)?.into(),
    })
}

#[get("/stats")]
async fn get_stats(admin: Option<Admin>, pool: web::Data<PgPool>) -> Result<HttpResponse, ApiError> {
    let mut cache = CACHE.lock().await;
    let fresh = cache.built.as_ref().is_some_and(|built| built.at.elapsed() < CACHE_FOR);
    let waiting = cache.retry_at.is_some_and(|at| Instant::now() < at);
    if !fresh && !waiting {
        match build(&pool).await.and_then(|stats| render(&stats)) {
            Ok(built) => {
                cache.built = Some(built);
                cache.retry_at = None;
                cache.backoff = Duration::ZERO;
            }
            Err(e) => {
                log::error!("Building stats failed: {e:#}");
                cache.backoff = (cache.backoff * 2).clamp(RETRY_FIRST, RETRY_MAX);
                cache.retry_at = Some(Instant::now() + cache.backoff);
            }
        }
    }
    let Some(built) = &cache.built else {
        return Err(reject_with(StatusCode::SERVICE_UNAVAILABLE, "Stats are unavailable right now"));
    };
    Ok(match admin {
        Some(_) => HttpResponse::Ok()
            .content_type("application/json")
            .insert_header((header::CACHE_CONTROL, "private, no-store"))
            .body(built.exact.clone()),
        None => HttpResponse::Ok().content_type("application/json").body(built.public.clone()),
    })
}

fn visit_key(ip: IpAddr) -> IpAddr {
    match ip {
        IpAddr::V6(ip) => IpAddr::V6(Ipv6Addr::from(u128::from(ip) & !0 << 80)),
        ip => ip,
    }
}

fn allow_visit(ip: IpAddr) -> bool {
    let ip = visit_key(ip);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn visits_group_ipv6_by_48() {
        let a: IpAddr = "2001:db8:1:2::1".parse().unwrap();
        let b: IpAddr = "2001:db8:1:ffff::9".parse().unwrap();
        let c: IpAddr = "2001:db8:2::1".parse().unwrap();
        let v4: IpAddr = "198.51.100.7".parse().unwrap();
        assert_eq!(visit_key(a), visit_key(b));
        assert_ne!(visit_key(a), visit_key(c));
        assert_eq!(visit_key(v4), v4);
    }

    #[test]
    fn onboarding_milestones_exist() {
        for step in ["Edit", "Tech Tree", "Unlock token"] {
            assert!(crate::dashboard::ONBOARDING_STEPS.contains(&step), "{step}");
        }
    }

    #[test]
    fn small_counts_are_masked() {
        let mut value = serde_json::json!({ "a": 4, "b": 5, "c": 0, "d": [1, 12], "e": 2.5, "f": "x" });
        mask_small(&mut value);
        assert_eq!(
            value,
            serde_json::json!({ "a": null, "b": 5, "c": null, "d": [null, 12], "e": 2.5, "f": "x" })
        );
    }
}
