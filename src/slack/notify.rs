use super::{RECIPIENTS, messages};
use crate::{
    accounts::{self, hackatime::Totals},
    env,
    projects::Project,
    snapshots,
};
use chrono::{DateTime, Datelike, Duration, NaiveDate, NaiveDateTime, NaiveTime, Utc, Weekday};
use serde::Deserialize;
use sqlx::{PgExecutor, PgPool};
use std::sync::LazyLock;

const CHALLENGES_JSON: &str = include_str!("../../frontend/src/lib/assets/challenges.json");
const ROUNDS_JSON: &str = include_str!("../../frontend/src/lib/assets/fulfillment_rounds.json");
const HIDDEN_ROUNDS: usize = 2;
const ROUND_NOTICE_DAYS: i64 = 3;

#[derive(Deserialize)]
struct Challenge {
    name: String,
    description: String,
    start: String,
    end: String,
}

#[derive(Deserialize)]
struct Round {
    ship_by: NaiveDate,
}

static CHALLENGES: LazyLock<Vec<Challenge>> =
    LazyLock::new(|| serde_json::from_str(CHALLENGES_JSON).expect("challenges json must be valid"));

static ROUNDS: LazyLock<Vec<Round>> = LazyLock::new(|| {
    let mut rounds: Vec<Round> = serde_json::from_str(ROUNDS_JSON).expect("rounds json must be valid");
    rounds.truncate(rounds.len().saturating_sub(HIDDEN_ROUNDS));
    rounds
});

pub fn spawn(pool: PgPool) {
    actix_web::rt::spawn(async move {
        let mut interval = env::interval_secs("SLACK_NOTIFY_INTERVAL_SECS", 600);
        loop {
            interval.tick().await;
            if let Err(e) = notify(&pool).await {
                log::error!("Slack notifications failed: {e:#}");
            }
        }
    });
}

async fn notify(pool: &PgPool) -> anyhow::Result<()> {
    let now = Utc::now();
    for challenge in CHALLENGES.iter() {
        let (Some(start), Some(end)) = (eastern(&challenge.start, false), eastern(&challenge.end, true)) else {
            log::error!("Challenge {} has an unreadable start or end", challenge.name);
            continue;
        };
        if start <= now && now < end {
            let text = messages::challenge(&challenge.name, &challenge.description);
            draft(pool, "challenge", &format!("challenge:{}", challenge.name), &text).await?;
        }
    }
    for round in ROUNDS.iter() {
        let notice = eastern_at(round.ship_by - Duration::days(ROUND_NOTICE_DAYS), NaiveTime::MIN);
        if notice <= now && now < notice + Duration::days(1) {
            let key = format!("round:{}", round.ship_by);
            draft(pool, "round", &key, &messages::round(round.ship_by)).await?;
        }
    }
    ship_updates(pool).await?;
    inactivity(pool).await?;
    Ok(())
}

async fn draft(pool: &PgPool, kind: &str, key: &str, text: &str) -> anyhow::Result<()> {
    let mut tx = pool.begin().await?;
    let created = sqlx::query(
        "INSERT INTO slack_announcement (kind, text, dedupe_key)
         SELECT $1, $2, $3 WHERE NOT EXISTS (SELECT 1 FROM slack_announcement WHERE dedupe_key = $3)
         ON CONFLICT (dedupe_key) DO NOTHING",
    )
    .bind(kind)
    .bind(text)
    .bind(key)
    .execute(&mut *tx)
    .await?
    .rows_affected()
        > 0;
    if !created {
        return Ok(());
    }
    match env::var("SLACK_ADMIN_ID") {
        Some(admin) => {
            sqlx::query(
                "INSERT INTO slack_message (slack_id, user_id, kind, text, dedupe_key)
                 VALUES ($1, (SELECT id FROM \"user\" WHERE slack_id = $1 ORDER BY deleted_at IS NOT NULL, id LIMIT 1),
                     'admin', $2, $3)
                 ON CONFLICT (dedupe_key) DO NOTHING",
            )
            .bind(&admin)
            .bind(messages::draft_waiting(kind, &accounts::site_url("/admin")))
            .bind(format!("admin:{key}"))
            .execute(&mut *tx)
            .await?;
        }
        None => log::warn!("SLACK_ADMIN_ID not set, nobody was told about the new {kind} Slack message"),
    }
    tx.commit().await?;
    Ok(())
}

async fn ship_updates(pool: &PgPool) -> sqlx::Result<()> {
    let mut tx = pool.begin().await?;
    let changed: Vec<(i32, String, String, i32, i32, Option<String>)> = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "WITH changed AS (
             UPDATE submission s SET status_notified = CASE
                 WHEN s.rejected THEN 'rejected'
                 WHEN s.approved_at IS NOT NULL AND s.hours IS NOT NULL THEN 'approved' END
             WHERE s.deleted_at IS NULL AND s.status_notified IS DISTINCT FROM CASE
                 WHEN s.rejected THEN 'rejected'
                 WHEN s.approved_at IS NOT NULL AND s.hours IS NOT NULL THEN 'approved' END
             RETURNING s.id, s.project_id, s.status_notified, s.mips
         )
         SELECT c.id, c.status_notified, pr.name, c.mips, u.id,
             CASE WHEN {RECIPIENTS} AND p.ship_updates THEN u.slack_id END
         FROM changed c JOIN project pr ON pr.id = c.project_id JOIN \"user\" u ON u.id = pr.user_id
         JOIN slack_preference p ON p.user_id = u.id
         WHERE c.status_notified IS NOT NULL"
    )))
    .fetch_all(&mut *tx)
    .await?;
    for (id, status, project, mips, user_id, slack_id) in changed {
        let Some(slack_id) = slack_id else { continue };
        let text = match status.as_str() {
            "approved" => messages::ship_approved(&project, mips),
            _ => messages::ship_rejected(&project),
        };
        sqlx::query(
            "INSERT INTO slack_message (slack_id, user_id, kind, text, dedupe_key) VALUES ($1, $2, 'ship', $3, $4)
             ON CONFLICT (dedupe_key) DO NOTHING",
        )
        .bind(&slack_id)
        .bind(user_id)
        .bind(text)
        .bind(format!("ship:{id}:{status}:{slack_id}"))
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await
}

async fn inactivity(pool: &PgPool) -> sqlx::Result<()> {
    sqlx::query(sqlx::AssertSqlSafe(format!(
        "INSERT INTO slack_message (slack_id, user_id, kind, text, dedupe_key)
         SELECT u.slack_id, u.id, 'inactivity', $1,
             'inactivity:' || u.slack_id || ':' || floor(extract(epoch FROM a.last_active))::bigint
         FROM \"user\" u JOIN slack_preference p ON p.user_id = u.id
         CROSS JOIN LATERAL (
             SELECT GREATEST(max(pr.last_logged_at), u.hackatime_linked_at) AS last_active FROM project pr
             WHERE pr.user_id = u.id AND pr.deleted_at IS NULL AND pr.hackatime_projects <> '{{}}'
             HAVING count(*) > 0
         ) a
         WHERE {RECIPIENTS} AND p.inactivity_reminders AND u.hackatime_linked_at IS NOT NULL
         AND a.last_active < now() - interval '7 days'
         ON CONFLICT (dedupe_key) DO NOTHING"
    )))
    .bind(messages::inactivity())
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn snapshot_reminder(pool: &PgPool, user_id: i32, totals: &Totals, projects: &[Project]) -> sqlx::Result<()> {
    if super::token().is_none() {
        return Ok(());
    }
    let slack_id: Option<String> = sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
        "SELECT u.slack_id FROM \"user\" u JOIN slack_preference p ON p.user_id = u.id
         WHERE u.id = $1 AND {RECIPIENTS} AND p.snapshot_reminders"
    )))
    .bind(user_id)
    .fetch_optional(pool)
    .await?;
    let Some(slack_id) = slack_id else {
        return Ok(());
    };
    if !snapshots::ready(pool, user_id, totals, projects).await? {
        return Ok(());
    }
    sqlx::query(
        "INSERT INTO slack_message (slack_id, user_id, kind, text)
         SELECT $1, $2, 'snapshot', $3 WHERE NOT EXISTS (
             SELECT 1 FROM slack_message WHERE slack_id = $1 AND kind = 'snapshot'
             AND created_at > now() - interval '23 hours'
         )",
    )
    .bind(&slack_id)
    .bind(user_id)
    .bind(messages::snapshot())
    .execute(pool)
    .await?;
    Ok(())
}

pub fn templates() -> Vec<(&'static str, String)> {
    let now = Utc::now();
    let today = now.date_naive();
    let challenge = CHALLENGES
        .iter()
        .find(|c| eastern(&c.end, true).is_some_and(|end| now < end))
        .or(CHALLENGES.last());
    let round = ROUNDS.iter().find(|r| r.ship_by >= today).or(ROUNDS.last());
    let mut templates = vec![("Welcome", messages::welcome())];
    if let Some(challenge) = challenge {
        templates.push(("New challenge", messages::challenge(&challenge.name, &challenge.description)));
    }
    if let Some(round) = round {
        templates.push(("Fulfillment round", messages::round(round.ship_by)));
    }
    templates.extend([
        ("Snapshot reminder", messages::snapshot()),
        ("Ship approved", messages::ship_approved("Example Project", 100)),
        ("Ship rejected", messages::ship_rejected("Example Project")),
        ("Inactivity reminder", messages::inactivity()),
        ("Admin alert", messages::draft_waiting("challenge", &accounts::site_url("/admin"))),
    ]);
    templates
}

pub async fn welcome(executor: impl PgExecutor<'_>, user_id: i32) -> sqlx::Result<()> {
    if super::token().is_none() {
        return Ok(());
    }
    sqlx::query(sqlx::AssertSqlSafe(format!(
        "INSERT INTO slack_message (slack_id, user_id, kind, text, dedupe_key)
         SELECT u.slack_id, u.id, 'welcome', $2, 'welcome:' || u.id
         FROM \"user\" u JOIN slack_preference p ON p.user_id = u.id
         WHERE u.id = $1 AND {RECIPIENTS}
         ON CONFLICT (dedupe_key) DO NOTHING"
    )))
    .bind(user_id)
    .bind(messages::welcome())
    .execute(executor)
    .await?;
    Ok(())
}

fn eastern(value: &str, end: bool) -> Option<DateTime<Utc>> {
    if let Ok(local) = NaiveDateTime::parse_from_str(value, "%Y-%m-%dT%H:%M") {
        return Some(eastern_at(local.date(), local.time()));
    }
    let date = NaiveDate::parse_from_str(value, "%Y-%m-%d").ok()?;
    Some(eastern_at(if end { date.succ_opt()? } else { date }, NaiveTime::MIN))
}

fn eastern_at(date: NaiveDate, time: NaiveTime) -> DateTime<Utc> {
    let local = date.and_time(time);
    let sunday = |month, n| {
        NaiveDate::from_weekday_of_month_opt(date.year(), month, Weekday::Sun, n)
            .expect("every month has a second Sunday")
            .and_hms_opt(2, 0, 0)
            .expect("2:00 is a valid time")
    };
    let daylight = sunday(3, 2) <= local && local < sunday(11, 1);
    (local + Duration::hours(if daylight { 4 } else { 5 })).and_utc()
}

#[cfg(test)]
mod tests {
    use super::eastern;

    #[test]
    fn converts_eastern_wall_time() {
        assert_eq!(eastern("2026-10-15T12:00", false).unwrap().to_rfc3339(), "2026-10-15T16:00:00+00:00");
        assert_eq!(eastern("2026-12-01", true).unwrap().to_rfc3339(), "2026-12-02T05:00:00+00:00");
    }
}
