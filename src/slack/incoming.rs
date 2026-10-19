use crate::{env, error::ApiError};
use actix_web::{HttpRequest, HttpResponse, post, web};
use hmac::{Hmac, KeyInit, Mac};
use serde::{Deserialize, de::IgnoredAny};
use serde_json::json;
use sha2::Sha256;
use sqlx::PgPool;

const MAX_AGE_SECS: i64 = 5 * 60;

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum Payload {
    UrlVerification {
        challenge: String,
    },
    EventCallback {
        event: Event,
    },
    #[serde(other)]
    Other,
}

#[derive(Deserialize)]
struct Event {
    #[serde(rename = "type")]
    kind: String,
    channel_type: Option<String>,
    subtype: Option<String>,
    bot_id: Option<String>,
    user: Option<String>,
    text: Option<String>,
    files: Option<Vec<IgnoredAny>>,
    ts: Option<String>,
}

fn verified(req: &HttpRequest, body: &[u8], secret: &str) -> bool {
    let header = |name| req.headers().get(name).and_then(|value| value.to_str().ok());
    let (Some(timestamp), Some(signature)) = (header("X-Slack-Request-Timestamp"), header("X-Slack-Signature"))
    else {
        return false;
    };
    let fresh = timestamp
        .parse::<i64>()
        .is_ok_and(|sent| (chrono::Utc::now().timestamp() - sent).abs() <= MAX_AGE_SECS);
    let Some(signature) = signature.strip_prefix("v0=").and_then(|hex| hex::decode(hex).ok()) else {
        return false;
    };
    let Ok(mut mac) = Hmac::<Sha256>::new_from_slice(secret.as_bytes()) else {
        return false;
    };
    mac.update(b"v0:");
    mac.update(timestamp.as_bytes());
    mac.update(b":");
    mac.update(body);
    fresh && mac.verify_slice(&signature).is_ok()
}

#[post("/slack/events")]
pub async fn events(req: HttpRequest, pool: web::Data<PgPool>, body: web::Bytes) -> Result<HttpResponse, ApiError> {
    let Some(secret) = env::var("SLACK_SIGNING_SECRET") else {
        return Ok(HttpResponse::NotFound().finish());
    };
    if !verified(&req, &body, &secret) {
        return Ok(HttpResponse::Unauthorized().finish());
    }
    let Ok(payload) = serde_json::from_slice::<Payload>(&body) else {
        return Ok(HttpResponse::BadRequest().finish());
    };
    match payload {
        Payload::UrlVerification { challenge } => Ok(HttpResponse::Ok().json(json!({ "challenge": challenge }))),
        Payload::EventCallback {
            event:
                Event {
                    kind,
                    channel_type: Some(channel_type),
                    subtype,
                    bot_id: None,
                    user: Some(slack_id),
                    text,
                    files,
                    ts: Some(ts),
                },
        } if kind == "message" && channel_type == "im" && matches!(subtype.as_deref(), None | Some("file_share")) => {
            let mut text = text.unwrap_or_default();
            match files.map_or(0, |files| files.len()) {
                0 => {}
                1 => text.push_str("\n_Sent a file_"),
                count => text.push_str(&format!("\n_Sent {count} files_")),
            }
            sqlx::query(
                "INSERT INTO slack_message (slack_id, user_id, incoming, kind, text, ts)
                 VALUES ($1, (SELECT id FROM \"user\" WHERE slack_id = $1 ORDER BY deleted_at IS NOT NULL, id LIMIT 1),
                     true, 'message', $2, $3)
                 ON CONFLICT (slack_id, ts) DO NOTHING",
            )
            .bind(slack_id)
            .bind(text.trim())
            .bind(ts)
            .execute(pool.get_ref())
            .await?;
            Ok(HttpResponse::Ok().finish())
        }
        _ => Ok(HttpResponse::Ok().finish()),
    }
}
