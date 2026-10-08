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
        "SELECT node_id FROM unlock WHERE user_id = $1 AND deleted_at IS NULL ORDER BY node_id",
    )
    .bind(user_id)
    .fetch_all(db)
    .await
}

async fn lock_user(conn: &mut PgConnection, user_id: i32) -> Result<i32, ApiError> {
    let (mips, status): (i32, Option<String>) = sqlx::query_as(
        "SELECT mips, verification_status FROM \"user\" WHERE id = $1 FOR UPDATE",
    )
    .bind(user_id)
    .fetch_one(conn)
    .await?;
    match profile::shop_blocked(status.as_deref()) {
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
    sqlx::query("UPDATE \"user\" SET mips = mips - $2 WHERE id = $1")
        .bind(user_id)
        .bind(node.price)
        .execute(&mut *tx)
        .await?;
    sqlx::query(
        "INSERT INTO unlock (user_id, node_id, price) VALUES ($1, $2, $3)
         ON CONFLICT (user_id, node_id) DO UPDATE
         SET deleted_at = NULL, unlocked_at = now(), price = EXCLUDED.price",
    )
    .bind(user_id)
    .bind(node.id as i32)
    .bind(node.price)
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
    if lock_user(&mut tx, user_id).await? < 0 {
        return Err(reject(format!("Your {} balance is negative", TREE.currency)));
    }
    let unlocked = unlocked(&mut *tx, user_id).await?;
    if !unlocked.contains(&(node.id as i32)) {
        return Err(reject("item not unlocked"));
    }
    let ordered: Vec<i32> =
        sqlx::query_scalar("SELECT node_id FROM \"order\" WHERE user_id = $1 AND deleted_at IS NULL")
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
        "INSERT INTO \"order\" (
             user_id, node_id, item_name, cost,
             address_first_name, address_last_name, address_line_1, address_line_2, address_city,
             address_state, address_postal_code, address_country, address_phone_number
         )
         SELECT id, $2, $3, 1,
             address_first_name, address_last_name, address_line_1, address_line_2, address_city,
             address_state, address_postal_code, address_country, address_phone_number
         FROM \"user\"
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

pub async fn backfill_prices(pool: &PgPool) -> sqlx::Result<()> {
    let (ids, prices): (Vec<i32>, Vec<i32>) = TREE
        .nodes
        .iter()
        .map(|node| (node.id as i32, node.price))
        .unzip();
    let filled = sqlx::query(
        "UPDATE unlock SET price = t.price
         FROM UNNEST($1::int[], $2::int[]) AS t (node_id, price)
         WHERE unlock.node_id = t.node_id AND unlock.price IS NULL",
    )
    .bind(ids)
    .bind(prices)
    .execute(pool)
    .await?
    .rows_affected();
    if filled > 0 {
        log::info!("Backfilled the price of {filled} unlocks");
    }
    Ok(())
}

fn reached(unlocked: &[i32]) -> Vec<i32> {
    let mut reached = Vec::new();
    loop {
        let before = reached.len();
        for &id in unlocked {
            if !reached.contains(&id)
                && TREE.node(id as u32).is_none_or(|node| {
                    node.requires
                        .iter()
                        .any(|&req| TREE.is(req, Kind::Origin) || reached.contains(&(req as i32)))
                })
            {
                reached.push(id);
            }
        }
        if reached.len() == before {
            return reached;
        }
    }
}

pub async fn refund(conn: &mut PgConnection, user_id: i32) -> anyhow::Result<()> {
    let mips: i32 = sqlx::query_scalar("SELECT mips FROM \"user\" WHERE id = $1 FOR UPDATE")
        .bind(user_id)
        .fetch_one(&mut *conn)
        .await?;
    let mut orders: Vec<(i32, i32, bool)> = sqlx::query_as(
        "SELECT id, node_id, fulfilled FROM \"order\" WHERE user_id = $1 AND deleted_at IS NULL
         ORDER BY ordered_at DESC, id DESC",
    )
    .bind(user_id)
    .fetch_all(&mut *conn)
    .await?;
    let unlocks: Vec<(i32, Option<i32>)> = sqlx::query_as(
        "SELECT node_id, price FROM unlock WHERE user_id = $1 AND deleted_at IS NULL
         ORDER BY unlocked_at DESC, id DESC",
    )
    .bind(user_id)
    .fetch_all(&mut *conn)
    .await?;
    let mut kept: Vec<i32> = unlocks.iter().map(|&(id, _)| id).collect();
    let mut undone = Vec::new();
    let mut cancelled = Vec::new();
    let mut owed = -mips;
    let mut refunded = 0;
    for (id, price) in unlocks {
        if owed <= 0 {
            break;
        }
        if orders.iter().any(|&(_, node, fulfilled)| node == id && fulfilled) {
            continue;
        }
        let rest: Vec<i32> = kept.iter().copied().filter(|&node| node != id).collect();
        let after = reached(&rest);
        if reached(&kept).iter().any(|node| *node != id && !after.contains(node)) {
            continue;
        }
        let mut dropped: Vec<i32> = orders
            .iter()
            .filter(|&&(_, node, _)| node == id)
            .map(|&(order, ..)| order)
            .collect();
        if TREE.is(id as u32, Kind::Token) {
            let tokens = rest.iter().filter(|&&node| TREE.is(node as u32, Kind::Token)).count();
            let excess = (orders.len() - dropped.len()).saturating_sub(tokens);
            let spare: Vec<i32> = orders
                .iter()
                .filter(|&&(order, _, fulfilled)| !fulfilled && !dropped.contains(&order))
                .take(excess)
                .map(|&(order, ..)| order)
                .collect();
            if spare.len() < excess {
                continue;
            }
            dropped.extend(spare);
        }
        orders.retain(|(order, ..)| !dropped.contains(order));
        cancelled.extend(dropped);
        kept = rest;
        undone.push(id);
        let price = price.or_else(|| TREE.node(id as u32).map(|node| node.price)).unwrap_or(0);
        owed -= price;
        refunded += price;
    }
    sqlx::query("UPDATE \"order\" SET deleted_at = now() WHERE id = ANY($1)")
        .bind(&cancelled)
        .execute(&mut *conn)
        .await?;
    sqlx::query("UPDATE unlock SET deleted_at = now() WHERE user_id = $1 AND node_id = ANY($2)")
        .bind(user_id)
        .bind(&undone)
        .execute(&mut *conn)
        .await?;
    sqlx::query("UPDATE \"user\" SET mips = mips + $2 WHERE id = $1")
        .bind(user_id)
        .bind(refunded)
        .execute(&mut *conn)
        .await?;
    log::warn!(
        "Refunded user {user_id}'s shop: {} orders cancelled, {} unlocks undone for {refunded} {}{}",
        cancelled.len(),
        undone.len(),
        TREE.currency,
        if owed > 0 { format!(", still {owed} short") } else { String::new() }
    );
    Ok(())
}
