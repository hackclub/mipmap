use super::tokens::{self, Provider};
use crate::http;
use chrono::NaiveDate;
use hc_helper::auth::data::{Address, AuthData, forget_auth_token, get_auth_data_with_token};
use serde::Deserialize;
use sqlx::{PgExecutor, PgPool};

pub const INELIGIBLE: &str = "Your Hack Club account isn't eligible for YSWS prizes";
pub const NO_ADDRESS: &str = "Add a shipping address to your Hack Club account";

pub fn address(data: &AuthData) -> Option<Address> {
    data.primary_address().or_else(|| data.addresses.first().cloned())
}

pub async fn sync(executor: impl PgExecutor<'_>, data: &AuthData) -> anyhow::Result<i32> {
    anyhow::ensure!(!data.id.is_empty(), "HCA returned a profile without an id");
    let address = address(data).unwrap_or_default();
    let birthday = match data.birthday.trim() {
        "" => None,
        birthday => NaiveDate::parse_from_str(birthday, "%Y-%m-%d")
            .inspect_err(|e| log::warn!("HCA birthday of {} isn't a date: {e}", data.id))
            .ok(),
    };
    let id = sqlx::query_scalar(
        "INSERT INTO \"user\" (
             hca_id, first_name, last_name, email, verification_status, ysws_eligible,
             birthday, phone_number, slack_id,
             address_first_name, address_last_name, address_line_1, address_line_2, address_city,
             address_state, address_postal_code, address_country, address_phone_number
         ) VALUES (
             $1, NULLIF($2, ''), NULLIF($3, ''), NULLIF($4, ''), NULLIF($5, ''), $6,
             $7, NULLIF($8, ''), NULLIF($9, ''),
             NULLIF($10, ''), NULLIF($11, ''), NULLIF($12, ''), NULLIF($13, ''), NULLIF($14, ''),
             NULLIF($15, ''), NULLIF($16, ''), NULLIF($17, ''), NULLIF($18, '')
         )
         ON CONFLICT (hca_id) DO UPDATE SET
             first_name = EXCLUDED.first_name,
             last_name = EXCLUDED.last_name,
             email = EXCLUDED.email,
             verification_status = EXCLUDED.verification_status,
             ysws_eligible = EXCLUDED.ysws_eligible,
             birthday = EXCLUDED.birthday,
             phone_number = EXCLUDED.phone_number,
             slack_id = EXCLUDED.slack_id,
             address_first_name = EXCLUDED.address_first_name,
             address_last_name = EXCLUDED.address_last_name,
             address_line_1 = EXCLUDED.address_line_1,
             address_line_2 = EXCLUDED.address_line_2,
             address_city = EXCLUDED.address_city,
             address_state = EXCLUDED.address_state,
             address_postal_code = EXCLUDED.address_postal_code,
             address_country = EXCLUDED.address_country,
             address_phone_number = EXCLUDED.address_phone_number
         RETURNING id",
    )
    .bind(&data.id)
    .bind(&data.first_name)
    .bind(&data.last_name)
    .bind(&data.primary_email)
    .bind(&data.verification_status)
    .bind(data.ysws_eligible)
    .bind(birthday)
    .bind(&data.phone_number)
    .bind(&data.slack_id)
    .bind(&address.first_name)
    .bind(&address.last_name)
    .bind(&address.line_1)
    .bind(&address.line_2)
    .bind(&address.city)
    .bind(&address.state)
    .bind(&address.postal_code)
    .bind(&address.country)
    .bind(&address.phone_number)
    .fetch_one(executor)
    .await?;
    Ok(id)
}

pub async fn refresh(pool: &PgPool, user_id: i32) -> anyhow::Result<()> {
    let Some(token) = tokens::load(pool, user_id, Provider::Hca).await? else {
        return Ok(());
    };
    forget_auth_token(&token);
    sync(pool, &get_auth_data_with_token(&token).await?).await?;
    Ok(())
}

#[derive(Deserialize)]
struct Check {
    result: String,
}

pub async fn ysws_eligible(hca_id: &str) -> anyhow::Result<bool> {
    let check: Check = http::CLIENT
        .get("https://auth.hackclub.com/api/external/check")
        .query(&[("idv_id", hca_id)])
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    Ok(check.result == "verified_eligible")
}

pub fn shop_blocked(verification_status: Option<&str>, ysws_eligible: bool) -> Option<&'static str> {
    blocked(verification_status, ysws_eligible, "Verify your identity to unlock and purchase items")
}

pub fn ship_blocked(verification_status: Option<&str>, ysws_eligible: bool) -> Option<&'static str> {
    blocked(verification_status, ysws_eligible, "Verify your identity to ship projects")
}

fn blocked(
    verification_status: Option<&str>,
    ysws_eligible: bool,
    unverified: &'static str,
) -> Option<&'static str> {
    match verification_status {
        Some("verified" | "verified_eligible") if ysws_eligible => None,
        Some("verified" | "verified_eligible" | "verified_but_over_18") => {
            Some(INELIGIBLE)
        }
        Some("pending") => Some("Your identity verification is still pending"),
        Some("rejected") => Some("Your identity verification was rejected"),
        _ => Some(unverified),
    }
}
