use crate::{
    accounts::{
        hackatime, profile, redirect,
        session::{self, User},
    },
    error::{ApiError, reject_with},
    projects, snapshots, tree,
};
use actix_web::{HttpRequest, HttpResponse, get, http::StatusCode, post, web};
use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::PgPool;
use std::{
    collections::HashMap,
    sync::{LazyLock, Mutex, PoisonError},
    time::{Duration, Instant},
};

const REFRESH_EVERY: Duration = Duration::from_secs(30);

static REFRESHED: LazyLock<Mutex<HashMap<i32, Instant>>> = LazyLock::new(Default::default);

pub fn api(cfg: &mut web::ServiceConfig) {
    cfg.service((get_dashboard, see_guides));
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
    guides_seen: bool,
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
             hackatime_user_id IS NOT NULL AS hackatime_linked, guides_seen
         FROM \"user\" WHERE id = $1",
    )
    .bind(user_id)
    .fetch_one(pool)
    .await?;
    user.blocked = profile::shop_blocked(user.verification_status.as_deref(), user.ysws_eligible);

    let mut hackatime = hackatime::summaries(pool, user_id, fresh).await;
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

#[post("/guides/seen")]
async fn see_guides(
    User(user_id): User,
    pool: web::Data<PgPool>,
) -> Result<HttpResponse, ApiError> {
    sqlx::query("UPDATE \"user\" SET guides_seen = true WHERE id = $1 AND NOT guides_seen")
        .bind(user_id)
        .execute(pool.get_ref())
        .await?;
    Ok(HttpResponse::NoContent().finish())
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
