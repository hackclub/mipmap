use crate::{
    accounts::{
        hackatime::{self, Totals},
        session::User,
    },
    dashboard,
    error::{ApiError, reject},
    projects::{self, Project},
    uploads,
};
use actix_multipart::form::{MultipartForm, bytes::Bytes, text::Text};
use actix_web::{HttpResponse, post, web};
use serde::Serialize;
use sqlx::{PgPool, types::Json};
use std::collections::BTreeSet;

const REQUIRED_SECONDS: i64 = 15 * 60;
const FEED_SIZE: i64 = 24;
const CAPTION_LIMIT: usize = 500;
const DISPLAY_NAME_LIMIT: usize = 40;
const ALREADY_SENT: &str = "You've already sent a snapshot today";

#[derive(Serialize, sqlx::FromRow)]
struct Snapshot {
    id: i32,
    image: String,
    caption: String,
    author: String,
}

#[derive(Serialize)]
struct Status {
    tracked_seconds: i64,
    required_seconds: i64,
    sent_today: bool,
    multiplier: f64,
}

pub fn multiplier(shared: i64) -> f64 {
    (100 + shared.clamp(0, 32)) as f64 / 100.0
}

#[derive(Serialize)]
pub struct Overview {
    everyone: Vec<Snapshot>,
    mine: Vec<Snapshot>,
    status: Status,
}

pub async fn overview(
    pool: &PgPool,
    user_id: i32,
    current: Option<&Totals>,
    projects: &[Project],
) -> sqlx::Result<Overview> {
    Ok(Overview {
        status: status(pool, user_id, current, projects).await?,
        everyone: everyone(pool).await?,
        mine: mine(pool, user_id).await?,
    })
}

async fn everyone(pool: &PgPool) -> sqlx::Result<Vec<Snapshot>> {
    sqlx::query_as(
        "SELECT snapshot.id, image_url AS image, caption, display_name AS author
         FROM snapshot JOIN \"user\" ON \"user\".id = snapshot.user_id
         WHERE snapshot.deleted_at IS NULL AND \"user\".deleted_at IS NULL AND NOT \"user\".is_banned
         ORDER BY snapshot.created_at DESC LIMIT $1",
    )
    .bind(FEED_SIZE)
    .fetch_all(pool)
    .await
}

async fn mine(pool: &PgPool, user_id: i32) -> sqlx::Result<Vec<Snapshot>> {
    sqlx::query_as(
        "SELECT id, image_url AS image, caption, display_name AS author FROM snapshot
         WHERE user_id = $1 AND deleted_at IS NULL ORDER BY created_at DESC",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
}

async fn status(
    pool: &PgPool,
    user_id: i32,
    current: Option<&Totals>,
    projects: &[Project],
) -> sqlx::Result<Status> {
    let (last, baseline, sent_today, shared): (Option<Json<Totals>>, Option<Json<Totals>>, bool, i64) =
        sqlx::query_as(
            "SELECT
                 (SELECT hackatime_totals FROM snapshot
                  WHERE user_id = $1 AND deleted_at IS NULL ORDER BY created_at DESC LIMIT 1),
                 hackatime_baseline,
                 EXISTS (SELECT 1 FROM snapshot
                         WHERE user_id = $1 AND deleted_at IS NULL
                         AND created_at > now() - interval '16 hours'),
                 (SELECT count(*) FROM snapshot WHERE user_id = $1 AND deleted_at IS NULL)
             FROM \"user\" WHERE id = $1",
        )
        .bind(user_id)
        .fetch_one(pool)
        .await?;
    let reference = last.or(baseline).map(|json| json.0).unwrap_or_default();
    let linked: BTreeSet<&str> = projects
        .iter()
        .flat_map(|project| project.hackatime.iter().map(String::as_str))
        .collect();
    let tracked_seconds = current.map_or(0, |current| {
        linked
            .iter()
            .map(|name| {
                let now = current.get(*name).copied().unwrap_or(0);
                let then = reference.get(*name).copied().unwrap_or(0);
                (now - then).max(0)
            })
            .sum()
    });
    Ok(Status {
        tracked_seconds,
        required_seconds: REQUIRED_SECONDS,
        sent_today,
        multiplier: multiplier(shared),
    })
}

#[derive(MultipartForm)]
struct SnapshotForm {
    #[multipart(limit = "10MB")]
    image: Bytes,
    caption: Text<String>,
    display_name: Text<String>,
}

#[post("/snapshots")]
pub async fn create_snapshot(
    User(user_id): User,
    pool: web::Data<PgPool>,
    MultipartForm(form): MultipartForm<SnapshotForm>,
) -> Result<HttpResponse, ApiError> {
    let caption = form.caption.trim();
    let display_name = form.display_name.trim();
    if caption.is_empty() || caption.chars().count() > CAPTION_LIMIT {
        return Err(reject(format!("Caption must be 1 to {CAPTION_LIMIT} characters")));
    }
    if display_name.is_empty() || display_name.chars().count() > DISPLAY_NAME_LIMIT {
        return Err(reject(format!(
            "Publicly displayed username must be 1 to {DISPLAY_NAME_LIMIT} characters"
        )));
    }
    uploads::ensure_image(&form.image.data)?;

    let slot = uploads::reserve(user_id)?;
    let Some(hackatime) = hackatime::summaries(&pool, user_id).await else {
        return Err(reject("Link Hackatime first"));
    };
    let totals = hackatime::totals(&hackatime);
    let projects = projects::list(&pool, user_id).await?;
    let status = status(&pool, user_id, Some(&totals), &projects).await?;
    if status.sent_today {
        return Err(reject(ALREADY_SENT));
    }
    if status.tracked_seconds < status.required_seconds {
        return Err(reject("You haven't tracked enough time yet"));
    }

    uploads::ensure_text_allowed(&[display_name, caption]).await?;
    let url = uploads::upload(&slot, &form.image.data).await?;
    let mut tx = pool.begin().await?;
    let sent_today: bool = sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM snapshot
                        WHERE user_id = u.id AND deleted_at IS NULL
                        AND created_at > now() - interval '16 hours')
         FROM \"user\" u WHERE u.id = $1 FOR UPDATE",
    )
    .bind(user_id)
    .fetch_one(&mut *tx)
    .await?;
    if sent_today {
        return Err(reject(ALREADY_SENT));
    }
    sqlx::query(
        "INSERT INTO snapshot (user_id, display_name, caption, image_url, hackatime_totals)
         VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(user_id)
    .bind(display_name)
    .bind(caption)
    .bind(url)
    .bind(Json(&totals))
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    dashboard::respond(&pool, user_id).await
}
