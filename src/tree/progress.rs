use super::{Kind, Node, STICKER, STICKER_TOKEN, TREE, tutorial};
use crate::{
    accounts::{profile, session::User},
    dashboard,
    error::{ApiError, reject},
};
use actix_web::{HttpResponse, post, web};
use serde::Deserialize;
use sqlx::{AssertSqlSafe, PgConnection, PgExecutor, PgPool};
use std::{
    collections::HashMap,
    sync::{LazyLock, Mutex, PoisonError},
    time::{Duration, Instant},
};

const ELIGIBLE_FOR: Duration = Duration::from_secs(5 * 60);

static ELIGIBLE: LazyLock<Mutex<HashMap<i32, Instant>>> = LazyLock::new(Default::default);

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

struct Locked {
    mips: i32,
    onboarded: bool,
    sticker_skipped: bool,
}

async fn read_user(db: impl PgExecutor<'_>, user_id: i32, verify: bool, lock: bool) -> Result<Locked, ApiError> {
    let (mips, onboarded, sticker_skipped, status, eligible): (i32, bool, bool, Option<String>, bool) =
        sqlx::query_as(AssertSqlSafe(format!(
            "SELECT mips, onboarded_at IS NOT NULL, sticker_skipped_at IS NOT NULL,
                 verification_status, ysws_eligible
             FROM \"user\" WHERE id = $1{}",
            if lock { " FOR UPDATE" } else { "" }
        )))
    .bind(user_id)
    .fetch_one(db)
    .await?;
    match profile::shop_blocked(status.as_deref(), eligible).filter(|_| verify) {
        Some(reason) => Err(reject(reason)),
        None => Ok(Locked { mips, onboarded, sticker_skipped }),
    }
}

async fn lock_user(conn: &mut PgConnection, user_id: i32, verify: bool) -> Result<Locked, ApiError> {
    read_user(conn, user_id, verify, true).await
}

fn can_unlock(node: &Node, practice: bool, user: &Locked, unlocked: &[i32]) -> Result<(), ApiError> {
    if !user.onboarded && !practice {
        return Err(reject("Finish the tutorial first"));
    }
    if unlocked.contains(&(node.id as i32)) {
        return Err(reject("already unlocked"));
    }
    let reachable = node
        .requires
        .iter()
        .any(|&req| unlocked.contains(&(req as i32)) || TREE.is(req, Kind::Origin));
    if !reachable {
        return Err(reject("Missing prerequisite, maybe you need to reload?"));
    }
    if user.mips < node.price {
        return Err(reject(format!("Not enough {}", TREE.currency)));
    }
    Ok(())
}

fn can_purchase(node: &Node, user: &Locked, unlocked: &[i32], ordered: &[i32]) -> Result<(), ApiError> {
    let sticker = node.id == STICKER;
    if user.mips < 0 {
        return Err(reject(format!("Your {} balance is negative", TREE.currency)));
    }
    if !unlocked.contains(&(node.id as i32)) {
        return Err(reject("item not unlocked"));
    }
    if ordered.contains(&(node.id as i32)) {
        return Err(reject("Item already redeemed"));
    }
    let available = if sticker {
        !user.sticker_skipped && unlocked.contains(&(STICKER_TOKEN as i32))
    } else {
        tokens(unlocked) > ordered.iter().filter(|&&id| id != STICKER as i32).count()
    };
    if !available {
        return Err(reject("No Tokens available"));
    }
    Ok(())
}

async fn ordered(db: impl PgExecutor<'_>, user_id: i32) -> sqlx::Result<Vec<i32>> {
    sqlx::query_scalar("SELECT node_id FROM \"order\" WHERE user_id = $1 AND deleted_at IS NULL")
        .bind(user_id)
        .fetch_all(db)
        .await
}

async fn ensure_eligible(pool: &PgPool, user_id: i32) -> Result<(), ApiError> {
    {
        let mut eligible = ELIGIBLE.lock().unwrap_or_else(PoisonError::into_inner);
        eligible.retain(|_, at| at.elapsed() < ELIGIBLE_FOR);
        if eligible.contains_key(&user_id) {
            return Ok(());
        }
    }
    let hca_id: String = sqlx::query_scalar("SELECT hca_id FROM \"user\" WHERE id = $1")
        .bind(user_id)
        .fetch_one(pool)
        .await?;
    let eligible = profile::ysws_eligible(&hca_id).await.map_err(|e| {
        log::warn!("Checking user {user_id}'s YSWS eligibility failed: {e:#}");
        reject("Couldn't reach Hack Club Auth. Please try again")
    })?;
    if !eligible {
        return Err(reject(profile::INELIGIBLE));
    }
    ELIGIBLE
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .insert(user_id, Instant::now());
    Ok(())
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
    let practice = tutorial(node.id);
    if !practice {
        let user = read_user(pool.get_ref(), user_id, true, false).await?;
        can_unlock(node, practice, &user, &unlocked(pool.get_ref(), user_id).await?)?;
        ensure_eligible(&pool, user_id).await?;
    }
    let mut tx = pool.begin().await?;
    let user = lock_user(&mut tx, user_id, !practice).await?;
    can_unlock(node, practice, &user, &unlocked(&mut *tx, user_id).await?)?;
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
    let sticker = node.id == STICKER;
    if !sticker {
        let user = read_user(pool.get_ref(), user_id, true, false).await?;
        can_purchase(
            node,
            &user,
            &unlocked(pool.get_ref(), user_id).await?,
            &ordered(pool.get_ref(), user_id).await?,
        )?;
        ensure_eligible(&pool, user_id).await?;
    }
    let addressed: bool =
        sqlx::query_scalar("SELECT address_line_1 IS NOT NULL FROM \"user\" WHERE id = $1")
            .bind(user_id)
            .fetch_one(pool.get_ref())
            .await?;
    if !addressed && let Err(e) = profile::refresh(&pool, user_id).await {
        log::warn!("refreshing user {user_id}'s HCA profile failed: {e:#}");
    }
    let mut tx = pool.begin().await?;
    let locked = lock_user(&mut tx, user_id, !sticker).await?;
    let unlocked = unlocked(&mut *tx, user_id).await?;
    let ordered = ordered(&mut *tx, user_id).await?;
    can_purchase(node, &locked, &unlocked, &ordered)?;
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

#[post("/skip-sticker")]
pub async fn skip_sticker(User(user_id): User, pool: web::Data<PgPool>) -> Result<HttpResponse, ApiError> {
    let mut tx = pool.begin().await?;
    if lock_user(&mut tx, user_id, false).await?.sticker_skipped {
        return Err(reject("Sticker already skipped"));
    }
    if !unlocked(&mut *tx, user_id).await?.contains(&(STICKER_TOKEN as i32)) {
        return Err(reject("No Tokens available"));
    }
    let skipped = sqlx::query(
        "UPDATE \"user\" SET sticker_skipped_at = now()
         WHERE id = $1 AND sticker_skipped_at IS NULL
         AND NOT EXISTS (SELECT 1 FROM \"order\" WHERE user_id = $1 AND node_id = $2 AND deleted_at IS NULL)",
    )
    .bind(user_id)
    .bind(STICKER as i32)
    .execute(&mut *tx)
    .await?;
    if skipped.rows_affected() == 0 {
        return Err(reject("Item already redeemed"));
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

fn tokens(unlocked: &[i32]) -> usize {
    unlocked
        .iter()
        .filter(|&&id| id != STICKER_TOKEN as i32 && TREE.is(id as u32, Kind::Token))
        .count()
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
                        .any(|&req| TREE.is(req, Kind::Origin) || tutorial(req) || reached.contains(&(req as i32)))
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
         ORDER BY ordered_at DESC, id DESC FOR UPDATE",
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
    let mut progressed = true;
    while owed > 0 && progressed {
        progressed = false;
        for &(id, price) in &unlocks {
            if owed <= 0 {
                break;
            }
            if undone.contains(&id) || tutorial(id as u32) {
                continue;
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
                let generic = |node: i32| node != STICKER as i32;
                let spent = orders
                    .iter()
                    .filter(|&&(order, node, _)| generic(node) && !dropped.contains(&order))
                    .count();
                let excess = spent.saturating_sub(tokens(&rest));
                let spare: Vec<i32> = orders
                    .iter()
                    .filter(|&&(order, node, fulfilled)| {
                        !fulfilled && generic(node) && !dropped.contains(&order)
                    })
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
            progressed = true;
        }
    }
    sqlx::query("UPDATE \"order\" SET deleted_at = now() WHERE id = ANY($1) AND NOT fulfilled AND deleted_at IS NULL")
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
