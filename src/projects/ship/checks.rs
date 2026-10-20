use super::{Draft, queue, repos, repos::Repo};
use crate::{
    accounts::{
        profile,
        tokens::{self, Provider},
    },
    error::{ApiError, reject},
    http,
    projects::{ensure_hackatime_free, review},
    snapshots,
};
use hc_helper::{
    airtable::AirtableTable,
    auth::data::{forget_auth_token, get_auth_data_with_token},
    hackatime::data::{
        HackatimeUser, forget_hackatime_token, get_hackatime_project_summaries_with_token,
        get_hackatime_user_with_token,
    },
};
use chrono::{DateTime, Utc};
use reqwest::Url;
use sqlx::{PgPool, types::Json};
use std::{
    collections::HashSet,
    sync::{LazyLock, Mutex, PoisonError},
};

static CLAIMED: LazyLock<Mutex<HashSet<String>>> = LazyLock::new(Default::default);

pub struct RepoClaim(String);

impl Drop for RepoClaim {
    fn drop(&mut self) {
        CLAIMED.lock().unwrap_or_else(PoisonError::into_inner).remove(&self.0);
    }
}

pub fn details(project: &Draft) -> Result<(), ApiError> {
    let missing: Vec<&str> = [
        ("name", project.name.trim().is_empty()),
        ("description", project.description.trim().is_empty()),
        ("repository link", project.repo.trim().is_empty()),
        ("demo link", project.demo.trim().is_empty()),
        ("screenshot", project.screenshot.is_none()),
        ("tier", project.tier.is_none()),
        ("Hackatime project", project.hackatime.is_empty()),
    ]
    .into_iter()
    .filter_map(|(field, missing)| missing.then_some(field))
    .collect();
    if !missing.is_empty() {
        return Err(reject(format!("Missing {}", missing.join(", "))));
    }
    Ok(())
}

pub struct Account {
    token: String,
    verified: bool,
}

pub async fn account(pool: &PgPool, user_id: i32) -> Result<Account, ApiError> {
    let sign_in = || reject("Couldn't load your Hack Club account. Try signing in again");
    let token = tokens::load(pool, user_id, Provider::Hca)
        .await?
        .ok_or_else(sign_in)?;
    forget_auth_token(&token);
    let data = get_auth_data_with_token(&token).await.map_err(|e| {
        log::warn!("loading HCA profile for ship failed: {e:#}");
        sign_in()
    })?;
    profile::sync(pool, &data).await?;
    let verified = match profile::ship_blocked(Some(&data.verification_status), data.ysws_eligible) {
        None => true,
        Some(_) if profile::verifying(&data.verification_status) => false,
        Some(reason) => return Err(reject(reason)),
    };
    if !profile::address(&data).is_some_and(|address| {
        [&address.line_1, &address.city, &address.country]
            .iter()
            .all(|field| !field.trim().is_empty())
    }) {
        return Err(reject(profile::NO_ADDRESS));
    }
    if data.birthday.trim().is_empty() {
        return Err(reject("Add your birthday to your Hack Club Auth account"));
    }
    Ok(Account { token, verified })
}

pub struct Hackatime {
    token: String,
    user: HackatimeUser,
    tracked: f64,
    hours: f64,
}

pub async fn hackatime(pool: &PgPool, user_id: i32, project: &Draft) -> Result<Hackatime, ApiError> {
    let names = &project.hackatime;
    ensure_hackatime_free(pool, user_id, project.id, names).await?;
    let token = tokens::load(pool, user_id, Provider::Hackatime)
        .await?
        .ok_or_else(|| reject("Link your Hackatime account first"))?;
    let unreachable = |e: anyhow::Error| {
        log::warn!("Hackatime lookup for ship failed: {e:#}");
        reject("Couldn't reach Hackatime. Please try again")
    };
    forget_hackatime_token(&token);
    let user = get_hackatime_user_with_token(&token).await.map_err(unreachable)?;
    sqlx::query("UPDATE \"user\" SET hackatime_trust_level = $2 WHERE id = $1")
        .bind(user_id)
        .bind(&user.trust_factor.trust_level)
        .execute(pool)
        .await?;
    if user.trust_factor.trust_level == "red" {
        return Err(reject("Your Hackatime account has been flagged, so you can't ship right now"));
    }
    let projects = get_hackatime_project_summaries_with_token(&token)
        .await
        .map_err(unreachable)?;
    let missing: Vec<&str> = names
        .iter()
        .filter(|name| !projects.iter().any(|project| &project.name == *name))
        .map(String::as_str)
        .collect();
    if !missing.is_empty() {
        return Err(reject(format!("Hackatime has no project called {}", missing.join(", "))));
    }
    let seconds: f64 = projects
        .iter()
        .filter(|project| names.contains(&project.name))
        .map(|project| project.total_duration_seconds)
        .sum();
    let tracked = (seconds / 36.0).round() / 100.0;
    let approved: Option<f64> = sqlx::query_scalar(
        "SELECT max(COALESCE(tracked_hours, hours)) FROM submission
         WHERE project_id = $1 AND deleted_at IS NULL AND NOT rejected AND review_status = $2",
    )
    .bind(project.id)
    .bind(review::APPROVED)
    .fetch_one(pool)
    .await?;
    let hours = ((tracked - approved.unwrap_or(0.0)) * 100.0).round() / 100.0;
    if hours <= 0.0 {
        return Err(reject("Log more time on this project since its last approved ship first"));
    }
    Ok(Hackatime { token, user, tracked, hours })
}

pub async fn readme(link: &str) -> Result<(Repo, bool), ApiError> {
    let repo = repos::parse(link)?;
    let found = match repos::has_readme(&repo).await {
        Ok(found) => found,
        Err(e) => {
            log::warn!("README check for {} failed: {e:#}", repo.url());
            false
        }
    };
    Ok((repo, found))
}

pub async fn not_shipped(pool: &PgPool, project_id: i32, link: &str) -> Result<RepoClaim, ApiError> {
    let repo = repos::parse(link).map_err(|_| reject("Fix the repository link first"))?;
    let root = repo.root();
    if !CLAIMED.lock().unwrap_or_else(PoisonError::into_inner).insert(root.clone()) {
        return Err(reject("Another project is shipping this repository right now"));
    }
    let claim = RepoClaim(root.clone());
    let others: Vec<String> = sqlx::query_scalar(
        "SELECT code_url FROM submission
         WHERE project_id <> $1 AND deleted_at IS NULL AND NOT rejected AND strpos(lower(code_url), $2) > 0",
    )
    .bind(project_id)
    .bind(&root)
    .fetch_all(pool)
    .await?;
    if others
        .iter()
        .any(|other| repos::parse(other).is_ok_and(|other| other.overlaps(&repo)))
    {
        return Err(reject("Another project has already shipped this repository"));
    }
    Ok(claim)
}

pub async fn demo(link: &str) -> Result<(), ApiError> {
    let link = link.trim();
    if link.is_empty() {
        return Err(reject("Add a demo link first"));
    }
    let url = Url::parse(link)
        .ok()
        .filter(http::is_public_url)
        .ok_or_else(|| reject("The demo link has to be a public http(s) URL"))?;
    let response = http::CLIENT.get(url).send().await.map_err(|e| {
        if e.is_timeout() {
            reject("The demo link timed out")
        } else {
            reject("Couldn't reach the demo link")
        }
    })?;
    if !response.status().is_success() {
        return Err(reject(format!(
            "The demo link returned {}. Make sure the page is public",
            response.status()
        )));
    }
    Ok(())
}

pub async fn submit(
    pool: &PgPool,
    project: &Draft,
    repo: &Repo,
    table: AirtableTable,
    account: Account,
    hackatime: Hackatime,
    shipped_at: DateTime<Utc>,
) -> Result<bool, ApiError> {
    let tier = project.tier.as_deref().unwrap_or_default();
    let mips_per_hour = review::mips_per_hour(Some(tier))?;
    review::tier_label(tier)?;
    let shared: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM snapshot s JOIN project p ON p.user_id = s.user_id
         WHERE p.id = $1 AND s.deleted_at IS NULL",
    )
    .bind(project.id)
    .fetch_one(pool)
    .await?;
    let code_url = repo.url();
    let queued = queue::Queued {
        demo: project.demo.trim().to_string(),
        screenshot: project.screenshot.clone().unwrap_or_default(),
        description: project.description.trim().to_string(),
        hackatime: project.hackatime.clone(),
        hours: hackatime.hours,
        hackatime_id: hackatime.user.user_id.to_string(),
        github_username: hackatime.user.github_username.filter(|name| !name.is_empty()),
    };
    let (record_id, queued) = if account.verified {
        let record_id = queue::send(
            table,
            &account.token,
            &hackatime.token,
            code_url.clone(),
            tier,
            shipped_at,
            queued,
        )
        .await?;
        (Some(record_id), None)
    } else {
        (None, Some(Json(queued)))
    };
    if let Err(e) = sqlx::query(
        "INSERT INTO submission (project_id, record_id, mips_per_hour, code_url, tier, multiplier, tracked_hours,
             created_at, queued)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
    )
    .bind(project.id)
    .bind(&record_id)
    .bind(mips_per_hour)
    .bind(code_url)
    .bind(tier)
    .bind(snapshots::multiplier(shared))
    .bind(hackatime.tracked)
    .bind(shipped_at)
    .bind(queued)
    .execute(pool)
    .await
    {
        if let Some(record_id) = record_id {
            log::error!(
                "Review record {record_id} was created for project {} but saving the submission failed",
                project.id
            );
        }
        return Err(e.into());
    }
    Ok(record_id.is_some())
}
