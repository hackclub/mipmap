use sqlx::{PgPool, Postgres, migrate::MigrateDatabase, postgres::PgPoolOptions};

pub async fn init(url: &str) -> anyhow::Result<PgPool> {
    if !Postgres::database_exists(url).await? {
        log::info!("creating database");
        Postgres::create_database(url).await?;
    }
    let pool = PgPoolOptions::new().max_connections(5).connect(url).await?;
    sqlx::migrate!().run(&pool).await?;
    Ok(pool)
}
