use crate::{
    accounts::session::{self, Session},
    env,
    error::{ApiError, reject, reject_with},
    projects::review,
};
use actix_web::{
    FromRequest, HttpRequest, HttpResponse,
    dev::Payload,
    get,
    http::{StatusCode, header},
    middleware::DefaultHeaders,
    post, web,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{PgConnection, PgPool};
use std::{future::Future, pin::Pin};

const PAGE: i64 = 50;
const REASON_LIMIT: usize = 500;
const TRACKING_LIMIT: usize = 200;
const MAX_BONUS_PERCENT: f64 = 1000.0;
const MAX_BONUS_MIPS: i32 = 100_000;
const MAX_MIPS: i32 = 10_000_000;

pub fn api(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/admin")
            .wrap(DefaultHeaders::new().add((header::CACHE_CONTROL, "no-store")))
            .service((get_overview, get_ships, set_bonus, get_snapshots, shadowban, get_users))
            .service((set_mips, ban, get_orders, fulfill, cancel, get_feedback, get_audit)),
    );
}

pub fn pages(cfg: &mut web::ServiceConfig) {
    cfg.service(page);
}

fn admin_ids() -> Vec<String> {
    env::var("ADMIN_HCA_IDS")
        .map(|ids| {
            ids.split(',')
                .map(str::trim)
                .filter(|id| !id.is_empty())
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

async fn lookup(req: &HttpRequest) -> anyhow::Result<Option<i32>> {
    let ids = admin_ids();
    if ids.is_empty() {
        return Ok(None);
    }
    let Some(Session { user_id, banned: false }) = session::lookup(req).await? else {
        return Ok(None);
    };
    let pool = req
        .app_data::<web::Data<PgPool>>()
        .ok_or_else(|| anyhow::anyhow!("database pool missing"))?;
    let hca_id: String = sqlx::query_scalar("SELECT hca_id FROM \"user\" WHERE id = $1")
        .bind(user_id)
        .fetch_one(pool.get_ref())
        .await?;
    Ok(ids.contains(&hca_id).then_some(user_id))
}

fn not_found() -> ApiError {
    reject_with(StatusCode::NOT_FOUND, "Not Found")
}

pub struct Admin(i32);

impl FromRequest for Admin {
    type Error = ApiError;
    type Future = Pin<Box<dyn Future<Output = Result<Self, Self::Error>>>>;

    fn from_request(req: &HttpRequest, _: &mut Payload) -> Self::Future {
        let req = req.clone();
        Box::pin(async move { lookup(&req).await?.map(Admin).ok_or_else(not_found) })
    }
}

#[get("/admin")]
async fn page(req: HttpRequest) -> HttpResponse {
    match lookup(&req).await {
        Ok(Some(_)) => crate::html(&req, "admin").await,
        Ok(None) => HttpResponse::NotFound().finish(),
        Err(e) => {
            log::error!("admin lookup failed: {e:#}");
            HttpResponse::InternalServerError().finish()
        }
    }
}

fn reason(reason: &str) -> Result<&str, ApiError> {
    let reason = reason.trim();
    if reason.is_empty() {
        return Err(reject("Add a reason"));
    }
    if reason.chars().count() > REASON_LIMIT {
        return Err(reject(format!("Reasons can be at most {REASON_LIMIT} characters")));
    }
    Ok(reason)
}

async fn record(
    conn: &mut PgConnection,
    admin_id: i32,
    action: &str,
    user_id: i32,
    summary: String,
    reason: Option<&str>,
) -> sqlx::Result<()> {
    sqlx::query(
        "INSERT INTO admin_action (admin_id, action, user_id, summary, reason) VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(admin_id)
    .bind(action)
    .bind(user_id)
    .bind(summary)
    .bind(reason)
    .execute(conn)
    .await?;
    Ok(())
}

fn like(query: &Option<String>) -> Option<String> {
    query
        .as_deref()
        .map(str::trim)
        .filter(|query| !query.is_empty())
        .map(|query| {
            let escaped = query.replace('\\', "\\\\").replace('%', "\\%").replace('_', "\\_");
            format!("%{escaped}%")
        })
}

#[derive(Deserialize)]
struct PageQuery {
    before: Option<i32>,
    status: Option<String>,
    q: Option<String>,
}

#[derive(Serialize)]
struct Overview {
    unfulfilled_orders: i64,
}

#[get("/overview")]
async fn get_overview(_: Admin, pool: web::Data<PgPool>) -> Result<HttpResponse, ApiError> {
    let unfulfilled_orders = sqlx::query_scalar(
        "SELECT count(*) FROM \"order\" WHERE deleted_at IS NULL AND NOT fulfilled",
    )
    .fetch_one(pool.get_ref())
    .await?;
    Ok(HttpResponse::Ok().json(Overview { unfulfilled_orders }))
}

#[derive(Serialize, sqlx::FromRow)]
struct Ship {
    id: i32,
    project_id: i32,
    project: String,
    repo_url: Option<String>,
    demo_url: Option<String>,
    code_url: String,
    screenshot_url: Option<String>,
    user_id: i32,
    user_name: String,
    email: Option<String>,
    tier: Option<String>,
    review_status: Option<String>,
    rejected: bool,
    hours: Option<f64>,
    tracked_hours: Option<f64>,
    mips_per_hour: f64,
    multiplier: Option<f64>,
    mips: i32,
    bonus_percent: f64,
    bonus_mips: i32,
    created_at: DateTime<Utc>,
    approved_at: Option<DateTime<Utc>>,
    #[serde(skip)]
    record_id: String,
    #[sqlx(skip)]
    airtable_url: Option<String>,
}

async fn fetch_ships(
    db: impl sqlx::PgExecutor<'_>,
    id: Option<i32>,
    query: &PageQuery,
) -> sqlx::Result<Vec<Ship>> {
    let mut ships: Vec<Ship> = sqlx::query_as(
        "SELECT s.id, s.project_id, p.name AS project, p.repo_url, p.demo_url, s.code_url, p.screenshot_url,
             u.id AS user_id, concat_ws(' ', u.first_name, u.last_name) AS user_name, u.email,
             s.tier, s.review_status, s.rejected, s.hours, s.tracked_hours, s.mips_per_hour, s.multiplier,
             s.mips, s.bonus_percent, s.bonus_mips, s.created_at, s.approved_at, s.record_id
         FROM submission s JOIN project p ON p.id = s.project_id JOIN \"user\" u ON u.id = p.user_id
         WHERE s.deleted_at IS NULL
         AND ($1::int IS NULL OR s.id = $1)
         AND ($2::int IS NULL OR s.id < $2)
         AND CASE $3::text
             WHEN 'pending' THEN NOT s.rejected AND s.review_status IS DISTINCT FROM $4
             WHEN 'approved' THEN NOT s.rejected AND s.review_status = $4
             WHEN 'rejected' THEN s.rejected
             ELSE true
         END
         ORDER BY s.id DESC LIMIT $5",
    )
    .bind(id)
    .bind(query.before)
    .bind(query.status.as_deref())
    .bind(review::APPROVED)
    .bind(PAGE)
    .fetch_all(db)
    .await?;
    let table = review::table()
        .filter(|table| table.base_id.starts_with("app") && table.table_id_or_name.starts_with("tbl"));
    for ship in &mut ships {
        ship.airtable_url = table.as_ref().map(|table| {
            format!("https://airtable.com/{}/{}/{}", table.base_id, table.table_id_or_name, ship.record_id)
        });
    }
    Ok(ships)
}

#[get("/ships")]
async fn get_ships(
    _: Admin,
    pool: web::Data<PgPool>,
    query: web::Query<PageQuery>,
) -> Result<HttpResponse, ApiError> {
    Ok(HttpResponse::Ok().json(fetch_ships(pool.get_ref(), None, &query).await?))
}

#[derive(Deserialize)]
struct BonusRequest {
    percent: f64,
    mips: i32,
    reason: String,
}

#[post("/ships/{id}/bonus")]
async fn set_bonus(
    Admin(admin_id): Admin,
    pool: web::Data<PgPool>,
    id: web::Path<i32>,
    body: web::Json<BonusRequest>,
) -> Result<HttpResponse, ApiError> {
    let id = id.into_inner();
    let reason = reason(&body.reason)?;
    if !(0.0..=MAX_BONUS_PERCENT).contains(&body.percent) {
        return Err(reject(format!("Percentage bonuses must be from 0 to {MAX_BONUS_PERCENT}")));
    }
    if !(0..=MAX_BONUS_MIPS).contains(&body.mips) {
        return Err(reject(format!("Raw bonuses must be from 0 to {MAX_BONUS_MIPS}")));
    }
    let mut tx = pool.begin().await?;
    let (project_id, project, user_id, percent, raw, before): (i32, String, i32, f64, i32, i32) =
        sqlx::query_as(
            "SELECT s.project_id, p.name, p.user_id, s.bonus_percent, s.bonus_mips, s.mips
             FROM submission s JOIN project p ON p.id = s.project_id
             WHERE s.id = $1 AND s.deleted_at IS NULL FOR UPDATE OF s",
        )
        .bind(id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| reject("Unknown ship"))?;
    sqlx::query("UPDATE submission SET bonus_percent = $2, bonus_mips = $3 WHERE id = $1")
        .bind(id)
        .bind(body.percent)
        .bind(body.mips)
        .execute(&mut *tx)
        .await?;
    review::rebalance(&mut tx, project_id).await?;
    let ship = fetch_ships(&mut *tx, Some(id), &PageQuery { before: None, status: None, q: None })
        .await?
        .pop()
        .ok_or_else(|| reject("Unknown ship"))?;
    record(
        &mut tx,
        admin_id,
        "bonus",
        user_id,
        format!(
            "Changed the bonus on ship #{id} ({project}) from +{percent}% +{raw} to +{}% +{}, Mips {before} → {}",
            body.percent, body.mips, ship.mips
        ),
        Some(reason),
    )
    .await?;
    tx.commit().await?;
    Ok(HttpResponse::Ok().json(ship))
}

#[derive(Serialize, sqlx::FromRow)]
struct Snapshot {
    id: i32,
    image: String,
    caption: String,
    author: String,
    user_id: i32,
    user_name: String,
    created_at: DateTime<Utc>,
    shadowbanned: bool,
}

const SNAPSHOT_SELECT: &str = "SELECT s.id, s.image_url AS image, s.caption, s.display_name AS author, s.user_id,
     concat_ws(' ', u.first_name, u.last_name) AS user_name, s.created_at,
     s.shadowbanned_at IS NOT NULL AS shadowbanned
     FROM snapshot s JOIN \"user\" u ON u.id = s.user_id";

#[get("/snapshots")]
async fn get_snapshots(
    _: Admin,
    pool: web::Data<PgPool>,
    query: web::Query<PageQuery>,
) -> Result<HttpResponse, ApiError> {
    let snapshots: Vec<Snapshot> = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "{SNAPSHOT_SELECT} WHERE s.deleted_at IS NULL AND ($1::int IS NULL OR s.id < $1)
         ORDER BY s.id DESC LIMIT $2"
    )))
    .bind(query.before)
    .bind(PAGE)
    .fetch_all(pool.get_ref())
    .await?;
    Ok(HttpResponse::Ok().json(snapshots))
}

#[derive(Deserialize)]
struct ShadowbanRequest {
    shadowbanned: bool,
    reason: String,
}

#[post("/snapshots/{id}/shadowban")]
async fn shadowban(
    Admin(admin_id): Admin,
    pool: web::Data<PgPool>,
    id: web::Path<i32>,
    body: web::Json<ShadowbanRequest>,
) -> Result<HttpResponse, ApiError> {
    let id = id.into_inner();
    let reason = reason(&body.reason)?;
    let mut tx = pool.begin().await?;
    let (user_id, author): (i32, String) = sqlx::query_as(
        "UPDATE snapshot SET shadowbanned_at = CASE WHEN $2 THEN COALESCE(shadowbanned_at, now()) END
         WHERE id = $1 AND deleted_at IS NULL RETURNING user_id, display_name",
    )
    .bind(id)
    .bind(body.shadowbanned)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| reject("Unknown snapshot"))?;
    let (action, verb) = if body.shadowbanned {
        ("shadowban", "Shadowbanned")
    } else {
        ("unshadowban", "Unshadowbanned")
    };
    record(&mut tx, admin_id, action, user_id, format!("{verb} snapshot #{id} by {author}"), Some(reason))
        .await?;
    let snapshot: Snapshot = sqlx::query_as(sqlx::AssertSqlSafe(format!("{SNAPSHOT_SELECT} WHERE s.id = $1")))
        .bind(id)
        .fetch_one(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(HttpResponse::Ok().json(snapshot))
}

#[derive(Serialize, sqlx::FromRow)]
struct UserRow {
    id: i32,
    hca_id: String,
    name: String,
    email: Option<String>,
    slack_id: Option<String>,
    mips: i32,
    verification_status: Option<String>,
    banned: bool,
    projects: i64,
    signup_at: DateTime<Utc>,
    last_activity_at: Option<DateTime<Utc>>,
    #[sqlx(skip)]
    admin: bool,
}

const USER_SELECT: &str = "SELECT u.id, u.hca_id, concat_ws(' ', u.first_name, u.last_name) AS name, u.email,
     u.slack_id, u.mips, u.verification_status, u.is_banned AS banned,
     (SELECT count(*) FROM project WHERE user_id = u.id AND deleted_at IS NULL) AS projects,
     u.signup_at, u.last_activity_at
     FROM \"user\" u";

fn mark_admins(mut users: Vec<UserRow>) -> Vec<UserRow> {
    let ids = admin_ids();
    for user in &mut users {
        user.admin = ids.contains(&user.hca_id);
    }
    users
}

async fn fetch_user(conn: &mut PgConnection, id: i32) -> sqlx::Result<UserRow> {
    let user = sqlx::query_as(sqlx::AssertSqlSafe(format!("{USER_SELECT} WHERE u.id = $1")))
        .bind(id)
        .fetch_one(conn)
        .await?;
    Ok(mark_admins(vec![user]).remove(0))
}

#[get("/users")]
async fn get_users(
    _: Admin,
    pool: web::Data<PgPool>,
    query: web::Query<PageQuery>,
) -> Result<HttpResponse, ApiError> {
    let users: Vec<UserRow> = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "{USER_SELECT} WHERE u.deleted_at IS NULL AND ($1::int IS NULL OR u.id < $1)
         AND ($2::text IS NULL OR concat_ws(' ', u.first_name, u.last_name, u.email, u.slack_id, u.hca_id, u.id)
             ILIKE $2)
         ORDER BY u.id DESC LIMIT $3"
    )))
    .bind(query.before)
    .bind(like(&query.q))
    .bind(PAGE)
    .fetch_all(pool.get_ref())
    .await?;
    Ok(HttpResponse::Ok().json(mark_admins(users)))
}

async fn lock_user(conn: &mut PgConnection, id: i32) -> Result<(String, i32, bool), ApiError> {
    sqlx::query_as(
        "SELECT concat_ws(' ', first_name, last_name), mips, is_banned FROM \"user\"
         WHERE id = $1 AND deleted_at IS NULL FOR UPDATE",
    )
    .bind(id)
    .fetch_optional(conn)
    .await?
    .ok_or_else(|| reject("Unknown user"))
}

fn not_self(admin_id: i32, id: i32) -> Result<(), ApiError> {
    if admin_id == id {
        return Err(reject("You can't do that to your own account"));
    }
    Ok(())
}

fn label(name: &str, id: i32) -> String {
    if name.is_empty() {
        format!("user #{id}")
    } else {
        format!("{name} (#{id})")
    }
}

#[derive(Deserialize)]
struct MipsRequest {
    mips: i32,
    reason: String,
}

#[post("/users/{id}/mips")]
async fn set_mips(
    Admin(admin_id): Admin,
    pool: web::Data<PgPool>,
    id: web::Path<i32>,
    body: web::Json<MipsRequest>,
) -> Result<HttpResponse, ApiError> {
    let id = id.into_inner();
    let reason = reason(&body.reason)?;
    if !(0..=MAX_MIPS).contains(&body.mips) {
        return Err(reject(format!("Mips must be from 0 to {MAX_MIPS}")));
    }
    not_self(admin_id, id)?;
    let mut tx = pool.begin().await?;
    let (name, before, _) = lock_user(&mut tx, id).await?;
    sqlx::query("UPDATE \"user\" SET mips = $2 WHERE id = $1")
        .bind(id)
        .bind(body.mips)
        .execute(&mut *tx)
        .await?;
    record(
        &mut tx,
        admin_id,
        "set_mips",
        id,
        format!("Set {}'s Mips from {before} to {}", label(&name, id), body.mips),
        Some(reason),
    )
    .await?;
    let user = fetch_user(&mut tx, id).await?;
    tx.commit().await?;
    Ok(HttpResponse::Ok().json(user))
}

#[derive(Deserialize)]
struct BanRequest {
    banned: bool,
    reason: String,
}

#[post("/users/{id}/ban")]
async fn ban(
    Admin(admin_id): Admin,
    pool: web::Data<PgPool>,
    id: web::Path<i32>,
    body: web::Json<BanRequest>,
) -> Result<HttpResponse, ApiError> {
    let id = id.into_inner();
    let reason = reason(&body.reason)?;
    not_self(admin_id, id)?;
    let mut tx = pool.begin().await?;
    let (name, ..) = lock_user(&mut tx, id).await?;
    sqlx::query("UPDATE \"user\" SET is_banned = $2 WHERE id = $1")
        .bind(id)
        .bind(body.banned)
        .execute(&mut *tx)
        .await?;
    let (action, verb) = if body.banned { ("ban", "Banned") } else { ("unban", "Unbanned") };
    record(&mut tx, admin_id, action, id, format!("{verb} {}", label(&name, id)), Some(reason)).await?;
    let user = fetch_user(&mut tx, id).await?;
    tx.commit().await?;
    Ok(HttpResponse::Ok().json(user))
}

#[derive(Serialize, sqlx::FromRow)]
struct Order {
    id: i32,
    user_id: i32,
    user_name: String,
    email: Option<String>,
    item_name: String,
    ordered_at: DateTime<Utc>,
    fulfilled: bool,
    fulfilled_at: Option<DateTime<Utc>>,
    tracking_number: Option<String>,
    address_first_name: Option<String>,
    address_last_name: Option<String>,
    address_line_1: Option<String>,
    address_line_2: Option<String>,
    address_city: Option<String>,
    address_state: Option<String>,
    address_postal_code: Option<String>,
    address_country: Option<String>,
    address_phone_number: Option<String>,
}

const ORDER_SELECT: &str = "SELECT o.id, o.user_id, concat_ws(' ', u.first_name, u.last_name) AS user_name, u.email,
     o.item_name, o.ordered_at, o.fulfilled, o.fulfilled_at, o.tracking_number,
     o.address_first_name, o.address_last_name, o.address_line_1, o.address_line_2, o.address_city,
     o.address_state, o.address_postal_code, o.address_country, o.address_phone_number
     FROM \"order\" o JOIN \"user\" u ON u.id = o.user_id";

#[get("/orders")]
async fn get_orders(
    _: Admin,
    pool: web::Data<PgPool>,
    query: web::Query<PageQuery>,
) -> Result<HttpResponse, ApiError> {
    let orders: Vec<Order> = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "{ORDER_SELECT} WHERE o.deleted_at IS NULL AND ($1::int IS NULL OR o.id < $1)
         AND ($2::text IS DISTINCT FROM 'unfulfilled' OR NOT o.fulfilled)
         ORDER BY o.id DESC LIMIT $3"
    )))
    .bind(query.before)
    .bind(query.status.as_deref())
    .bind(PAGE)
    .fetch_all(pool.get_ref())
    .await?;
    Ok(HttpResponse::Ok().json(orders))
}

async fn fetch_order(conn: &mut PgConnection, id: i32) -> sqlx::Result<Order> {
    sqlx::query_as(sqlx::AssertSqlSafe(format!("{ORDER_SELECT} WHERE o.id = $1")))
        .bind(id)
        .fetch_one(conn)
        .await
}

#[derive(Deserialize)]
struct FulfillRequest {
    fulfilled: bool,
    tracking_number: Option<String>,
}

#[post("/orders/{id}/fulfill")]
async fn fulfill(
    Admin(admin_id): Admin,
    pool: web::Data<PgPool>,
    id: web::Path<i32>,
    body: web::Json<FulfillRequest>,
) -> Result<HttpResponse, ApiError> {
    let id = id.into_inner();
    let tracking = body
        .tracking_number
        .as_deref()
        .map(str::trim)
        .filter(|tracking| !tracking.is_empty());
    if tracking.is_some_and(|tracking| tracking.chars().count() > TRACKING_LIMIT) {
        return Err(reject(format!("Tracking numbers can be at most {TRACKING_LIMIT} characters")));
    }
    let mut tx = pool.begin().await?;
    let (user_id, item): (i32, String) = sqlx::query_as(
        "UPDATE \"order\" SET fulfilled = $2, tracking_number = $3
         WHERE id = $1 AND deleted_at IS NULL RETURNING user_id, item_name",
    )
    .bind(id)
    .bind(body.fulfilled)
    .bind(tracking)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| reject("Unknown order"))?;
    let summary = match (body.fulfilled, tracking) {
        (true, Some(tracking)) => format!("Marked order #{id} ({item}) fulfilled with tracking {tracking}"),
        (true, None) => format!("Marked order #{id} ({item}) fulfilled"),
        (false, _) => format!("Marked order #{id} ({item}) unfulfilled"),
    };
    let action = if body.fulfilled { "fulfill" } else { "unfulfill" };
    record(&mut tx, admin_id, action, user_id, summary, None).await?;
    let order = fetch_order(&mut tx, id).await?;
    tx.commit().await?;
    Ok(HttpResponse::Ok().json(order))
}

#[derive(Deserialize)]
struct CancelRequest {
    reason: String,
}

#[post("/orders/{id}/cancel")]
async fn cancel(
    Admin(admin_id): Admin,
    pool: web::Data<PgPool>,
    id: web::Path<i32>,
    body: web::Json<CancelRequest>,
) -> Result<HttpResponse, ApiError> {
    let id = id.into_inner();
    let reason = reason(&body.reason)?;
    let mut tx = pool.begin().await?;
    let (user_id, fulfilled): (i32, bool) =
        sqlx::query_as("SELECT user_id, fulfilled FROM \"order\" WHERE id = $1 AND deleted_at IS NULL")
            .bind(id)
            .fetch_optional(&mut *tx)
            .await?
            .ok_or_else(|| reject("Unknown order"))?;
    if fulfilled {
        return Err(reject("Mark the order unfulfilled before cancelling it"));
    }
    let (name, ..) = lock_user(&mut tx, user_id).await?;
    let item: String = sqlx::query_scalar(
        "UPDATE \"order\" SET deleted_at = now()
         WHERE id = $1 AND deleted_at IS NULL AND NOT fulfilled RETURNING item_name",
    )
    .bind(id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| reject("Unknown order"))?;
    record(
        &mut tx,
        admin_id,
        "cancel_order",
        user_id,
        format!("Cancelled order #{id} ({item}) for {} and refunded the Token", label(&name, user_id)),
        Some(reason),
    )
    .await?;
    tx.commit().await?;
    Ok(HttpResponse::NoContent().finish())
}

#[derive(Serialize, sqlx::FromRow)]
struct Feedback {
    id: i32,
    user_id: i32,
    user_name: String,
    score: i16,
    source: Option<String>,
    positives: Option<String>,
    improvements: Option<String>,
    created_at: DateTime<Utc>,
}

#[get("/feedback")]
async fn get_feedback(
    _: Admin,
    pool: web::Data<PgPool>,
    query: web::Query<PageQuery>,
) -> Result<HttpResponse, ApiError> {
    let feedback: Vec<Feedback> = sqlx::query_as(
        "SELECT f.id, f.user_id, concat_ws(' ', u.first_name, u.last_name) AS user_name, f.score, f.source,
             f.positives, f.improvements, f.created_at
         FROM feedback f JOIN \"user\" u ON u.id = f.user_id
         WHERE f.deleted_at IS NULL AND ($1::int IS NULL OR f.id < $1)
         ORDER BY f.id DESC LIMIT $2",
    )
    .bind(query.before)
    .bind(PAGE)
    .fetch_all(pool.get_ref())
    .await?;
    Ok(HttpResponse::Ok().json(feedback))
}

#[derive(Serialize, sqlx::FromRow)]
struct Action {
    id: i32,
    admin_name: String,
    action: String,
    summary: String,
    reason: Option<String>,
    created_at: DateTime<Utc>,
}

#[get("/audit")]
async fn get_audit(
    _: Admin,
    pool: web::Data<PgPool>,
    query: web::Query<PageQuery>,
) -> Result<HttpResponse, ApiError> {
    let actions: Vec<Action> = sqlx::query_as(
        "SELECT a.id, concat_ws(' ', u.first_name, u.last_name) AS admin_name, a.action, a.summary, a.reason,
             a.created_at
         FROM admin_action a JOIN \"user\" u ON u.id = a.admin_id
         WHERE ($1::int IS NULL OR a.id < $1)
         ORDER BY a.id DESC LIMIT $2",
    )
    .bind(query.before)
    .bind(PAGE)
    .fetch_all(pool.get_ref())
    .await?;
    Ok(HttpResponse::Ok().json(actions))
}
