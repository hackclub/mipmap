use crate::{
    admin::{Admin, record_view},
    error::ApiError,
};
use actix_web::{HttpResponse, get, web};
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

const CITIES: i64 = 50;
const PEOPLE: i64 = 1000;
const COUNTRY: &str = "NULLIF(upper(trim(u.address_country)), '')";

pub fn admin_routes(cfg: &mut web::ServiceConfig) {
    cfg.service((get_locations, get_people));
}

#[derive(Serialize, sqlx::FromRow)]
struct Country {
    code: String,
    users: i64,
    onboarded: i64,
    shipped: i64,
    hours: f64,
    mips: i64,
}

#[derive(Serialize, sqlx::FromRow)]
struct Place {
    country: String,
    name: String,
    users: i64,
}

#[derive(Serialize, sqlx::FromRow)]
struct Week {
    week: NaiveDate,
    country: String,
    users: i64,
}

#[derive(Serialize)]
struct Locations {
    unknown: i64,
    countries: Vec<Country>,
    regions: Vec<Place>,
    cities: Vec<Place>,
    weeks: Vec<Week>,
}

#[get("/locations")]
async fn get_locations(_: Admin, pool: web::Data<PgPool>) -> Result<HttpResponse, ApiError> {
    let pool = pool.get_ref();
    let unknown = sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
        "SELECT count(*) FROM \"user\" u WHERE u.deleted_at IS NULL AND {COUNTRY} IS NULL"
    )))
    .fetch_one(pool)
    .await?;
    let countries = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "WITH hours AS (
             SELECT user_id, sum(total_seconds) AS seconds FROM project WHERE deleted_at IS NULL GROUP BY user_id
         ), ships AS (
             SELECT p.user_id, COALESCE(sum(s.mips) FILTER (WHERE NOT s.rejected), 0) AS mips
             FROM submission s JOIN project p ON p.id = s.project_id
             WHERE s.deleted_at IS NULL AND p.deleted_at IS NULL GROUP BY p.user_id
         )
         SELECT {COUNTRY} AS code, count(*) AS users, count(u.onboarded_at) AS onboarded,
             count(ships.user_id) AS shipped,
             (COALESCE(sum(hours.seconds), 0) / 3600.0)::float8 AS hours,
             COALESCE(sum(ships.mips), 0)::bigint AS mips
         FROM \"user\" u
         LEFT JOIN hours ON hours.user_id = u.id
         LEFT JOIN ships ON ships.user_id = u.id
         WHERE u.deleted_at IS NULL AND {COUNTRY} IS NOT NULL
         GROUP BY 1 ORDER BY users DESC, code"
    )))
    .fetch_all(pool)
    .await?;
    let regions = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "SELECT {COUNTRY} AS country, min(trim(u.address_state)) AS name, count(*) AS users
         FROM \"user\" u
         WHERE u.deleted_at IS NULL AND {COUNTRY} IS NOT NULL AND NULLIF(trim(u.address_state), '') IS NOT NULL
         GROUP BY 1, lower(trim(u.address_state)) ORDER BY users DESC, name"
    )))
    .fetch_all(pool)
    .await?;
    let cities = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "SELECT {COUNTRY} AS country, min(trim(u.address_city)) AS name, count(*) AS users
         FROM \"user\" u
         WHERE u.deleted_at IS NULL AND {COUNTRY} IS NOT NULL AND NULLIF(trim(u.address_city), '') IS NOT NULL
         GROUP BY 1, lower(trim(u.address_city)) ORDER BY users DESC, name LIMIT $1"
    )))
    .bind(CITIES)
    .fetch_all(pool)
    .await?;
    let weeks = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "SELECT date_trunc('week', u.signup_at AT TIME ZONE 'UTC')::date AS week, {COUNTRY} AS country,
             count(*) AS users
         FROM \"user\" u
         WHERE u.deleted_at IS NULL AND {COUNTRY} IS NOT NULL
         GROUP BY 1, 2 ORDER BY 1, 2"
    )))
    .fetch_all(pool)
    .await?;
    Ok(HttpResponse::Ok().json(Locations { unknown, countries, regions, cities, weeks }))
}

#[derive(Deserialize)]
struct PeopleQuery {
    country: String,
}

#[derive(Serialize, sqlx::FromRow)]
struct Person {
    id: i32,
    hca_id: String,
    name: String,
    region: Option<String>,
    city: Option<String>,
    signup_at: DateTime<Utc>,
}

#[get("/locations/people")]
async fn get_people(
    Admin(admin_id): Admin,
    pool: web::Data<PgPool>,
    query: web::Query<PeopleQuery>,
) -> Result<HttpResponse, ApiError> {
    let people: Vec<Person> = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "SELECT u.id, u.hca_id, concat_ws(' ', u.first_name, u.last_name) AS name,
             NULLIF(trim(u.address_state), '') AS region, NULLIF(trim(u.address_city), '') AS city, u.signup_at
         FROM \"user\" u
         WHERE u.deleted_at IS NULL AND {COUNTRY} = upper(trim($1))
         ORDER BY u.signup_at DESC LIMIT $2"
    )))
    .bind(&query.country)
    .bind(PEOPLE)
    .fetch_all(pool.get_ref())
    .await?;
    let country: String = query.country.trim().chars().take(8).collect();
    let summary = format!("Viewed the people in {} ({} shown)", country.to_uppercase(), people.len());
    record_view(pool.get_ref(), admin_id, "view_locations", summary).await?;
    Ok(HttpResponse::Ok().json(people))
}
