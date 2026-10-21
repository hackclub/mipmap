use hc_helper::airtable::{AirtableTable, get_records_with_fields};
use serde::Deserialize;
use serde_json::Value;
use sqlx::{PgPool, types::Json};
use std::{
    collections::{BTreeSet, HashMap},
    sync::{LazyLock, Mutex, PoisonError},
};

const JUSTIFICATION: &str = "Justification - Hackatime Project Name(s) + Date Range(s)";
const RANGE: &str = " 0000-00-00-0000-00-00";

static NAMES: LazyLock<Mutex<HashMap<i32, BTreeSet<String>>>> = LazyLock::new(Default::default);

#[derive(Deserialize)]
struct Record {
    #[serde(rename = "Justification - Hackatime Project Name(s) + Date Range(s)")]
    justification: Option<String>,
}

#[derive(sqlx::FromRow)]
struct Row {
    id: i32,
    live: bool,
    record_id: Option<String>,
    queued: Option<Json<Value>>,
}

pub struct Shipped {
    pub id: i32,
    pub live: bool,
    pub names: BTreeSet<String>,
}

pub async fn shipped(pool: &PgPool, table: &AirtableTable, user_id: i32) -> anyhow::Result<Vec<Shipped>> {
    let rows: Vec<Row> = sqlx::query_as(
        "SELECT s.id, s.deleted_at IS NULL AS live, s.record_id, s.queued
         FROM submission s JOIN project p ON p.id = s.project_id
         WHERE p.user_id = $1 ORDER BY s.id",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;
    let missing: Vec<(i32, String)> = {
        let mut cache = NAMES.lock().unwrap_or_else(PoisonError::into_inner);
        let mut missing = Vec::new();
        for row in &rows {
            if cache.contains_key(&row.id) {
                continue;
            }
            match (&row.record_id, &row.queued) {
                (Some(record_id), _) => missing.push((row.id, record_id.clone())),
                (None, Some(queued)) => {
                    let names = queued["hackatime"]
                        .as_array()
                        .into_iter()
                        .flatten()
                        .filter_map(|name| name.as_str().map(str::to_string))
                        .collect();
                    cache.insert(row.id, names);
                }
                (None, None) => {
                    cache.insert(row.id, BTreeSet::new());
                }
            }
        }
        missing
    };
    if !missing.is_empty() {
        let ids: Vec<String> = missing.iter().map(|(_, record_id)| record_id.clone()).collect();
        let records = get_records_with_fields::<Record>(table.clone(), &ids, &[JUSTIFICATION]).await?;
        let mut cache = NAMES.lock().unwrap_or_else(PoisonError::into_inner);
        for (id, record_id) in missing {
            let names = records
                .iter()
                .find(|(found, _)| *found == record_id)
                .and_then(|(_, record)| record.justification.as_deref())
                .map(parse)
                .unwrap_or_default();
            cache.insert(id, names);
        }
    }
    let cache = NAMES.lock().unwrap_or_else(PoisonError::into_inner);
    Ok(rows
        .into_iter()
        .map(|row| Shipped { id: row.id, live: row.live, names: cache.get(&row.id).cloned().unwrap_or_default() })
        .collect())
}

fn parse(justification: &str) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    let mut current = String::new();
    for part in justification.split(", ") {
        if !current.is_empty() {
            current.push_str(", ");
        }
        current.push_str(part);
        if let Some(name) = strip_range(&current) {
            names.insert(name.trim().to_string());
            current.clear();
        }
    }
    names
}

fn strip_range(entry: &str) -> Option<&str> {
    let split = entry.len().checked_sub(RANGE.len())?;
    let range = entry.get(split..)?;
    range
        .bytes()
        .zip(RANGE.bytes())
        .all(|(byte, pattern)| if pattern == b'0' { byte.is_ascii_digit() } else { byte == pattern })
        .then(|| &entry[..split])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_are_parsed_from_the_justification() {
        let names = parse("mipmap 2026-09-01-2026-10-02, odd, name 2026-01-01-2026-01-05, x 2026-02-03-2026-02-03");
        assert_eq!(
            names,
            BTreeSet::from(["mipmap".to_string(), "odd, name".to_string(), "x".to_string()])
        );
        assert!(parse("").is_empty());
        assert!(parse("no range here").is_empty());
    }
}
