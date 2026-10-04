use super::{Draft, repos, repos::Repo};
use crate::{
    accounts::{
        hackatime::Totals,
        profile,
        tokens::{self, Provider},
    },
    error::{ApiError, reject},
    http,
    projects::{ensure_hackatime_free, review},
};
use hc_helper::{
    airtable::{AirtableTable, get_records},
    auth::data::get_auth_data_with_token,
    hackatime::data::{
        HackatimeUser, get_hackatime_project_summaries_with_token, get_hackatime_user_with_token,
    },
    submission::create_unified_with_token,
};
use reqwest::Url;
use sqlx::{PgPool, types::Json};

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
        return Err(reject(format!("Missing {}.", missing.join(", "))));
    }
    Ok(())
}

pub async fn account(pool: &PgPool, user_id: i32) -> Result<String, ApiError> {
    let sign_in = || reject("Couldn't load your Hack Club account. Try signing in again.");
    let token = tokens::load(pool, user_id, Provider::Hca)
        .await?
        .ok_or_else(sign_in)?;
    let data = get_auth_data_with_token(&token).await.map_err(|e| {
        log::warn!("loading HCA profile for ship failed: {e:#}");
        sign_in()
    })?;
    profile::sync(pool, &data).await?;
    if let Some(reason) = profile::blocked_reason(Some(&data.verification_status), data.ysws_eligible) {
        return Err(reject(reason));
    }
    if !profile::address(&data).is_some_and(|address| {
        [&address.line_1, &address.city, &address.country]
            .iter()
            .all(|field| !field.trim().is_empty())
    }) {
        return Err(reject(profile::NO_ADDRESS));
    }
    if data.birthday.trim().is_empty() {
        return Err(reject("Add your birthday to your Hack Club account."));
    }
    Ok(token)
}

pub struct Hackatime {
    token: String,
    user: HackatimeUser,
    seconds: f64,
}

pub async fn hackatime(pool: &PgPool, user_id: i32, project: &Draft) -> Result<Hackatime, ApiError> {
    let names = &project.hackatime;
    ensure_hackatime_free(pool, user_id, project.id, names).await?;
    let token = tokens::load(pool, user_id, Provider::Hackatime)
        .await?
        .ok_or_else(|| reject("Link your Hackatime account first."))?;
    let unreachable = |e: anyhow::Error| {
        log::warn!("Hackatime lookup for ship failed: {e:#}");
        reject("Couldn't reach Hackatime. Please try again.")
    };
    let user = get_hackatime_user_with_token(&token).await.map_err(unreachable)?;
    sqlx::query("UPDATE \"user\" SET hackatime_trust_level = $2 WHERE id = $1")
        .bind(user_id)
        .bind(&user.trust_factor.trust_level)
        .execute(pool)
        .await?;
    if user.trust_factor.trust_level == "red" {
        return Err(reject("Your Hackatime account has been flagged, so you can't ship right now."));
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
        return Err(reject(format!("Hackatime has no project called {}.", missing.join(", "))));
    }
    let baseline: Option<Json<Totals>> =
        sqlx::query_scalar("SELECT hackatime_baseline FROM \"user\" WHERE id = $1")
            .bind(user_id)
            .fetch_one(pool)
            .await?;
    let baseline = baseline.map(|Json(totals)| totals).unwrap_or_default();
    let seconds = projects
        .iter()
        .filter(|project| names.contains(&project.name))
        .map(|project| {
            let before = baseline.get(&project.name).copied().unwrap_or(0) as f64;
            (project.total_duration_seconds - before).max(0.0)
        })
        .sum();
    Ok(Hackatime { token, user, seconds })
}

pub async fn readme(link: &str) -> Result<Repo, ApiError> {
    let repo = repos::parse(link)?;
    if !repos::has_readme(&repo).await? {
        return Err(reject(
            "Couldn't find a README. Make sure the repository is public and has one.",
        ));
    }
    Ok(repo)
}

pub async fn not_shipped(pool: &PgPool, project_id: i32, link: &str) -> Result<(), ApiError> {
    let key = repos::parse(link)
        .map_err(|_| reject("Fix the repository link first."))?
        .key();
    let others: Vec<String> = sqlx::query_scalar(
        "SELECT code_url FROM submission
         WHERE project_id <> $1 AND deleted_at IS NULL AND strpos(lower(code_url), $2) > 0",
    )
    .bind(project_id)
    .bind(&key)
    .fetch_all(pool)
    .await?;
    if others
        .iter()
        .any(|other| repos::parse(other).is_ok_and(|other| other.key() == key))
    {
        return Err(reject("Another project has already shipped this repository."));
    }
    Ok(())
}

pub async fn demo(link: &str) -> Result<(), ApiError> {
    let link = link.trim();
    if link.is_empty() {
        return Err(reject("Add a demo link first."));
    }
    let url = Url::parse(link)
        .ok()
        .filter(http::is_public_url)
        .ok_or_else(|| reject("The demo link has to be a public http(s) URL."))?;
    let response = http::CLIENT.get(url).send().await.map_err(|e| {
        if e.is_timeout() {
            reject("The demo link timed out.")
        } else {
            reject("Couldn't reach the demo link.")
        }
    })?;
    if !response.status().is_success() {
        return Err(reject(format!("The demo link returned {}.", response.status())));
    }
    Ok(())
}

pub async fn submit(
    pool: &PgPool,
    project: &Draft,
    repo: &Repo,
    table: AirtableTable,
    hca_token: &str,
    hackatime: Hackatime,
) -> Result<(), ApiError> {
    let code_url = repo.url();
    let record_id = create_unified_with_token(
        hca_token,
        &hackatime.token,
        table.clone(),
        code_url.clone(),
        project.demo.trim().to_string(),
        project.screenshot.clone().unwrap_or_default(),
        project.description.trim().to_string(),
        project.hackatime.clone(),
        review::Extras {
            hours: (hackatime.seconds / 36.0).round() / 100.0,
            hackatime_id: hackatime.user.user_id.to_string(),
            github_username: hackatime.user.github_username.filter(|name| !name.is_empty()),
        },
    )
    .await?;
    let found = get_records::<serde::de::IgnoredAny>(table, std::slice::from_ref(&record_id)).await?;
    if !found.iter().any(|(id, _)| *id == record_id) {
        return Err(anyhow::anyhow!("review record {record_id} is missing after creating it").into());
    }
    sqlx::query(
        "INSERT INTO submission (project_id, record_id, mips_per_hour, code_url)
         VALUES ($1, $2, $3, $4)",
    )
    .bind(project.id)
    .bind(record_id)
    .bind(review::mips_per_hour(project.tier.as_deref()))
    .bind(code_url)
    .execute(pool)
    .await?;
    Ok(())
}
