use super::{RECIPIENTS, SendError, TEXT_LIMIT, UNCONFIRMED, mrkdwn};
use crate::{
    admin::{Admin, admin_ids},
    error::{ApiError, reject, reject_with},
};
use actix_web::{HttpResponse, get, http::StatusCode, post, web};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{PgConnection, PgExecutor, PgPool};

const PAGE: i64 = 50;
const THREADS: i64 = 100;
const THREAD_MESSAGES: i64 = 200;
const USER_FOR_SLACK_ID: &str = "(SELECT id FROM \"user\" WHERE slack_id = $1 ORDER BY deleted_at IS NOT NULL, id LIMIT 1)";

pub fn routes(cfg: &mut web::ServiceConfig) {
    cfg.service((get_log, create_announcement, get_drafts, send_draft, dismiss_draft, refresh_draft, send_direct))
        .service((get_threads, get_thread, reply, get_admins, get_templates));
}

fn text(text: &str) -> Result<&str, ApiError> {
    let text = text.trim();
    if text.is_empty() {
        return Err(reject("Write a message first"));
    }
    if mrkdwn(text).chars().count() > TEXT_LIMIT {
        return Err(reject(format!("Messages can be at most {TEXT_LIMIT} characters")));
    }
    Ok(text)
}

fn slack_id(slack_id: &str) -> Result<&str, ApiError> {
    let slack_id = slack_id.trim();
    let valid = slack_id.len() > 2
        && slack_id.starts_with(['U', 'W'])
        && slack_id.bytes().all(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit());
    if !valid {
        return Err(reject("That isn't a Slack user ID, they look like U0123ABCD"));
    }
    Ok(slack_id)
}

fn dedupe_key(key: &str) -> Result<String, ApiError> {
    let valid = (1..=64).contains(&key.len()) && key.bytes().all(|byte| byte.is_ascii_alphanumeric() || byte == b'-');
    if !valid {
        return Err(reject("Reload the page and try again"));
    }
    Ok(format!("admin:{key}"))
}

fn configured() -> Result<(), ApiError> {
    super::token()
        .map(|_| ())
        .ok_or_else(|| reject("Slack isn't set up, SLACK_BOT_TOKEN is missing"))
}

fn send_error(error: SendError) -> ApiError {
    match error {
        SendError::RateLimited(secs) => reject_with(
            StatusCode::TOO_MANY_REQUESTS,
            format!("Slack is rate limiting the bot, try again in {secs} seconds"),
        ),
        SendError::Rejected(error) => reject(format!("Slack refused the message: {error}")),
        SendError::Unavailable(error) => {
            log::error!("Slack is unavailable: {error}");
            reject(format!("Slack isn't available: {error}"))
        }
        SendError::Down(error) | SendError::Failed(error) => {
            log::warn!("Sending a Slack message failed: {error:#}");
            reject("Slack had a temporary problem, try again in a minute")
        }
        SendError::Unconfirmed(error) => {
            log::warn!("Slack didn't confirm a message: {error:#}");
            reject("Slack didn't confirm the message, so it may have been sent. Check Slack before sending it again")
        }
    }
}

async fn audit(
    db: impl PgExecutor<'_>,
    admin_id: i32,
    action: &str,
    user_id: Option<i32>,
    summary: String,
) -> sqlx::Result<()> {
    sqlx::query("INSERT INTO admin_action (admin_id, action, user_id, summary) VALUES ($1, $2, $3, $4)")
        .bind(admin_id)
        .bind(action)
        .bind(user_id)
        .bind(summary)
        .execute(db)
        .await?;
    Ok(())
}

async fn send_now(
    pool: &PgPool,
    admin_id: i32,
    slack_id: &str,
    kind: &str,
    text: &str,
    key: &str,
) -> Result<(), ApiError> {
    let key = dedupe_key(key)?;
    let claimed: Option<(i32, Option<i32>)> = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "INSERT INTO slack_message (slack_id, user_id, kind, text, admin_id, dedupe_key, sending_at)
         VALUES ($1, {USER_FOR_SLACK_ID}, $2, $3, $4, $5, now())
         ON CONFLICT (dedupe_key) DO NOTHING RETURNING id, user_id"
    )))
    .bind(slack_id)
    .bind(kind)
    .bind(text)
    .bind(admin_id)
    .bind(&key)
    .fetch_optional(pool)
    .await?;
    let Some((id, user_id)) = claimed else {
        let (sent, failed): (bool, bool) = sqlx::query_as(
            "SELECT sent_at IS NOT NULL, failed_at IS NOT NULL FROM slack_message WHERE dedupe_key = $1",
        )
        .bind(&key)
        .fetch_one(pool)
        .await?;
        return match (sent, failed) {
            (true, _) => Ok(()),
            (_, true) => Err(reject(
                "This message may have been sent already. Check Slack, then change the message to send it again",
            )),
            _ => Err(reject_with(StatusCode::CONFLICT, "This message is still sending")),
        };
    };
    match super::post(slack_id, text).await {
        Ok(ts) => {
            let mut tx = pool.begin().await?;
            sqlx::query("UPDATE slack_message SET sent_at = now(), ts = $2 WHERE id = $1")
                .bind(id)
                .bind(ts)
                .execute(&mut *tx)
                .await?;
            let summary = match kind {
                "reply" => format!("Replied to {slack_id} on Slack"),
                _ => format!("Sent a Slack DM to {slack_id}"),
            };
            audit(&mut *tx, admin_id, &format!("slack_{kind}"), user_id, summary).await?;
            tx.commit().await?;
            Ok(())
        }
        Err(error @ SendError::Unconfirmed(_)) => {
            sqlx::query("UPDATE slack_message SET failed_at = now(), error = $2 WHERE id = $1")
                .bind(id)
                .bind(UNCONFIRMED)
                .execute(pool)
                .await?;
            Err(send_error(error))
        }
        Err(error) => {
            sqlx::query("DELETE FROM slack_message WHERE id = $1")
                .bind(id)
                .execute(pool)
                .await?;
            Err(send_error(error))
        }
    }
}

#[derive(Serialize, sqlx::FromRow)]
struct Entry {
    id: i32,
    kind: String,
    incoming: bool,
    text: String,
    at: DateTime<Utc>,
    slack_id: Option<String>,
    user_name: Option<String>,
    admin_name: Option<String>,
    recipients: i64,
    sent: i64,
    failed: i64,
    skipped: i64,
    pending: i64,
    error: Option<String>,
}

#[derive(Deserialize)]
struct LogQuery {
    before: Option<i32>,
}

#[get("/slack/log")]
async fn get_log(_: Admin, pool: web::Data<PgPool>, query: web::Query<LogQuery>) -> Result<HttpResponse, ApiError> {
    let entries: Vec<Entry> = sqlx::query_as(
        "SELECT e.*, t.text, NULLIF(concat_ws(' ', u.first_name, u.last_name), '') AS user_name,
             NULLIF(concat_ws(' ', a.first_name, a.last_name), '') AS admin_name
         FROM (
             SELECT min(m.id) AS id, min(m.kind) AS kind, bool_or(m.incoming) AS incoming, min(m.created_at) AS at,
                 CASE WHEN count(*) = 1 THEN min(m.slack_id) END AS slack_id,
                 CASE WHEN count(*) = 1 THEN min(m.user_id) END AS user_id,
                 min(m.admin_id) AS admin_id, count(*) AS recipients,
                 count(*) FILTER (WHERE m.sent_at IS NOT NULL) AS sent,
                 count(*) FILTER (WHERE m.failed_at IS NOT NULL) AS failed,
                 count(*) FILTER (WHERE m.skipped_at IS NOT NULL) AS skipped,
                 count(*) FILTER (WHERE NOT m.incoming AND m.sent_at IS NULL AND m.failed_at IS NULL
                     AND m.skipped_at IS NULL) AS pending,
                 CASE WHEN count(*) = 1 THEN min(m.error) END AS error
             FROM slack_message m
             GROUP BY COALESCE('a' || m.announcement_id, 'm' || m.id)
             HAVING $1::int IS NULL OR min(m.id) < $1
             ORDER BY min(m.id) DESC LIMIT $2
         ) e
         JOIN slack_message t ON t.id = e.id
         LEFT JOIN \"user\" u ON u.id = e.user_id
         LEFT JOIN \"user\" a ON a.id = e.admin_id
         ORDER BY e.id DESC",
    )
    .bind(query.before)
    .bind(PAGE)
    .fetch_all(pool.get_ref())
    .await?;
    Ok(HttpResponse::Ok().json(entries))
}

fn people(count: u64) -> String {
    format!("{count} {}", if count == 1 { "person" } else { "people" })
}

async fn broadcast(conn: &mut PgConnection, admin_id: i32, id: i32, kind: &str, text: &str) -> sqlx::Result<u64> {
    let column = match kind {
        "challenge" => "challenges",
        "round" => "fulfillment_rounds",
        _ => "announcements",
    };
    Ok(sqlx::query(sqlx::AssertSqlSafe(format!(
        "INSERT INTO slack_message (slack_id, user_id, kind, text, dedupe_key, announcement_id, admin_id)
         SELECT u.slack_id, u.id, $2, $3, 'announcement:' || $1 || ':' || u.slack_id, $1, $4
         FROM \"user\" u JOIN slack_preference p ON p.user_id = u.id
         WHERE {RECIPIENTS} AND p.{column}
         ORDER BY u.id
         ON CONFLICT (dedupe_key) DO NOTHING"
    )))
    .bind(id)
    .bind(kind)
    .bind(text)
    .bind(admin_id)
    .execute(conn)
    .await?
    .rows_affected())
}

#[derive(Deserialize)]
struct AnnouncementRequest {
    text: String,
    key: String,
}

#[post("/slack/announcements")]
async fn create_announcement(
    Admin(admin_id): Admin,
    pool: web::Data<PgPool>,
    body: web::Json<AnnouncementRequest>,
) -> Result<HttpResponse, ApiError> {
    configured()?;
    let text = text(&body.text)?;
    let key = dedupe_key(&body.key)?;
    let mut tx = pool.begin().await?;
    let Some(id): Option<i32> = sqlx::query_scalar(
        "INSERT INTO slack_announcement (kind, text, admin_id, sent_at, dedupe_key)
         VALUES ('announcement', $1, $2, now(), $3)
         ON CONFLICT (dedupe_key) DO NOTHING RETURNING id",
    )
    .bind(text)
    .bind(admin_id)
    .bind(key)
    .fetch_optional(&mut *tx)
    .await?
    else {
        return Ok(HttpResponse::NoContent().finish());
    };
    let recipients = broadcast(&mut tx, admin_id, id, "announcement", text).await?;
    let summary = format!("Sent Slack announcement #{id} to {}", people(recipients));
    audit(&mut *tx, admin_id, "slack_announcement", None, summary).await?;
    tx.commit().await?;
    Ok(HttpResponse::NoContent().finish())
}

#[derive(Serialize, sqlx::FromRow)]
struct Draft {
    id: i32,
    kind: String,
    text: String,
    created_at: DateTime<Utc>,
}

async fn drafts(db: impl PgExecutor<'_>) -> sqlx::Result<Vec<Draft>> {
    sqlx::query_as(
        "SELECT id, kind, text, created_at FROM slack_announcement
         WHERE sent_at IS NULL AND dismissed_at IS NULL ORDER BY id",
    )
    .fetch_all(db)
    .await
}

#[get("/slack/drafts")]
async fn get_drafts(_: Admin, pool: web::Data<PgPool>) -> Result<HttpResponse, ApiError> {
    Ok(HttpResponse::Ok().json(drafts(pool.get_ref()).await?))
}

async fn claim_draft(conn: &mut PgConnection, admin_id: i32, id: i32, column: &str) -> Result<Draft, ApiError> {
    sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "UPDATE slack_announcement SET {column} = now(), admin_id = $2
         WHERE id = $1 AND sent_at IS NULL AND dismissed_at IS NULL
         RETURNING id, kind, text, created_at"
    )))
    .bind(id)
    .bind(admin_id)
    .fetch_optional(conn)
    .await?
    .ok_or_else(|| reject("That message was already sent or skipped"))
}

#[post("/slack/drafts/{id}/send")]
async fn send_draft(
    Admin(admin_id): Admin,
    pool: web::Data<PgPool>,
    id: web::Path<i32>,
) -> Result<HttpResponse, ApiError> {
    configured()?;
    let mut tx = pool.begin().await?;
    let draft = claim_draft(&mut tx, admin_id, id.into_inner(), "sent_at").await?;
    let recipients = broadcast(&mut tx, admin_id, draft.id, &draft.kind, &draft.text).await?;
    let summary = format!("Sent {} Slack message #{} to {}", draft.kind, draft.id, people(recipients));
    audit(&mut *tx, admin_id, "slack_announcement", None, summary).await?;
    tx.commit().await?;
    Ok(HttpResponse::Ok().json(drafts(pool.get_ref()).await?))
}

#[post("/slack/drafts/{id}/dismiss")]
async fn dismiss_draft(
    Admin(admin_id): Admin,
    pool: web::Data<PgPool>,
    id: web::Path<i32>,
) -> Result<HttpResponse, ApiError> {
    let mut tx = pool.begin().await?;
    let draft = claim_draft(&mut tx, admin_id, id.into_inner(), "dismissed_at").await?;
    let summary = format!("Didn't send {} Slack message #{}", draft.kind, draft.id);
    audit(&mut *tx, admin_id, "slack_announcement", None, summary).await?;
    tx.commit().await?;
    Ok(HttpResponse::Ok().json(drafts(pool.get_ref()).await?))
}

#[post("/slack/drafts/{id}/refresh")]
async fn refresh_draft(
    Admin(admin_id): Admin,
    pool: web::Data<PgPool>,
    id: web::Path<i32>,
) -> Result<HttpResponse, ApiError> {
    let id = id.into_inner();
    let mut tx = pool.begin().await?;
    let (kind, key, old): (String, Option<String>, String) = sqlx::query_as(
        "SELECT kind, dedupe_key, text FROM slack_announcement
         WHERE id = $1 AND sent_at IS NULL AND dismissed_at IS NULL FOR UPDATE",
    )
    .bind(id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| reject("That message was already sent or skipped"))?;
    let text = key
        .as_deref()
        .and_then(super::notify::current_text)
        .ok_or_else(|| reject("The challenge or round this message is about is gone, so it can't be refreshed"))?;
    if text != old {
        sqlx::query("UPDATE slack_announcement SET text = $2 WHERE id = $1")
            .bind(id)
            .bind(&text)
            .execute(&mut *tx)
            .await?;
        let summary = format!("Refreshed the wording of {kind} Slack message #{id}");
        audit(&mut *tx, admin_id, "slack_announcement", None, summary).await?;
    }
    tx.commit().await?;
    Ok(HttpResponse::Ok().json(drafts(pool.get_ref()).await?))
}

#[derive(Deserialize)]
struct DirectRequest {
    slack_id: String,
    text: String,
    key: String,
}

#[post("/slack/direct")]
async fn send_direct(
    Admin(admin_id): Admin,
    pool: web::Data<PgPool>,
    body: web::Json<DirectRequest>,
) -> Result<HttpResponse, ApiError> {
    configured()?;
    let slack_id = slack_id(&body.slack_id)?;
    send_now(&pool, admin_id, slack_id, "direct", text(&body.text)?, &body.key).await?;
    Ok(HttpResponse::NoContent().finish())
}

#[derive(Serialize, sqlx::FromRow)]
struct AdminContact {
    slack_id: String,
    name: String,
}

#[get("/slack/admins")]
async fn get_admins(_: Admin, pool: web::Data<PgPool>) -> Result<HttpResponse, ApiError> {
    let admins: Vec<AdminContact> = sqlx::query_as(
        "SELECT slack_id, COALESCE(NULLIF(concat_ws(' ', first_name, last_name), ''), slack_id) AS name
         FROM \"user\" WHERE hca_id = ANY($1) AND slack_id IS NOT NULL AND deleted_at IS NULL
         ORDER BY name",
    )
    .bind(admin_ids())
    .fetch_all(pool.get_ref())
    .await?;
    Ok(HttpResponse::Ok().json(admins))
}

#[derive(Serialize)]
struct Template {
    label: &'static str,
    text: String,
}

#[get("/slack/templates")]
async fn get_templates(_: Admin) -> HttpResponse {
    let templates: Vec<Template> = super::notify::templates()
        .into_iter()
        .map(|(label, text)| Template { label, text })
        .collect();
    HttpResponse::Ok().json(templates)
}

#[derive(Serialize, sqlx::FromRow)]
struct Thread {
    slack_id: String,
    user_id: Option<i32>,
    user_name: Option<String>,
    last_text: String,
    last_at: DateTime<Utc>,
    unread: i64,
}

#[get("/slack/threads")]
async fn get_threads(_: Admin, pool: web::Data<PgPool>) -> Result<HttpResponse, ApiError> {
    let threads: Vec<Thread> = sqlx::query_as(
        "SELECT t.slack_id, u.id AS user_id, NULLIF(concat_ws(' ', u.first_name, u.last_name), '') AS user_name,
             last.text AS last_text, last.at AS last_at, t.unread
         FROM (SELECT slack_id, count(*) FILTER (WHERE incoming AND read_at IS NULL) AS unread
               FROM slack_message GROUP BY slack_id HAVING bool_or(incoming)) t
         CROSS JOIN LATERAL (
             SELECT text, COALESCE(sent_at, created_at) AS at FROM slack_message m
             WHERE m.slack_id = t.slack_id AND (m.incoming OR m.sent_at IS NOT NULL)
             ORDER BY at DESC, id DESC LIMIT 1
         ) last
         LEFT JOIN LATERAL (
             SELECT id, first_name, last_name FROM \"user\" WHERE slack_id = t.slack_id
             ORDER BY deleted_at IS NOT NULL, id LIMIT 1
         ) u ON true
         ORDER BY t.unread > 0 DESC, last.at DESC LIMIT $1",
    )
    .bind(THREADS)
    .fetch_all(pool.get_ref())
    .await?;
    Ok(HttpResponse::Ok().json(threads))
}

#[derive(Serialize, sqlx::FromRow)]
struct Message {
    id: i32,
    incoming: bool,
    kind: String,
    text: String,
    at: DateTime<Utc>,
    admin_name: Option<String>,
}

async fn thread(pool: &PgPool, slack_id: &str) -> sqlx::Result<Vec<Message>> {
    let mut messages: Vec<Message> = sqlx::query_as(
        "SELECT m.id, m.incoming, m.kind, m.text, COALESCE(m.sent_at, m.created_at) AS at,
             NULLIF(concat_ws(' ', a.first_name, a.last_name), '') AS admin_name
         FROM slack_message m LEFT JOIN \"user\" a ON a.id = m.admin_id
         WHERE m.slack_id = $1 AND (m.incoming OR m.sent_at IS NOT NULL)
         ORDER BY at DESC, m.id DESC LIMIT $2",
    )
    .bind(slack_id)
    .bind(THREAD_MESSAGES)
    .fetch_all(pool)
    .await?;
    messages.reverse();
    sqlx::query("UPDATE slack_message SET read_at = now() WHERE slack_id = $1 AND incoming AND read_at IS NULL")
        .bind(slack_id)
        .execute(pool)
        .await?;
    Ok(messages)
}

#[get("/slack/threads/{slack_id}")]
async fn get_thread(_: Admin, pool: web::Data<PgPool>, path: web::Path<String>) -> Result<HttpResponse, ApiError> {
    Ok(HttpResponse::Ok().json(thread(&pool, slack_id(&path)?).await?))
}

#[derive(Deserialize)]
struct ReplyRequest {
    text: String,
    key: String,
}

#[post("/slack/threads/{slack_id}/reply")]
async fn reply(
    Admin(admin_id): Admin,
    pool: web::Data<PgPool>,
    path: web::Path<String>,
    body: web::Json<ReplyRequest>,
) -> Result<HttpResponse, ApiError> {
    configured()?;
    let slack_id = slack_id(&path)?;
    send_now(&pool, admin_id, slack_id, "reply", text(&body.text)?, &body.key).await?;
    Ok(HttpResponse::Ok().json(thread(&pool, slack_id).await?))
}
