use crate::{
    admin::{Admin, PAGE},
    error::{ApiError, reject, reject_with},
    middleware,
};
use actix_web::{HttpRequest, HttpResponse, get, http::StatusCode, post, web};
use chrono::{NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use sqlx::PgPool;
use std::{
    collections::HashMap,
    net::IpAddr,
    sync::{LazyLock, Mutex, PoisonError},
};

const SURVEYS_JSON: &str = include_str!("../frontend/src/lib/assets/surveys.json");
const SCALE: std::ops::RangeInclusive<i64> = 1..=5;
const UNSURE: &str = "unsure";
const TEXT_LIMIT: usize = 2000;
const RESPONSES_PER_IP: u32 = 5;
const RESPONDER_IPS: usize = 100_000;

#[derive(Deserialize)]
struct Survey {
    id: i32,
    title: String,
    questions: Vec<Question>,
}

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
enum Question {
    Scale { key: String, items: Vec<Item> },
    Text {
        key: String,
        label: String,
        #[serde(default)]
        optional: bool,
    },
}

#[derive(Deserialize)]
struct Item {
    key: String,
    label: String,
}

static SURVEYS: LazyLock<Vec<Survey>> =
    LazyLock::new(|| serde_json::from_str(SURVEYS_JSON).expect("surveys json must be valid"));

static RESPONSES: LazyLock<Mutex<(NaiveDate, HashMap<IpAddr, u32>)>> = LazyLock::new(Default::default);

fn survey(id: i32) -> Option<&'static Survey> {
    SURVEYS.iter().find(|survey| survey.id == id)
}

pub fn routes(cfg: &mut web::ServiceConfig) {
    cfg.service(respond);
}

pub fn admin_routes(cfg: &mut web::ServiceConfig) {
    cfg.service((get_results, get_responses, delete_response));
}

pub fn pages(cfg: &mut web::ServiceConfig) {
    cfg.service(page);
}

fn allow_response(ip: IpAddr) -> bool {
    let today = Utc::now().date_naive();
    let mut responses = RESPONSES.lock().unwrap_or_else(PoisonError::into_inner);
    if responses.0 != today {
        *responses = (today, HashMap::new());
    }
    if responses.1.len() >= RESPONDER_IPS && !responses.1.contains_key(&ip) {
        return false;
    }
    let count = responses.1.entry(ip).or_default();
    *count = count.saturating_add(1);
    *count <= RESPONSES_PER_IP
}

fn validate(survey: &Survey, mut body: Map<String, Value>) -> Result<Value, ApiError> {
    let mut answers = Map::new();
    for question in &survey.questions {
        match question {
            Question::Scale { key, items } => {
                let Some(Value::Object(mut given)) = body.remove(key) else {
                    return Err(reject("Rate every topic before submitting"));
                };
                let mut ratings = Map::new();
                for item in items {
                    let rating = given
                        .remove(&item.key)
                        .filter(|rating| match rating {
                            Value::Number(n) => n.as_i64().is_some_and(|n| SCALE.contains(&n)),
                            Value::String(s) => s == UNSURE,
                            _ => false,
                        })
                        .ok_or_else(|| reject("Rate every topic before submitting"))?;
                    ratings.insert(item.key.clone(), rating);
                }
                if !given.is_empty() {
                    return Err(reject("Unknown topic"));
                }
                answers.insert(key.clone(), Value::Object(ratings));
            }
            Question::Text { key, optional, .. } => {
                let text = match body.remove(key) {
                    None | Some(Value::Null) => String::new(),
                    Some(Value::String(text)) => text.trim().to_string(),
                    Some(_) => return Err(reject("Answers must be text")),
                };
                if text.chars().count() > TEXT_LIMIT {
                    return Err(reject(format!("Answers can be at most {TEXT_LIMIT} characters")));
                }
                if text.is_empty() {
                    if !optional {
                        return Err(reject("Answer every question before submitting"));
                    }
                } else {
                    answers.insert(key.clone(), Value::String(text));
                }
            }
        }
    }
    if !body.is_empty() {
        return Err(reject("Unknown question"));
    }
    Ok(Value::Object(answers))
}

#[post("/surveys/{id}")]
async fn respond(
    req: HttpRequest,
    id: web::Path<i32>,
    pool: web::Data<PgPool>,
    body: web::Json<Map<String, Value>>,
) -> Result<HttpResponse, ApiError> {
    let survey = survey(id.into_inner()).ok_or_else(|| reject_with(StatusCode::NOT_FOUND, "Survey not found"))?;
    let answers = validate(survey, body.into_inner())?;
    if !middleware::client_ip(&req).is_some_and(allow_response) {
        return Err(reject_with(StatusCode::TOO_MANY_REQUESTS, "Too many responses, try again tomorrow"));
    }
    sqlx::query("INSERT INTO survey_response (survey_id, answers) VALUES ($1, $2)")
        .bind(survey.id)
        .bind(answers)
        .execute(pool.get_ref())
        .await?;
    Ok(HttpResponse::NoContent().finish())
}

#[derive(Serialize)]
struct SurveyResults {
    id: i32,
    title: &'static str,
    responses: i64,
    questions: Vec<QuestionResults>,
}

#[derive(Serialize)]
#[serde(tag = "type", rename_all = "lowercase")]
enum QuestionResults {
    Scale { key: &'static str, items: Vec<ItemResults> },
    Text { key: &'static str, label: &'static str, answers: Vec<TextAnswer> },
}

#[derive(Serialize)]
struct ItemResults {
    key: &'static str,
    label: &'static str,
    counts: Vec<i64>,
    unsure: i64,
}

#[derive(Serialize, sqlx::FromRow)]
struct TextAnswer {
    #[serde(skip)]
    survey_id: i32,
    #[serde(skip)]
    question: String,
    created_on: NaiveDate,
    text: String,
}

#[get("/surveys")]
async fn get_results(_: Admin, pool: web::Data<PgPool>) -> Result<HttpResponse, ApiError> {
    let totals: HashMap<i32, i64> = sqlx::query_as(
        "SELECT survey_id, count(*) FROM survey_response WHERE deleted_at IS NULL GROUP BY survey_id",
    )
    .fetch_all(pool.get_ref())
    .await?
    .into_iter()
    .collect();
    let ratings: Vec<(i32, String, String, String, i64)> = sqlx::query_as(
        "SELECT r.survey_id, q.key, i.key, i.value #>> '{}', count(*)
         FROM survey_response r
         CROSS JOIN LATERAL jsonb_each(r.answers) q
         CROSS JOIN LATERAL jsonb_each(CASE WHEN jsonb_typeof(q.value) = 'object' THEN q.value ELSE '{}' END) i
         WHERE r.deleted_at IS NULL
         GROUP BY 1, 2, 3, 4",
    )
    .fetch_all(pool.get_ref())
    .await?;
    let mut texts: Vec<TextAnswer> = sqlx::query_as(
        "SELECT r.survey_id, a.key AS question, r.created_on, a.value #>> '{}' AS text
         FROM survey_response r
         CROSS JOIN LATERAL jsonb_each(r.answers) a
         WHERE r.deleted_at IS NULL AND jsonb_typeof(a.value) = 'string'
         ORDER BY r.id DESC",
    )
    .fetch_all(pool.get_ref())
    .await?;

    let results: Vec<SurveyResults> = SURVEYS
        .iter()
        .map(|survey| SurveyResults {
            id: survey.id,
            title: &survey.title,
            responses: totals.get(&survey.id).copied().unwrap_or(0),
            questions: survey
                .questions
                .iter()
                .map(|question| match question {
                    Question::Scale { key, items } => QuestionResults::Scale {
                        key,
                        items: items
                            .iter()
                            .map(|item| {
                                let count = |value: &str| {
                                    ratings
                                        .iter()
                                        .filter(|(s, q, i, v, _)| {
                                            *s == survey.id && q == key && *i == item.key && v == value
                                        })
                                        .map(|row| row.4)
                                        .sum()
                                };
                                ItemResults {
                                    key: &item.key,
                                    label: &item.label,
                                    counts: SCALE.map(|n| count(&n.to_string())).collect(),
                                    unsure: count(UNSURE),
                                }
                            })
                            .collect(),
                    },
                    Question::Text { key, label, .. } => {
                        let (answers, rest) = std::mem::take(&mut texts)
                            .into_iter()
                            .partition(|text| text.survey_id == survey.id && &text.question == key);
                        texts = rest;
                        QuestionResults::Text { key, label, answers }
                    }
                })
                .collect(),
        })
        .collect();
    Ok(HttpResponse::Ok().json(results))
}

#[derive(Deserialize)]
struct ResponsesQuery {
    before: Option<i32>,
}

#[derive(Serialize, sqlx::FromRow)]
struct SurveyResponse {
    id: i32,
    created_on: NaiveDate,
    answers: Value,
}

#[get("/surveys/{id}/responses")]
async fn get_responses(
    _: Admin,
    pool: web::Data<PgPool>,
    id: web::Path<i32>,
    query: web::Query<ResponsesQuery>,
) -> Result<HttpResponse, ApiError> {
    let responses: Vec<SurveyResponse> = sqlx::query_as(
        "SELECT id, created_on, answers FROM survey_response
         WHERE survey_id = $1 AND deleted_at IS NULL AND ($2::int IS NULL OR id < $2)
         ORDER BY id DESC LIMIT $3",
    )
    .bind(id.into_inner())
    .bind(query.before)
    .bind(PAGE)
    .fetch_all(pool.get_ref())
    .await?;
    Ok(HttpResponse::Ok().json(responses))
}

#[post("/surveys/responses/{id}/delete")]
async fn delete_response(
    Admin(admin_id): Admin,
    pool: web::Data<PgPool>,
    id: web::Path<i32>,
) -> Result<HttpResponse, ApiError> {
    let id = id.into_inner();
    let mut tx = pool.begin().await?;
    let survey_id: i32 = sqlx::query_scalar(
        "UPDATE survey_response SET deleted_at = now() WHERE id = $1 AND deleted_at IS NULL RETURNING survey_id",
    )
    .bind(id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| reject_with(StatusCode::NOT_FOUND, "Response not found"))?;
    sqlx::query("INSERT INTO admin_action (admin_id, action, summary) VALUES ($1, 'delete_survey_response', $2)")
        .bind(admin_id)
        .bind(format!("Deleted response #{id} to survey #{survey_id}"))
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(HttpResponse::NoContent().finish())
}

#[get("/surveys/{id}")]
async fn page(req: HttpRequest, id: web::Path<String>) -> HttpResponse {
    match id.parse().ok().and_then(survey) {
        Some(survey) => crate::html(&req, &format!("surveys/{}", survey.id)).await,
        None => HttpResponse::NotFound().finish(),
    }
}
