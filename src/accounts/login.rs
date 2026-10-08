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
    login::handle_login(&req, scopes, callback_url(CALLBACK)).await
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
        callback_url(CALLBACK),
        "/dashboard".to_string(),
        Some(sink),
        false,
    )
    .await;
    let Some(token) = token.take() else {
        return response;
    };
    let referral = req.cookie(REFERRAL_COOKIE).map(|cookie| cookie.value().to_string());
    let mut failure = match sign_in(&pool, &token, referral.as_deref()).await {
        Ok(Some(cookie)) => {
            let _ = response.add_cookie(&cookie);
            if referral.is_some() {
                let _ = response.add_removal_cookie(&Cookie::build(REFERRAL_COOKIE, "").path("/").finish());
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
    tokens::store(&mut *tx, user_id, Provider::Hca, token).await?;
    let cookie = session::create(&mut *tx, user_id).await?;
    tx.commit().await?;
    Ok(Some(cookie))
}
