use crate::{accounts::session, env};
use hc_helper::airtable::{AirtableTable, create_field, create_table, tables, upsert_records};
use serde_json::{Map, Value, json};
use sqlx::{AssertSqlSafe, PgPool};

const TABLES: &[&str] = &["user", "project", "snapshot", "order", "unlock", "submission"];

fn table_name(table: &str) -> String {
    env::var(&format!("AIRTABLE_MIRROR_{}_TABLE", table.to_uppercase()))
        .unwrap_or_else(|| table.to_string())
}

pub fn spawn(pool: PgPool) {
    let airtable = env::var("AIRTABLE_TOKEN").zip(env::var("AIRTABLE_MIRROR_BASE_ID"));
    if airtable.is_none() {
        log::warn!("AIRTABLE_TOKEN or AIRTABLE_MIRROR_BASE_ID not set, Airtable mirror disabled");
    }
    actix_web::rt::spawn(async move {
        let mut interval = env::interval_secs("AIRTABLE_SYNC_INTERVAL_SECS", 300);
        let mut ready = false;
        loop {
            interval.tick().await;
            if let Err(e) = session::prune(&pool).await {
                log::error!("Pruning expired sessions failed: {e:#}");
            }
            let Some((token, base_id)) = &airtable else {
                continue;
            };
            if !ready {
                match set_up(&pool, token, base_id).await {
                    Ok(()) => ready = true,
                    Err(e) => {
                        log::error!("Setting up the Airtable mirror failed: {e:#}");
                        continue;
                    }
                }
            }
            for table in TABLES {
                if let Err(e) = sync(&pool, token, base_id, table).await {
                    log::error!("Airtable mirror sync of {table} failed: {e:#}");
                }
            }
        }
    });
}

fn field(name: &str, data_type: &str) -> Value {
    let (kind, options) = match data_type {
        "smallint" | "integer" | "bigint" => ("number", json!({ "precision": 0 })),
        "real" | "double precision" | "numeric" => ("number", json!({ "precision": 8 })),
        "boolean" => ("checkbox", json!({ "icon": "check", "color": "greenBright" })),
        "date" => ("date", json!({ "dateFormat": { "name": "iso" } })),
        "timestamp with time zone" | "timestamp without time zone" => (
            "dateTime",
            json!({
                "dateFormat": { "name": "iso" },
                "timeFormat": { "name": "24hour" },
                "timeZone": "utc",
            }),
        ),
        _ => return json!({ "name": name, "type": "multilineText" }),
    };
    json!({ "name": name, "type": kind, "options": options })
}

async fn set_up(pool: &PgPool, token: &str, base_id: &str) -> anyhow::Result<()> {
    let columns: Vec<(String, String, String)> = sqlx::query_as(
        "SELECT table_name::text, column_name::text, data_type::text FROM information_schema.columns
         WHERE table_schema = 'public' AND table_name = ANY($1)
         ORDER BY table_name, column_name <> 'id', ordinal_position",
    )
    .bind(TABLES)
    .fetch_all(pool)
    .await?;
    let existing = tables(token, base_id).await?;
    for table in TABLES {
        let name = table_name(table);
        let fields: Vec<Value> = columns
            .iter()
            .filter(|(column_table, ..)| column_table == table)
            .map(|(_, column, data_type)| field(column, data_type))
            .collect();
        let changed = match existing.iter().find(|t| t.name == name || t.id == name) {
            None => {
                create_table(token, base_id, &name, &fields).await?;
                true
            }
            Some(existing) => {
                let missing: Vec<&Value> = fields
                    .iter()
                    .filter(|field| !existing.fields.iter().any(|f| field["name"] == f.name.as_str()))
                    .collect();
                for field in &missing {
                    create_field(token, base_id, &existing.id, field).await?;
                }
                !missing.is_empty()
            }
        };
        if changed {
            log::info!("Set up Airtable mirror table {name}, resyncing all of {table}");
            sqlx::query("DELETE FROM airtable_sync WHERE table_name = $1")
                .bind(table)
                .execute(pool)
                .await?;
        }
    }
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
            value @ Value::Object(_) => (key, Value::String(value.to_string())),
            value => (key, value),
        })
        .collect()
}

async fn sync(pool: &PgPool, token: &str, base_id: &str, table: &str) -> anyhow::Result<()> {
    let started: String = sqlx::query_scalar("SELECT now()::text")
        .fetch_one(pool)
        .await?;
    let rows: Vec<String> = sqlx::query_scalar(AssertSqlSafe(format!(
        "SELECT to_jsonb(t)::text FROM \"{table}\" t
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
            token: token.to_string(),
            base_id: base_id.to_string(),
            table_id_or_name: table_name(table),
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
