use super::{
    profile, session, site_url, token_sink,
    tokens::{self, Provider},
};
use actix_web::{
    HttpRequest, HttpResponse,
    cookie::{Cookie, SameSite, time::Duration},
    get,
    http::header::{self, HeaderValue},
    web::{self, Query},
};
use hc_helper::auth::{
    data::get_auth_data_with_token,
    login::{self, CallbackArgs, Scopes},
};
use reqwest::Url;
use serde::Deserialize;
use sqlx::PgPool;

const CALLBACK: &str = "/callback";
const REFERRAL_COOKIE: &str = "__Host-ref";
const NEXT_COOKIE: &str = "next";

fn local_path(path: &str) -> bool {
    path.starts_with('/')
        && !path.starts_with("//")
        && path.bytes().all(|byte| byte.is_ascii_alphanumeric() || b"/?=&_.-%".contains(&byte))
}

pub fn login_redirect(req: &HttpRequest) -> HttpResponse {
    let next = req.uri().path_and_query().map_or(req.path(), |path| path.as_str());
    match Url::parse_with_params(&site_url("/login"), [("next", next)]) {
        Ok(url) if local_path(next) => super::redirect(url.as_str()),
        _ => super::redirect("/login"),
    }
}

#[derive(Deserialize)]
pub struct LoginQuery {
    email: Option<String>,
    next: Option<String>,
}

#[get("/login")]
pub async fn start(req: HttpRequest, query: Query<LoginQuery>) -> HttpResponse {
    let scopes = Scopes {
        openid: false,
        profile: false,
        email: false,
        name: false,
        slack_id: true,
        verification_status: true,
        basic_info: true,
        addresses: true,
        phone_number: true,
    };
    let mut response = login::handle_login(&req, scopes, site_url(CALLBACK)).await;
    let email = query.email.as_deref().map(str::trim).filter(|email| {
        email.len() <= 254 && email.contains('@') && !email.chars().any(char::is_control)
    });
    if let Some(email) = email
        && let Some(mut url) = response
            .headers()
            .get(header::LOCATION)
            .and_then(|location| location.to_str().ok())
            .and_then(|location| Url::parse(location).ok())
    {
        url.query_pairs_mut()
            .append_pair("login_hint", email)
            .append_pair("email", email);
        if let Ok(location) = HeaderValue::from_str(url.as_str()) {
            response.headers_mut().insert(header::LOCATION, location);
        }
    }
    match query.next.as_deref().filter(|next| local_path(next)) {
        Some(next) => {
            let cookie = Cookie::build(NEXT_COOKIE, next.to_string())
                .path("/")
                .http_only(true)
                .secure(true)
                .same_site(SameSite::Lax)
                .max_age(Duration::minutes(30))
                .finish();
            let _ = response.add_cookie(&cookie);
        }
        None if req.cookie(NEXT_COOKIE).is_some() => {
            let _ = response.add_removal_cookie(&Cookie::build(NEXT_COOKIE, "").path("/").finish());
        }
        None => {}
    }
    response
}

#[get("/callback")]
pub async fn callback(
    req: HttpRequest,
    query: Query<CallbackArgs>,
    pool: web::Data<PgPool>,
) -> HttpResponse {
    let (token, sink) = token_sink();
    let next = req.cookie(NEXT_COOKIE).map(|cookie| cookie.value().to_string());
    let mut response = login::handle_callback(
        &req,
        query,
        site_url(CALLBACK),
        next.clone().filter(|next| local_path(next)).unwrap_or_else(|| "/dashboard".to_string()),
        Some(sink),
        false,
    )
    .await;
    if next.is_some() {
        let _ = response.add_removal_cookie(&Cookie::build(NEXT_COOKIE, "").path("/").finish());
    }
    let Some(token) = token.take() else {
        return response;
    };
    let referral = req.cookie(REFERRAL_COOKIE).map(|cookie| cookie.value().to_string());
    let mut failure = match sign_in(&pool, &token, referral.as_deref()).await {
        Ok(Some(cookie)) => {
            let _ = response.add_cookie(&cookie);
            if referral.is_some() {
                let removal = Cookie::build(REFERRAL_COOKIE, "").path("/").secure(true).finish();
                let _ = response.add_removal_cookie(&removal);
            }
            return response;
        }
        Ok(None) => HttpResponse::Forbidden().body("This account has been deleted"),
        Err(e) => {
            log::error!("sign in failed: {e:#}");
            HttpResponse::InternalServerError().body("Couldn't sign you in, please try again")
        }
    };
    for cookie in response.cookies() {
        let _ = failure.add_cookie(&cookie);
    }
    failure
}

async fn sign_in(
    pool: &PgPool,
    token: &str,
    referral: Option<&str>,
) -> anyhow::Result<Option<Cookie<'static>>> {
    let data = get_auth_data_with_token(token).await?;
    let deleted: Option<bool> =
        sqlx::query_scalar("SELECT deleted_at IS NOT NULL FROM \"user\" WHERE hca_id = $1")
            .bind(&data.id)
            .fetch_optional(pool)
            .await?;
    if deleted == Some(true) {
        return Ok(None);
    }
    let existing = deleted.is_some();
    let mut tx = pool.begin().await?;
    let user_id = profile::sync(&mut *tx, &data).await?;
    if let (false, Some(code)) = (existing, referral) {
        sqlx::query(
            "UPDATE \"user\" SET referred_by = referrer.id FROM \"user\" referrer
             WHERE \"user\".id = $1 AND \"user\".referred_by IS NULL
             AND referrer.referral_code = $2 AND referrer.id <> $1",
        )
        .bind(user_id)
        .bind(code)
        .execute(&mut *tx)
        .await?;
    }
    sqlx::query("UPDATE \"user\" SET last_login_at = now() WHERE id = $1")
        .bind(user_id)
        .execute(&mut *tx)
        .await?;
    sqlx::query("INSERT INTO login (user_id) VALUES ($1)")
        .bind(user_id)
        .execute(&mut *tx)
        .await?;
    tokens::store(&mut *tx, user_id, Provider::Hca, token).await?;
    let cookie = session::create(&mut *tx, user_id).await?;
    tx.commit().await?;
    Ok(Some(cookie))
}
