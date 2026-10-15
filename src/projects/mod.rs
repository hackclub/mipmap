use crate::{
    accounts::session::User,
    dashboard,
    error::{ApiError, reject, reject_with},
    uploads,
};
use actix_web::{HttpResponse, delete, http::StatusCode, post, put, web};
use chrono::{DateTime, Utc};
use hc_helper::hackatime::data::Project as HackatimeProject;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

pub mod review;
mod ship;

const PROJECT_LIMIT: i64 = 50;
const NAME_LIMIT: usize = 100;
const DESCRIPTION_LIMIT: usize = 5000;
const LINK_LIMIT: usize = 500;
const HACKATIME_LIMIT: usize = 50;

pub fn routes(cfg: &mut web::ServiceConfig) {
    cfg.service((
        create_project,
        upload_screenshot,
        update_project,
        delete_project,
        ship::ship_project,
    ));
}

#[derive(Deserialize, Clone, Copy)]
#[serde(rename_all = "lowercase")]
enum Tier {
    Quebec,
    Romeo,
    Sierra,
}

impl Tier {
    fn name(self) -> &'static str {
        match self {
            Tier::Quebec => "quebec",
            Tier::Romeo => "romeo",
            Tier::Sierra => "sierra",
        }
    }
}

#[derive(Serialize, sqlx::FromRow)]
pub struct Project {
    id: i32,
    name: String,
    description: String,
    repo: String,
    demo: String,
    tier: Option<String>,
    pub hackatime: Vec<String>,
    screenshot: Option<String>,
    status: String,
    locked: bool,
    shipped_at: Option<DateTime<Utc>>,
    mips_awarded: i32,
    #[serde(skip)]
    total_seconds: i64,
    #[serde(skip)]
    last_logged_at: Option<DateTime<Utc>>,
}

#[derive(Deserialize)]
struct ProjectFields {
    name: String,
    description: String,
    repo: String,
    demo: String,
    tier: Option<Tier>,
    hackatime: Vec<String>,
}

#[derive(sqlx::FromRow)]
struct Reviewed {
    description: String,
    repo: String,
    demo: String,
    tier: Option<String>,
    hackatime: Vec<String>,
}

#[derive(Deserialize)]
struct CreateRequest {
    name: String,
}

fn unknown_project() -> ApiError {
    reject_with(StatusCode::NOT_FOUND, "unknown project")
}

fn limited<'a>(label: &str, value: &'a str, limit: usize) -> Result<&'a str, ApiError> {
    let value = value.trim();
    if value.chars().count() > limit {
        return Err(reject(format!("{label} must be at most {limit} characters")));
    }
    Ok(value)
}

fn required_name(name: &str) -> Result<&str, ApiError> {
    match limited("Name", name, NAME_LIMIT)? {
        "" => Err(reject("Make sure the name isn't empty!")),
        name => Ok(name),
    }
}

pub async fn list(pool: &PgPool, user_id: i32) -> sqlx::Result<Vec<Project>> {
    sqlx::query_as(
        "SELECT p.id, p.name, p.description, COALESCE(p.repo_url, '') AS repo,
             COALESCE(p.demo_url, '') AS demo, p.tier, p.hackatime_projects AS hackatime,
             p.screenshot_url AS screenshot,
             CASE WHEN latest.created_at IS NULL THEN 'draft'
                  WHEN latest.rejected THEN 'rejected'
                  WHEN latest.review_status = $2 THEN 'approved'
                  ELSE 'shipped' END AS status,
             EXISTS (SELECT 1 FROM submission WHERE project_id = p.id AND deleted_at IS NULL AND NOT rejected)
                 AS locked,
             latest.created_at AS shipped_at,
             (SELECT COALESCE(sum(mips), 0)::int FROM submission WHERE project_id = p.id)
                 AS mips_awarded,
             p.total_seconds, p.last_logged_at
         FROM project p
         LEFT JOIN LATERAL (
             SELECT review_status, rejected, created_at FROM submission
             WHERE project_id = p.id AND deleted_at IS NULL
             ORDER BY created_at DESC, id DESC LIMIT 1
         ) latest ON true
         WHERE p.user_id = $1 AND p.deleted_at IS NULL ORDER BY p.id",
    )
    .bind(user_id)
    .bind(review::APPROVED)
    .fetch_all(pool)
    .await
}

async fn ensure_owned(pool: &PgPool, user_id: i32, project_id: i32) -> Result<(), ApiError> {
    let owned: bool = sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM project WHERE id = $1 AND user_id = $2 AND deleted_at IS NULL)",
    )
    .bind(project_id)
    .bind(user_id)
    .fetch_one(pool)
    .await?;
    if !owned {
        return Err(unknown_project());
    }
    Ok(())
}

async fn reviewing(pool: &PgPool, project_id: i32) -> sqlx::Result<bool> {
    sqlx::query_scalar(
        "SELECT COALESCE((SELECT NOT rejected AND review_status IS DISTINCT FROM $2 FROM submission
                          WHERE project_id = $1 AND deleted_at IS NULL
                          ORDER BY created_at DESC, id DESC LIMIT 1), false)",
    )
    .bind(project_id)
    .bind(review::APPROVED)
    .fetch_one(pool)
    .await
}

async fn ensure_hackatime_free(
    pool: &PgPool,
    user_id: i32,
    project_id: i32,
    names: &[String],
) -> Result<(), ApiError> {
    let taken: Option<String> = sqlx::query_scalar(
        "SELECT linked.name FROM project p, unnest(p.hackatime_projects) AS linked (name)
         WHERE p.user_id = $1 AND p.id <> $2 AND p.deleted_at IS NULL AND linked.name = ANY($3)
         LIMIT 1",
    )
    .bind(user_id)
    .bind(project_id)
    .bind(names)
    .fetch_optional(pool)
    .await?;
    match taken {
        Some(name) => Err(reject(format!("{name} is already linked to another of your projects"))),
        None => Ok(()),
    }
}

pub async fn record_time(
    pool: &PgPool,
    projects: &[Project],
    hackatime: &[HackatimeProject],
) -> sqlx::Result<()> {
    for project in projects {
        let linked = hackatime
            .iter()
            .filter(|entry| project.hackatime.contains(&entry.name));
        let seconds: i64 = linked
            .clone()
            .map(|entry| entry.total_duration_seconds as i64)
            .sum();
        let last = linked
            .filter_map(|entry| entry.most_recent_heartbeat.as_deref())
            .filter_map(|time| DateTime::parse_from_rfc3339(time).ok())
            .map(|time| time.with_timezone(&Utc))
            .max();
        if seconds == project.total_seconds && last == project.last_logged_at {
            continue;
        }
        sqlx::query(
            "WITH updated AS (
                 UPDATE project SET total_seconds = $2, last_logged_at = $3 WHERE id = $1
                 RETURNING user_id
             )
             INSERT INTO active_day (user_id, day)
             SELECT \"user\".id, ($3 AT TIME ZONE 'UTC')::date FROM updated
             JOIN \"user\" ON \"user\".id = updated.user_id
             WHERE ($3 AT TIME ZONE 'UTC')::date >= (\"user\".signup_at AT TIME ZONE 'UTC')::date
             ON CONFLICT DO NOTHING",
        )
        .bind(project.id)
        .bind(seconds)
        .bind(last)
        .execute(pool)
        .await?;
    }
    Ok(())
}

#[post("/projects")]
async fn create_project(
    User(user_id): User,
    pool: web::Data<PgPool>,
    body: web::Json<CreateRequest>,
) -> Result<HttpResponse, ApiError> {
    let name = required_name(&body.name)?;
    let mut tx = pool.begin().await?;
    sqlx::query("SELECT 1 FROM \"user\" WHERE id = $1 FOR UPDATE")
        .bind(user_id)
        .execute(&mut *tx)
        .await?;
    let inserted = sqlx::query(
        "INSERT INTO project (user_id, name)
         SELECT $1, $2
         WHERE (SELECT count(*) FROM project WHERE user_id = $1 AND deleted_at IS NULL) < $3",
    )
    .bind(user_id)
    .bind(name)
    .bind(PROJECT_LIMIT)
    .execute(&mut *tx)
    .await?;
    if inserted.rows_affected() == 0 {
        return Err(reject(format!("You can have at most {PROJECT_LIMIT} projects")));
    }
    tx.commit().await?;
    dashboard::respond(&pool, user_id).await
}

#[put("/projects/{id}")]
async fn update_project(
    User(user_id): User,
    pool: web::Data<PgPool>,
    id: web::Path<i32>,
    body: web::Json<ProjectFields>,
) -> Result<HttpResponse, ApiError> {
    ensure_owned(&pool, user_id, *id).await?;
    let _shipping = ship::Shipping::start(*id)
        .ok_or_else(|| reject_with(StatusCode::CONFLICT, "This project is being shipped"))?;
    let fields = body.into_inner();
    let name = required_name(&fields.name)?;
    let description = limited("Description", &fields.description, DESCRIPTION_LIMIT)?;
    let repo = limited("Repo link", &fields.repo, LINK_LIMIT)?;
    let demo = limited("Demo link", &fields.demo, LINK_LIMIT)?;
    let mut hackatime = fields.hackatime;
    hackatime.sort();
    hackatime.dedup();
    if hackatime.len() > HACKATIME_LIMIT {
        return Err(reject(format!("Link at most {HACKATIME_LIMIT} Hackatime projects")));
    }
    if hackatime.iter().any(|name| name.chars().count() > LINK_LIMIT) {
        return Err(reject("Hackatime project name is too long"));
    }
    let mut current: Reviewed = sqlx::query_as(
        "SELECT description, COALESCE(repo_url, '') AS repo, COALESCE(demo_url, '') AS demo, tier,
             hackatime_projects AS hackatime
         FROM project WHERE id = $1 AND user_id = $2 AND deleted_at IS NULL",
    )
    .bind(*id)
    .bind(user_id)
    .fetch_optional(pool.get_ref())
    .await?
    .ok_or_else(unknown_project)?;
    current.hackatime.sort();
    current.hackatime.dedup();
    let unchanged = current.description == description
        && current.repo == repo
        && current.demo == demo
        && current.tier.as_deref() == fields.tier.map(Tier::name)
        && current.hackatime == hackatime;
    if !unchanged && reviewing(&pool, *id).await? {
        return Err(reject_with(
            StatusCode::CONFLICT,
            "This project is being reviewed, so only its name can change",
        ));
    }
    ensure_hackatime_free(&pool, user_id, *id, &hackatime).await?;
    let shipped: bool = sqlx::query_scalar(
        "SELECT EXISTS (
             SELECT 1 FROM submission
             WHERE project_id = $1 AND deleted_at IS NULL AND NOT rejected
         )",
    )
    .bind(*id)
    .fetch_one(pool.get_ref())
    .await?;
    if shipped {
        if current.repo != repo {
            return Err(reject("A shipped project's repo link can't change"));
        }
        if current.tier.as_deref() != fields.tier.map(Tier::name) {
            return Err(reject("A shipped project's tier can't change"));
        }
        if let Some(name) = current.hackatime.iter().find(|name| !hackatime.contains(name)) {
            return Err(reject(format!("{name} can't be unlinked from a shipped project")));
        }
    }
    let updated = sqlx::query(
        "UPDATE project SET name = $3, description = $4, repo_url = NULLIF($5, ''),
             demo_url = NULLIF($6, ''), tier = $7, hackatime_projects = $8
         WHERE id = $1 AND user_id = $2 AND deleted_at IS NULL",
    )
    .bind(*id)
    .bind(user_id)
    .bind(name)
    .bind(description)
    .bind(repo)
    .bind(demo)
    .bind(fields.tier.map(Tier::name))
    .bind(hackatime)
    .execute(pool.get_ref())
    .await?;
    if updated.rows_affected() == 0 {
        return Err(unknown_project());
    }
    dashboard::respond(&pool, user_id).await
}

#[delete("/projects/{id}")]
async fn delete_project(
    User(user_id): User,
    pool: web::Data<PgPool>,
    id: web::Path<i32>,
) -> Result<HttpResponse, ApiError> {
    ensure_owned(&pool, user_id, *id).await?;
    let _shipping = ship::Shipping::start(*id)
        .ok_or_else(|| reject_with(StatusCode::CONFLICT, "This project is being shipped"))?;
    let shipped: bool = sqlx::query_scalar(
        "SELECT EXISTS (
             SELECT 1 FROM submission
             WHERE project_id = $1 AND deleted_at IS NULL AND NOT rejected
         )",
    )
    .bind(*id)
    .fetch_one(pool.get_ref())
    .await?;
    if shipped {
        return Err(reject_with(StatusCode::CONFLICT, "A shipped project can't be deleted"));
    }
    let deleted = sqlx::query(
        "UPDATE project SET deleted_at = now()
         WHERE id = $1 AND user_id = $2 AND deleted_at IS NULL",
    )
    .bind(*id)
    .bind(user_id)
    .execute(pool.get_ref())
    .await?;
    if deleted.rows_affected() == 0 {
        return Err(unknown_project());
    }
    dashboard::respond(&pool, user_id).await
}

#[post("/projects/{id}/screenshot")]
async fn upload_screenshot(
    User(user_id): User,
    pool: web::Data<PgPool>,
    id: web::Path<i32>,
    body: web::Bytes,
) -> Result<HttpResponse, ApiError> {
    uploads::ensure_image(&body)?;
    ensure_owned(&pool, user_id, *id).await?;
    let _shipping = ship::Shipping::start(*id)
        .ok_or_else(|| reject_with(StatusCode::CONFLICT, "This project is being shipped"))?;
    if reviewing(&pool, *id).await? {
        return Err(reject_with(
            StatusCode::CONFLICT,
            "This project is being reviewed, so its screenshot can't change",
        ));
    }
    let slot = uploads::reserve(user_id)?;
    let url = uploads::upload(&slot, &body).await?;
    sqlx::query("UPDATE project SET screenshot_url = $2 WHERE id = $1")
        .bind(*id)
        .bind(url)
        .execute(pool.get_ref())
        .await?;
    dashboard::respond(&pool, user_id).await
}
