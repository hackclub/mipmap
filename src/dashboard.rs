use crate::{
    accounts::{
        hackatime, profile, redirect,
        session::{self, User},
    },
    error::{ApiError, reject, reject_with},
    projects, snapshots, tree,
};
use actix_web::{HttpRequest, HttpResponse, get, http::StatusCode, post, web};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::{
    collections::HashMap,
    sync::{LazyLock, Mutex, PoisonError},
    time::{Duration, Instant},
};

const REFRESH_EVERY: Duration = Duration::from_secs(30);

static REFRESHED: LazyLock<Mutex<HashMap<i32, Instant>>> = LazyLock::new(Default::default);

pub fn api(cfg: &mut web::ServiceConfig) {
    cfg.service((get_dashboard, save_onboarding_step, grant_onboarding_mips, finish_onboarding));
}

pub fn pages(cfg: &mut web::ServiceConfig) {
    cfg.service((page, tree_json, layout_json));
}

#[derive(Serialize, sqlx::FromRow)]
struct UserInfo {
    first_name: Option<String>,
    mips: i32,
    referral_code: String,
    #[sqlx(skip)]
    blocked: Option<&'static str>,
    #[serde(skip)]
    verification_status: Option<String>,
    #[serde(skip)]
    ysws_eligible: bool,
    hackatime_linked: bool,
    #[sqlx(skip)]
    hackatime_revoked: bool,
    onboarding_step: Option<String>,
    onboarding_granted: bool,
    onboarded: bool,
    saw_old_onboarding: bool,
    sticker_skipped: bool,
}

#[derive(Serialize, sqlx::FromRow)]
struct Order {
    id: i32,
    node_id: i32,
    item_name: String,
    ordered_at: DateTime<Utc>,
    fulfilled: bool,
    tracking_number: Option<String>,
}

#[derive(Serialize)]
struct HackatimeProject {
    name: String,
    seconds: i64,
}

#[derive(Serialize)]
pub struct Dashboard {
    user: UserInfo,
    unlocked: Vec<i32>,
    purchased: Vec<i32>,
    orders: Vec<Order>,
    projects: Vec<projects::Project>,
    hackatime_projects: Vec<HackatimeProject>,
    snapshots: snapshots::Overview,
}

pub async fn build(pool: &PgPool, user_id: i32, fresh: bool) -> anyhow::Result<Dashboard> {
    let mut user: UserInfo = sqlx::query_as(
        "SELECT first_name, mips, referral_code, verification_status, ysws_eligible,
             hackatime_user_id IS NOT NULL AS hackatime_linked, onboarding_step,
             onboarding_granted_at IS NOT NULL AS onboarding_granted,
             onboarded_at IS NOT NULL AS onboarded, saw_old_onboarding,
             sticker_skipped_at IS NOT NULL AS sticker_skipped
         FROM \"user\" WHERE id = $1",
    )
    .bind(user_id)
    .fetch_one(pool)
    .await?;
    user.blocked = profile::shop_blocked(user.verification_status.as_deref(), user.ysws_eligible);

    let mut hackatime = match hackatime::fetch(pool, user_id, fresh).await {
        hackatime::Summaries::Projects(projects) => Some(projects),
        hackatime::Summaries::Revoked => {
            user.hackatime_revoked = true;
            None
        }
        _ => None,
    };
    let totals = hackatime.as_deref().map(hackatime::totals);
    let projects = projects::list(pool, user_id).await?;
    if let Some(hackatime) = &hackatime {
        projects::record_time(pool, &projects, hackatime).await?;
    }
    let snapshots = snapshots::overview(pool, user_id, totals.as_ref(), &projects).await?;

    let unlocked = tree::unlocked(pool, user_id).await?;
    let orders: Vec<Order> = sqlx::query_as(
        "SELECT id, node_id, item_name, ordered_at, fulfilled, tracking_number FROM \"order\"
         WHERE user_id = $1 AND deleted_at IS NULL ORDER BY ordered_at DESC",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;

    if let Some(hackatime) = &mut hackatime {
        hackatime.sort_by(|a, b| b.most_recent_heartbeat.cmp(&a.most_recent_heartbeat));
    }
    Ok(Dashboard {
        user,
        unlocked,
        purchased: orders.iter().map(|order| order.node_id).collect(),
        orders,
        projects,
        hackatime_projects: hackatime
            .unwrap_or_default()
            .into_iter()
            .map(|project| HackatimeProject {
                name: project.name,
                seconds: project.total_duration_seconds as i64,
            })
            .collect(),
        snapshots,
    })
}

fn refresh_due(user_id: i32) -> bool {
    let mut refreshed = REFRESHED.lock().unwrap_or_else(PoisonError::into_inner);
    let now = Instant::now();
    refreshed.retain(|_, at| now.duration_since(*at) < REFRESH_EVERY);
    refreshed.insert(user_id, now).is_none()
}

pub async fn respond(pool: &PgPool, user_id: i32) -> Result<HttpResponse, ApiError> {
    Ok(HttpResponse::Ok().json(build(pool, user_id, false).await?))
}

#[get("/dashboard")]
async fn get_dashboard(
    User(user_id): User,
    pool: web::Data<PgPool>,
) -> Result<HttpResponse, ApiError> {
    let fresh = refresh_due(user_id);
    if fresh && let Err(e) = profile::refresh(&pool, user_id).await {
        let expired = e
            .downcast_ref::<reqwest::Error>()
            .and_then(reqwest::Error::status)
            == Some(reqwest::StatusCode::UNAUTHORIZED);
        if expired {
            sqlx::query("DELETE FROM session WHERE user_id = $1")
                .bind(user_id)
                .execute(pool.get_ref())
                .await?;
            return Err(reject_with(StatusCode::UNAUTHORIZED, "Please sign in again"));
        }
        log::warn!("refreshing HCA profile failed: {e:#}");
    }
    Ok(HttpResponse::Ok().json(build(&pool, user_id, fresh).await?))
}

#[derive(Deserialize)]
struct OnboardingStep {
    step: String,
}

#[post("/onboarding/step")]
async fn save_onboarding_step(
    User(user_id): User,
    pool: web::Data<PgPool>,
    body: web::Json<OnboardingStep>,
) -> Result<HttpResponse, ApiError> {
    if body.step.is_empty() || body.step.len() > 64 {
        return Err(reject("Invalid onboarding step"));
    }
    sqlx::query(
        "UPDATE \"user\" SET onboarding_step = $2,
             onboarding_started_at = COALESCE(onboarding_started_at, now())
         WHERE id = $1 AND onboarded_at IS NULL",
    )
    .bind(user_id)
    .bind(&body.step)
    .execute(pool.get_ref())
    .await?;
    Ok(HttpResponse::NoContent().finish())
}

#[post("/onboarding/grant")]
async fn grant_onboarding_mips(
    User(user_id): User,
    pool: web::Data<PgPool>,
) -> Result<HttpResponse, ApiError> {
    sqlx::query(
        "UPDATE \"user\" SET mips = mips + $2, onboarding_granted_at = now()
         WHERE id = $1 AND onboarding_granted_at IS NULL AND onboarded_at IS NULL",
    )
    .bind(user_id)
    .bind(tree::tutorial_mips())
    .execute(pool.get_ref())
    .await?;
    respond(&pool, user_id).await
}

#[post("/onboarding/finish")]
async fn finish_onboarding(
    User(user_id): User,
    pool: web::Data<PgPool>,
) -> Result<HttpResponse, ApiError> {
    let (done, onboarded): (bool, bool) = sqlx::query_as(
        "SELECT (SELECT count(*) FROM unlock
                 WHERE user_id = $1 AND node_id = ANY($2) AND deleted_at IS NULL) = 2
             AND (sticker_skipped_at IS NOT NULL
                  OR EXISTS (SELECT 1 FROM \"order\"
                             WHERE user_id = $1 AND node_id = $3 AND deleted_at IS NULL)),
             onboarded_at IS NOT NULL
         FROM \"user\" WHERE id = $1",
    )
    .bind(user_id)
    .bind([tree::STICKER as i32, tree::STICKER_TOKEN as i32])
    .bind(tree::STICKER as i32)
    .fetch_one(pool.get_ref())
    .await?;
    if !done && !onboarded {
        return Err(reject("Redeem the Mipmap sticker to finish the tutorial"));
    }
    sqlx::query("UPDATE \"user\" SET onboarded_at = now() WHERE id = $1 AND onboarded_at IS NULL")
        .bind(user_id)
        .execute(pool.get_ref())
        .await?;
    respond(&pool, user_id).await
}

#[get("/dashboard")]
async fn page(req: HttpRequest) -> HttpResponse {
    match session::lookup_page(&req).await {
        Ok(Some(_)) => {}
        Ok(None) => return redirect("/login"),
        Err(response) => return *response,
    }
    crate::html(&req, "dashboard").await
}

fn json(body: &'static str) -> HttpResponse {
    HttpResponse::Ok().content_type("application/json").body(body)
}

#[get("/dashboard/tree.json")]
async fn tree_json() -> HttpResponse {
    json(tree::TREE_JSON)
}

#[get("/dashboard/layout.json")]
async fn layout_json() -> HttpResponse {
    json(tree::LAYOUT_JSON.as_str())
}
