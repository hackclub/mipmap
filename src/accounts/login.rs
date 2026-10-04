use super::{
    callback_url, profile, session, token_sink,
    tokens::{self, Provider},
};
use actix_web::{
    HttpRequest, HttpResponse, cookie::Cookie, get,
    web::{self, Query},
};
use hc_helper::auth::{
    data::get_auth_data_with_token,
    login::{self, CallbackArgs, Scopes},
};
use sqlx::PgPool;

const CALLBACK: &str = "/callback";
const REFERRAL_COOKIE: &str = "ref";

#[get("/login")]
pub async fn start(req: HttpRequest) -> HttpResponse {
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
    login::handle_login(&req, scopes, callback_url(&req, CALLBACK)).await
}

#[get("/callback")]
pub async fn callback(
    req: HttpRequest,
    query: Query<CallbackArgs>,
    pool: web::Data<PgPool>,
) -> HttpResponse {
    let (token, sink) = token_sink();
    let mut response = login::handle_callback(
        &req,
        query,
        callback_url(&req, CALLBACK),
        "/dashboard".to_string(),
        Some(sink),
        false,
    )
    .await;
    let Some(token) = token.take() else {
        return response;
    };
    let referral = req.cookie(REFERRAL_COOKIE).map(|cookie| cookie.value().to_string());
    match sign_in(&pool, &token, referral.as_deref()).await {
        Ok(None) => HttpResponse::Forbidden().body("This account has been deleted."),
        Ok(Some(cookie)) => {
            let _ = response.add_cookie(&cookie);
            if referral.is_some() {
                let _ = response.add_removal_cookie(&Cookie::build(REFERRAL_COOKIE, "").path("/").finish());
            }
            response
        }
        Err(e) => {
            log::error!("sign in failed: {e:#}");
            HttpResponse::InternalServerError().body("Couldn't sign you in, please try again.")
        }
    }
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
    let user_id = profile::sync(pool, &data).await?;
    if let (false, Some(code)) = (existing, referral) {
        sqlx::query(
            "UPDATE \"user\" SET referred_by = referrer.id FROM \"user\" referrer
             WHERE \"user\".id = $1 AND \"user\".referred_by IS NULL
             AND referrer.referral_code = $2 AND referrer.id <> $1",
        )
        .bind(user_id)
        .bind(code)
        .execute(pool)
        .await?;
    }
    sqlx::query("UPDATE \"user\" SET last_login_at = now() WHERE id = $1")
        .bind(user_id)
        .execute(pool)
        .await?;
    tokens::store(pool, user_id, Provider::Hca, token).await?;
    session::create(pool, user_id).await.map(Some)
}
