use super::{
    callback_url, redirect, session, token_sink,
    tokens::{self, Provider},
};
use actix_web::{
    HttpRequest, HttpResponse, get,
    web::{self, Query},
};
use hc_helper::hackatime::{
    data::{Project, get_hackatime_project_summaries_with_token, get_hackatime_user_with_token},
    login::{self, CallbackArgs, Scopes},
};
use sqlx::{PgPool, types::Json};
use std::collections::BTreeMap;

const CALLBACK: &str = "/hackatime/callback";

pub type Totals = BTreeMap<String, i64>;

pub fn totals(projects: &[Project]) -> Totals {
    projects
        .iter()
        .map(|p| (p.name.clone(), p.total_duration_seconds as i64))
        .collect()
}

pub async fn summaries(pool: &PgPool, user_id: i32) -> Option<Vec<Project>> {
    let token = match tokens::load(pool, user_id, Provider::Hackatime).await {
        Ok(token) => token?,
        Err(e) => {
            log::error!("loading Hackatime token failed: {e:#}");
            return None;
        }
    };
    match get_hackatime_project_summaries_with_token(&token).await {
        Ok(projects) => Some(projects),
        Err(e) => {
            log::warn!("fetching Hackatime projects failed: {e:#}");
            None
        }
    }
}

async fn signed_in(req: &HttpRequest) -> Result<i32, HttpResponse> {
    match session::lookup_page(req).await? {
        Some(session) if !session.banned => Ok(session.user_id),
        Some(_) => Err(redirect("/dashboard")),
        None => Err(redirect("/login")),
    }
}

#[get("/hackatime/login")]
pub async fn start(req: HttpRequest) -> HttpResponse {
    if let Err(response) = signed_in(&req).await {
        return response;
    }
    let scopes = Scopes {
        profile: true,
        read: true,
        admin: false,
    };
    login::handle_login(&req, scopes, callback_url(CALLBACK), true).await
}

#[get("/hackatime/callback")]
pub async fn callback(
    req: HttpRequest,
    query: Query<CallbackArgs>,
    pool: web::Data<PgPool>,
) -> HttpResponse {
    let user_id = match signed_in(&req).await {
        Ok(user_id) => user_id,
        Err(response) => return response,
    };
    let (token, sink) = token_sink();
    let response = login::handle_callback(
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
    match link(&pool, user_id, &token).await {
        Ok(Linked::Yes) => response,
        Ok(Linked::Taken) => HttpResponse::Conflict()
            .body("That Hackatime account is already linked to another mipmap account"),
        Ok(Linked::Other) => HttpResponse::Conflict()
            .body("Your Mipmap account is already linked to a different Hackatime account"),
        Err(e) => {
            log::error!("linking Hackatime failed: {e:#}");
            HttpResponse::InternalServerError().body("Couldn't link Hackatime, please try again")
        }
    }
}

enum Linked {
    Yes,
    Taken,
    Other,
}

async fn link(pool: &PgPool, user_id: i32, token: &str) -> anyhow::Result<Linked> {
    let user = get_hackatime_user_with_token(token).await?;
    anyhow::ensure!(user.user_id != 0, "Hackatime returned a user without an id");
    let baseline = totals(&get_hackatime_project_summaries_with_token(token).await?);
    let mut tx = pool.begin().await?;
    let linked = sqlx::query(
        "UPDATE \"user\" SET hackatime_user_id = $2, hackatime_trust_level = $3,
             hackatime_baseline = COALESCE(hackatime_baseline, $4),
             hackatime_linked_at = COALESCE(hackatime_linked_at, now())
         WHERE id = $1 AND (hackatime_user_id IS NULL OR hackatime_user_id = $2)",
    )
    .bind(user_id)
    .bind(user.user_id as i64)
    .bind(&user.trust_factor.trust_level)
    .bind(Json(baseline))
    .execute(&mut *tx)
    .await;
    match linked {
        Err(e) if e.as_database_error().is_some_and(|e| e.is_unique_violation()) => Ok(Linked::Taken),
        Err(e) => Err(e.into()),
        Ok(done) if done.rows_affected() == 0 => Ok(Linked::Other),
        Ok(_) => {
            tokens::store(&mut *tx, user_id, Provider::Hackatime, token).await?;
            tx.commit().await?;
            Ok(Linked::Yes)
        }
    }
}
