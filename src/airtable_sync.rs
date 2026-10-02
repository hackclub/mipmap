use hc_helper::airtable::{AirtableTable, spawn_airtable_queue_handler, upsert_records};
use serde_json::{Map, Value};
use sqlx::{AssertSqlSafe, PgPool};
use std::time::Duration;

const TABLES: &[&str] = &["users", "projects", "snapshots", "orders", "unlocks"];

fn env(key: &str) -> Option<String> {
    std::env::var(key).ok().filter(|value| !value.is_empty())
}

pub fn spawn(pool: PgPool) -> anyhow::Result<()> {
    let (Some(_), Some(base_id)) = (env("AIRTABLE_TOKEN"), env("AIRTABLE_BASE_ID")) else {
        log::warn!("AIRTABLE_TOKEN or AIRTABLE_BASE_ID not set, Airtable sync disabled");
        return Ok(());
    };
    let interval = env("AIRTABLE_SYNC_INTERVAL_SECS")
        .and_then(|secs| secs.parse().ok())
        .unwrap_or(300);
    spawn_airtable_queue_handler()?;
    actix_web::rt::spawn(async move {
        let mut interval = actix_web::rt::time::interval(Duration::from_secs(interval));
        loop {
            interval.tick().await;
            for table in TABLES {
                if let Err(e) = sync(&pool, &base_id, table).await {
                    log::error!("Airtable sync of {table} failed: {e:#}");
                }
            }
        }
    });
    Ok(())
}

fn flatten(row: Map<String, Value>) -> Map<String, Value> {
    row.into_iter()
        .map(|(key, value)| match value {
            Value::Array(items) => {
                let items: Vec<String> = items
                    .into_iter()
                    .map(|item| {
                        item.as_str()
                            .map_or_else(|| item.to_string(), str::to_string)
                    })
                    .collect();
                (key, Value::String(items.join(", ")))
            }
            value => (key, value),
        })
        .collect()
}

async fn sync(pool: &PgPool, base_id: &str, table: &str) -> anyhow::Result<()> {
    let started: String = sqlx::query_scalar("SELECT now()::text")
        .fetch_one(pool)
        .await?;
    let rows: Vec<String> = sqlx::query_scalar(AssertSqlSafe(format!(
        "SELECT to_jsonb(t)::text FROM {table} t
         WHERE t.updated_at > COALESCE((SELECT synced_until FROM airtable_sync WHERE table_name = $1), '-infinity') - interval '1 minute'
         ORDER BY t.id"
    )))
    .bind(table)
    .fetch_all(pool)
    .await?;
    if !rows.is_empty() {
        let records = rows
            .iter()
            .map(|row| serde_json::from_str(row).map(flatten))
            .collect::<Result<Vec<_>, _>>()?;
        let airtable = AirtableTable {
            base_id: base_id.to_string(),
            table_id_or_name: env(&format!("AIRTABLE_{}_TABLE", table.to_uppercase()))
                .unwrap_or_else(|| table.to_string()),
        };
        upsert_records(airtable, records, vec!["id".to_string()]).await?;
    }
    sqlx::query(
        "INSERT INTO airtable_sync (table_name, synced_until) VALUES ($1, $2::timestamptz)
         ON CONFLICT (table_name) DO UPDATE SET synced_until = $2::timestamptz",
    )
    .bind(table)
    .bind(started)
    .execute(pool)
    .await?;
    Ok(())
}
