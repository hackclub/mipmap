use actix_web::{HttpResponse, ResponseError, http::StatusCode};
use serde_json::json;
use std::fmt;

#[derive(Debug)]
pub enum ApiError {
    Rejected(StatusCode, String),
    Internal(anyhow::Error),
}

pub fn reject(message: impl Into<String>) -> ApiError {
    reject_with(StatusCode::BAD_REQUEST, message)
}

pub fn reject_with(status: StatusCode, message: impl Into<String>) -> ApiError {
    ApiError::Rejected(status, message.into())
}

impl fmt::Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ApiError::Rejected(_, message) => f.write_str(message),
            ApiError::Internal(error) => write!(f, "{error:#}"),
        }
    }
}

impl ResponseError for ApiError {
    fn error_response(&self) -> HttpResponse {
        match self {
            ApiError::Rejected(status, message) => {
                HttpResponse::build(*status).json(json!({ "error": message }))
            }
            ApiError::Internal(error) => {
                log::error!("{error:#}");
                HttpResponse::InternalServerError().json(json!({ "error": "Something went wrong" }))
            }
        }
    }
}

impl From<anyhow::Error> for ApiError {
    fn from(error: anyhow::Error) -> Self {
        ApiError::Internal(error)
    }
}

impl From<sqlx::Error> for ApiError {
    fn from(error: sqlx::Error) -> Self {
        ApiError::Internal(error.into())
    }
}
