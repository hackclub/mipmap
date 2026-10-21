use crate::{accounts::hackatime, env, projects, slack};
use sqlx::PgPool;
use tokio::time::MissedTickBehavior;

pub fn spawn(pool: PgPool) {
    actix_web::rt::spawn(async move {
        let mut interval = env::interval_secs("HACKATIME_SYNC_INTERVAL_SECS", 3600);
        interval.set_missed_tick_behavior(MissedTickBehavior::Delay);
        loop {
            interval.tick().await;
            if let Err(e) = sync(&pool).await {
                log::error!("Hackatime sync failed: {e:#}");
            }
        }
    });
}

async fn sync(pool: &PgPool) -> sqlx::Result<()> {
    let users: Vec<i32> = sqlx::query_scalar(
        "SELECT DISTINCT project.user_id FROM project JOIN \"user\" ON \"user\".id = project.user_id
         WHERE project.deleted_at IS NULL AND \"user\".deleted_at IS NULL AND hackatime_projects <> '{}'",
    )
    .fetch_all(pool)
    .await?;
    for user_id in users {
        if let Err(e) = sync_user(pool, user_id).await {
            log::error!("Hackatime sync for user {user_id} failed: {e:#}");
        }
    }
    Ok(())
}

async fn sync_user(pool: &PgPool, user_id: i32) -> sqlx::Result<()> {
    let Some(hackatime) = hackatime::summaries(pool, user_id, false).await else {
        return Ok(());
    };
    let projects = projects::list(pool, user_id).await?;
    projects::record_time(pool, &projects, &hackatime).await?;
    let totals = hackatime::totals(&hackatime);
    if let Err(e) = slack::snapshot_reminder(pool, user_id, &totals, &projects).await {
        log::error!("Queueing a Snapshot reminder for user {user_id} failed: {e:#}");
    }
    Ok(())
}
