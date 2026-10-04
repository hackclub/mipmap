use super::{Kind, TREE};
use crate::{
    accounts::{profile, session::User},
    dashboard,
    error::{ApiError, reject},
};
use actix_web::{HttpResponse, post, web};
use serde::Deserialize;
use sqlx::{PgConnection, PgExecutor, PgPool};

#[derive(Deserialize)]
struct NodeRequest {
    id: u32,
}

pub async fn unlocked(db: impl PgExecutor<'_>, user_id: i32) -> sqlx::Result<Vec<i32>> {
    sqlx::query_scalar(
        "SELECT node_id FROM unlocks WHERE user_id = $1 AND deleted_at IS NULL ORDER BY node_id",
    )
    .bind(user_id)
    .fetch_all(db)
    .await
}

async fn lock_user(conn: &mut PgConnection, user_id: i32) -> Result<i32, ApiError> {
    let (mips, status, eligible): (i32, Option<String>, bool) = sqlx::query_as(
        "SELECT mips, verification_status, ysws_eligible FROM users WHERE id = $1 FOR UPDATE",
    )
    .bind(user_id)
    .fetch_one(conn)
    .await?;
    match profile::blocked_reason(status.as_deref(), eligible) {
        Some(reason) => Err(reject(reason)),
        None => Ok(mips),
    }
}

#[post("/unlock-item")]
pub async fn unlock_item(
    User(user_id): User,
    pool: web::Data<PgPool>,
    body: web::Json<NodeRequest>,
) -> Result<HttpResponse, ApiError> {
    let node = TREE
        .node(body.id)
        .filter(|node| node.kind != Kind::Origin)
        .ok_or_else(|| reject("unknown node"))?;
    let mut tx = pool.begin().await?;
    let mips = lock_user(&mut tx, user_id).await?;
    let unlocked = unlocked(&mut *tx, user_id).await?;
    if unlocked.contains(&(node.id as i32)) {
        return Err(reject("already unlocked"));
    }
    let reachable = node
        .requires
        .iter()
        .any(|&req| unlocked.contains(&(req as i32)) || TREE.is(req, Kind::Origin));
    if !reachable {
        return Err(reject("requirements not met"));
    }
    if mips < node.price {
        return Err(reject(format!("Not enough {}", TREE.currency)));
    }
    sqlx::query("UPDATE users SET mips = mips - $2 WHERE id = $1")
        .bind(user_id)
        .bind(node.price)
        .execute(&mut *tx)
        .await?;
    sqlx::query(
        "INSERT INTO unlocks (user_id, node_id) VALUES ($1, $2)
         ON CONFLICT (user_id, node_id) DO UPDATE SET deleted_at = NULL, unlocked_at = now()",
    )
    .bind(user_id)
    .bind(node.id as i32)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    dashboard::respond(&pool, user_id).await
}

#[post("/purchase-item")]
pub async fn purchase_item(
    User(user_id): User,
    pool: web::Data<PgPool>,
    body: web::Json<NodeRequest>,
) -> Result<HttpResponse, ApiError> {
    let node = TREE
        .node(body.id)
        .filter(|node| node.kind == Kind::Item)
        .ok_or_else(|| reject("unknown item"))?;
    let mut tx = pool.begin().await?;
    lock_user(&mut tx, user_id).await?;
    let unlocked = unlocked(&mut *tx, user_id).await?;
    if !unlocked.contains(&(node.id as i32)) {
        return Err(reject("item not unlocked"));
    }
    let ordered: Vec<i32> =
        sqlx::query_scalar("SELECT node_id FROM orders WHERE user_id = $1 AND deleted_at IS NULL")
            .bind(user_id)
            .fetch_all(&mut *tx)
            .await?;
    if ordered.contains(&(node.id as i32)) {
        return Err(reject("item already purchased"));
    }
    let tokens = unlocked
        .iter()
        .filter(|&&id| TREE.is(id as u32, Kind::Token))
        .count();
    if ordered.len() >= tokens {
        return Err(reject("no tokens available"));
    }
    let inserted = sqlx::query(
        "INSERT INTO orders (
             user_id, node_id, item_name, cost,
             address_first_name, address_last_name, address_line_1, address_line_2, address_city,
             address_state, address_postal_code, address_country, address_phone_number
         )
         SELECT id, $2, $3, 1,
             address_first_name, address_last_name, address_line_1, address_line_2, address_city,
             address_state, address_postal_code, address_country, address_phone_number
         FROM users
         WHERE id = $1 AND address_line_1 IS NOT NULL AND address_city IS NOT NULL
         AND address_country IS NOT NULL",
    )
    .bind(user_id)
    .bind(node.id as i32)
    .bind(&node.name)
    .execute(&mut *tx)
    .await?;
    if inserted.rows_affected() == 0 {
        return Err(reject(profile::NO_ADDRESS));
    }
    tx.commit().await?;
    dashboard::respond(&pool, user_id).await
}
