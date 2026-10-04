use crate::{accounts::hackatime, env, projects};
use sqlx::PgPool;

pub fn spawn(pool: PgPool) {
    actix_web::rt::spawn(async move {
        let mut interval = env::interval_secs("HACKATIME_SYNC_INTERVAL_SECS", 3600);
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
        "SELECT DISTINCT projects.user_id FROM projects JOIN users ON users.id = projects.user_id
         WHERE projects.deleted_at IS NULL AND users.deleted_at IS NULL AND hackatime_projects <> '{}'",
    )
    .fetch_all(pool)
    .await?;
    for user_id in users {
        if let Some(hackatime) = hackatime::summaries(pool, user_id).await {
            projects::record_time(pool, &projects::list(pool, user_id).await?, &hackatime).await?;
        }
    }
    Ok(())
}
