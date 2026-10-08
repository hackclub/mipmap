use crate::error::{ApiError, reject_with};
use actix_web::{
    FromRequest, HttpRequest, HttpResponse,
    cookie::{Cookie, SameSite, time::Duration},
    dev::Payload,
    http::StatusCode,
    post, web,
};
use sha2::{Digest, Sha256};
use sqlx::{PgExecutor, PgPool};
use std::{future::Future, pin::Pin};

const COOKIE: &str = "session";

fn hash(token: &str) -> Vec<u8> {
    Sha256::digest(token.as_bytes()).to_vec()
}

pub async fn create(executor: impl PgExecutor<'_>, user_id: i32) -> anyhow::Result<Cookie<'static>> {
    let token = hex::encode(rand::random::<[u8; 32]>());
    sqlx::query(
        "INSERT INTO session (user_id, token_hash, expires_at) VALUES ($1, $2, now() + interval '30 days')",
    )
    .bind(user_id)
    .bind(hash(&token))
    .execute(executor)
    .await?;
    Ok(Cookie::build(COOKIE, token)
        .path("/")
        .http_only(true)
        .secure(true)
        .same_site(SameSite::Lax)
        .max_age(Duration::days(400))
        .finish())
}

#[post("/logout")]
pub async fn logout(req: HttpRequest, pool: web::Data<PgPool>) -> Result<HttpResponse, ApiError> {
    if let Some(cookie) = req.cookie(COOKIE) {
        sqlx::query("DELETE FROM session WHERE token_hash = $1")
            .bind(hash(cookie.value()))
            .execute(pool.get_ref())
            .await?;
    }
    let mut response = HttpResponse::NoContent().finish();
    let _ = response.add_removal_cookie(&Cookie::build(COOKIE, "").path("/").finish());
    Ok(response)
}

pub async fn prune(pool: &PgPool) -> sqlx::Result<()> {
    sqlx::query("DELETE FROM session WHERE expires_at <= now()")
        .execute(pool)
        .await?;
    Ok(())
}

pub struct Session {
    pub user_id: i32,
    pub banned: bool,
}

pub async fn lookup(req: &HttpRequest) -> anyhow::Result<Option<Session>> {
    let Some(cookie) = req.cookie(COOKIE) else {
        return Ok(None);
    };
    let pool = req
        .app_data::<web::Data<PgPool>>()
        .ok_or_else(|| anyhow::anyhow!("database pool missing"))?;
    let row: Option<(i32, bool)> = sqlx::query_as(
        "WITH s AS (
             SELECT \"user\".id, \"user\".is_banned, \"user\".last_activity_at, session.expires_at
             FROM session JOIN \"user\" ON \"user\".id = session.user_id
             WHERE token_hash = $1 AND expires_at > now() AND \"user\".deleted_at IS NULL
         ), extended AS (
             UPDATE session SET expires_at = now() + interval '30 days' FROM s
             WHERE token_hash = $1 AND s.expires_at < now() + interval '29 days'
         ), active AS (
             UPDATE \"user\" SET last_activity_at = now() FROM s
             WHERE \"user\".id = s.id
             AND (s.last_activity_at IS NULL OR s.last_activity_at < now() - interval '1 hour')
         )
         SELECT id, is_banned FROM s",
    )
    .bind(hash(cookie.value()))
    .fetch_optional(pool.get_ref())
    .await?;
    Ok(row.map(|(user_id, banned)| Session { user_id, banned }))
}

pub async fn lookup_page(req: &HttpRequest) -> Result<Option<Session>, HttpResponse> {
    lookup(req).await.map_err(|e| {
        log::error!("session lookup failed: {e:#}");
        HttpResponse::InternalServerError().finish()
    })
}

pub struct User(pub i32);

impl FromRequest for User {
    type Error = ApiError;
    type Future = Pin<Box<dyn Future<Output = Result<Self, Self::Error>>>>;

    fn from_request(req: &HttpRequest, _: &mut Payload) -> Self::Future {
        let req = req.clone();
        Box::pin(async move {
            match lookup(&req).await? {
                Some(Session {
                    user_id,
                    banned: false,
                }) => Ok(User(user_id)),
                Some(_) => Err(reject_with(StatusCode::FORBIDDEN, "Your account has been suspended")),
                None => Err(reject_with(StatusCode::UNAUTHORIZED, "Not logged in")),
            }
        })
    }
}
