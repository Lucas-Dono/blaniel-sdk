use std::fmt;

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;

#[derive(Debug)]
pub enum ApiError {
    DatabaseError(npc_db::DbError),
    CacheError(npc_cache::CacheError),
    Unauthorized,
    NotFound(String),
    BadRequest(String),
    RateLimitExceeded,
    InternalError(String),
}

impl fmt::Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ApiError::Unauthorized => write!(f, "Authentication required"),
            ApiError::NotFound(msg) => write!(f, "Not found: {}", msg),
            ApiError::BadRequest(msg) => write!(f, "Bad request: {}", msg),
            ApiError::RateLimitExceeded => write!(f, "Rate limit exceeded"),
            ApiError::DatabaseError(e) => write!(f, "Database error: {}", e),
            ApiError::CacheError(e) => write!(f, "Cache error: {}", e),
            ApiError::InternalError(msg) => write!(f, "Internal error: {}", msg),
        }
    }
}

impl From<npc_db::DbError> for ApiError {
    fn from(err: npc_db::DbError) -> Self {
        ApiError::DatabaseError(err)
    }
}

impl From<npc_cache::CacheError> for ApiError {
    fn from(err: npc_cache::CacheError) -> Self {
        ApiError::CacheError(err)
    }
}

impl From<anyhow::Error> for ApiError {
    fn from(err: anyhow::Error) -> Self {
        ApiError::InternalError(err.to_string())
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, code, message) = match self {
            ApiError::Unauthorized => (
                StatusCode::UNAUTHORIZED,
                "UNAUTHORIZED",
                "Authentication required".to_string(),
            ),
            ApiError::NotFound(msg) => (StatusCode::NOT_FOUND, "NOT_FOUND", msg),
            ApiError::BadRequest(msg) => (StatusCode::BAD_REQUEST, "BAD_REQUEST", msg),
            ApiError::RateLimitExceeded => (
                StatusCode::TOO_MANY_REQUESTS,
                "RATE_LIMIT_EXCEEDED",
                "Too many requests".to_string(),
            ),
            ApiError::DatabaseError(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "DATABASE_ERROR",
                format!("Database error: {}", e),
            ),
            ApiError::CacheError(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "CACHE_ERROR",
                format!("Cache error: {}", e),
            ),
            ApiError::InternalError(msg) => {
                (StatusCode::INTERNAL_SERVER_ERROR, "INTERNAL_ERROR", msg)
            }
        };

        let body = Json(json!({
            "error": message,
            "code": code,
        }));

        (status, body).into_response()
    }
}
