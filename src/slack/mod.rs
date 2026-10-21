use crate::{accounts, env, http};
use anyhow::Context;
use serde::Deserialize;
use serde_json::json;
use sqlx::PgPool;
use std::{collections::HashMap, sync::LazyLock};

mod admin;
mod incoming;
mod messages;
mod notify;
mod preferences;
mod queue;

pub use admin::routes as admin_routes;
pub use incoming::events;
pub use notify::{snapshot_reminder, welcome};
pub use preferences::{pages, routes};

pub const TEXT_LIMIT: usize = 3000;
const CHANNELS_JSON: &str = include_str!("../../frontend/src/lib/assets/slack_channels.json");
const RECIPIENTS: &str =
    "u.slack_id IS NOT NULL AND u.deleted_at IS NULL AND NOT u.is_banned AND p.enabled";

static CHANNELS: LazyLock<HashMap<String, String>> =
    LazyLock::new(|| serde_json::from_str(CHANNELS_JSON).expect("slack channels json must be valid"));

fn token() -> Option<String> {
    env::var("SLACK_BOT_TOKEN")
}

pub fn spawn(pool: PgPool) {
    if token().is_none() {
        log::warn!("SLACK_BOT_TOKEN not set, Slack DMs disabled");
        return;
    }
    queue::spawn(pool.clone());
    notify::spawn(pool);
}

pub const UNCONFIRMED: &str = "Slack didn't confirm it, so it may have been sent and wasn't retried";

pub enum SendError {
    RateLimited(u64),
    Unavailable(String),
    Down(anyhow::Error),
    Failed(anyhow::Error),
    Rejected(String),
    Unconfirmed(anyhow::Error),
}

#[derive(Deserialize)]
struct Posted {
    ok: bool,
    ts: Option<String>,
    error: Option<String>,
}

pub async fn post(slack_id: &str, text: &str, footer: bool) -> Result<String, SendError> {
    let token = token().ok_or_else(|| SendError::Unavailable("SLACK_BOT_TOKEN isn't set".into()))?;
    let text = mrkdwn(text);
    let mut blocks = vec![json!({ "type": "section", "text": { "type": "mrkdwn", "text": text } })];
    if footer {
        let footer = format!(
            "<{}|Manage these DMs> · <{}|Unsubscribe>",
            accounts::site_url("/notifications"),
            accounts::site_url("/notifications?unsubscribe")
        );
        blocks.push(json!({ "type": "context", "elements": [{ "type": "mrkdwn", "text": footer }] }));
    }
    let response = http::CLIENT
        .post("https://slack.com/api/chat.postMessage")
        .bearer_auth(token)
        .json(&json!({
            "channel": slack_id,
            "text": text,
            "blocks": blocks,
            "unfurl_links": false,
            "unfurl_media": false,
        }))
        .send()
        .await
        .map_err(|e| {
            if e.is_connect() {
                SendError::Down(e.into())
            } else {
                SendError::Unconfirmed(e.into())
            }
        })?;
    let status = response.status();
    if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
        let wait = response
            .headers()
            .get(reqwest::header::RETRY_AFTER)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.parse().ok())
            .unwrap_or(30);
        return Err(SendError::RateLimited(wait));
    }
    if status.is_server_error() {
        return Err(SendError::Unconfirmed(anyhow::anyhow!("Slack returned HTTP {status}")));
    }
    let posted: Posted = response
        .error_for_status()
        .map_err(|e| SendError::Failed(e.into()))?
        .json()
        .await
        .map_err(|e| SendError::Unconfirmed(e.into()))?;
    match (posted.ok, posted.ts, posted.error) {
        (true, Some(ts), _) => Ok(ts),
        (true, None, _) => Err(SendError::Unconfirmed(anyhow::anyhow!("Slack didn't return a timestamp"))),
        (_, _, Some(error))
            if matches!(
                error.as_str(),
                "not_authed" | "invalid_auth" | "account_inactive" | "token_revoked" | "token_expired"
                    | "missing_scope"
            ) =>
        {
            Err(SendError::Unavailable(error))
        }
        (_, _, Some(error)) if matches!(error.as_str(), "fatal_error" | "internal_error") => {
            Err(SendError::Unconfirmed(anyhow::anyhow!("Slack returned {error}")))
        }
        (_, _, Some(error)) if matches!(error.as_str(), "service_unavailable" | "request_timeout") => {
            Err(SendError::Failed(anyhow::anyhow!("Slack returned {error}")))
        }
        (_, _, error) => Err(SendError::Rejected(error.unwrap_or_else(|| "unknown_error".into()))),
    }
}

pub async fn invite(slack_id: &str, channel: &str) -> anyhow::Result<()> {
    let token = token().context("SLACK_BOT_TOKEN isn't set")?;
    let channel = CHANNELS.get(channel).with_context(|| format!("No Slack channel named {channel}"))?;
    let invited: Posted = http::CLIENT
        .post("https://slack.com/api/conversations.invite")
        .bearer_auth(token)
        .json(&json!({ "channel": channel, "users": slack_id }))
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    match invited.error.as_deref() {
        _ if invited.ok => Ok(()),
        Some("already_in_channel") => Ok(()),
        error => anyhow::bail!("Slack returned {}", error.unwrap_or("unknown_error")),
    }
}

fn control(text: &str) -> Option<usize> {
    let end = text[1..].find(['<', '>', '\n'])? + 1;
    let inner = &text[1..end];
    (text[end..].starts_with('>')
        && ["http://", "https://", "mailto:", "@", "#", "!"]
            .iter()
            .any(|prefix| inner.starts_with(prefix)))
    .then_some(end + 1)
}

fn code(text: &str) -> Option<usize> {
    if let Some(rest) = text.strip_prefix("```") {
        return rest.find("```").map(|end| end + 6);
    }
    let end = text[1..].find(['`', '\n'])? + 1;
    text[end..].starts_with('`').then_some(end + 1)
}

fn channel(text: &str) -> Option<(usize, &'static str)> {
    let name = &text[1..];
    let name = &name[..name
        .find(|c: char| !(c.is_ascii_alphanumeric() || c == '-' || c == '_'))
        .unwrap_or(name.len())];
    let id = CHANNELS.get(&name.to_ascii_lowercase())?;
    Some((name.len() + 1, id))
}

fn channels(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    let mut prev: Option<char> = None;
    while let Some(c) = rest.chars().next() {
        let skip = match c {
            '<' => control(rest),
            '`' => code(rest),
            _ => None,
        };
        if let Some(len) = skip {
            out.push_str(&rest[..len]);
            prev = rest[..len].chars().last();
            rest = &rest[len..];
            continue;
        }
        if c == '#'
            && !prev.is_some_and(|p| p.is_alphanumeric() || "_-&/#:".contains(p))
            && let Some((len, id)) = channel(rest)
        {
            out.push_str(&format!("<#{id}>"));
            prev = Some('>');
            rest = &rest[len..];
            continue;
        }
        out.push(c);
        prev = Some(c);
        rest = &rest[c.len_utf8()..];
    }
    out
}

pub fn mrkdwn(text: &str) -> String {
    escape(&channels(text))
}

pub fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    let mut line_start = true;
    while let Some(c) = rest.chars().next() {
        if c == '<'
            && let Some(len) = control(rest)
        {
            out.push_str(&rest[..len]);
            rest = &rest[len..];
            line_start = false;
            continue;
        }
        match c {
            '<' => out.push_str("&lt;"),
            '>' if line_start => out.push('>'),
            '>' => out.push_str("&gt;"),
            '&' if ["&amp;", "&lt;", "&gt;"].iter().any(|entity| rest.starts_with(entity)) => out.push('&'),
            '&' => out.push_str("&amp;"),
            c => out.push(c),
        }
        line_start = c == '\n' || (line_start && c == '>');
        rest = &rest[c.len_utf8()..];
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{escape, mrkdwn};

    #[test]
    fn escapes_outside_control_sequences() {
        assert_eq!(escape("a < b & c > d"), "a &lt; b &amp; c &gt; d");
        assert_eq!(escape("<https://x.com|X> <@U123> <!here>"), "<https://x.com|X> <@U123> <!here>");
        assert_eq!(escape("> quote\n>>> rest"), "> quote\n>>> rest");
        assert_eq!(escape("&amp; <nope>"), "&amp; &lt;nope&gt;");
    }

    #[test]
    fn links_known_channels() {
        assert_eq!(mrkdwn("check out #mipmap!"), "check out <#C0C2UMX4S9E>!");
        assert_eq!(mrkdwn("#Mipmap, #mipmap-help #other"), "<#C0C2UMX4S9E>, #mipmap-help #other");
        assert_eq!(mrkdwn("a#mipmap `#mipmap` ```\n#mipmap```"), "a#mipmap `#mipmap` ```\n#mipmap```");
        assert_eq!(mrkdwn("<https://x.com/#mipmap|#mipmap> (#mipmap)"), "<https://x.com/#mipmap|#mipmap> (<#C0C2UMX4S9E>)");
        assert_eq!(mrkdwn("``` #mipmap"), "``` <#C0C2UMX4S9E>");
    }
}
