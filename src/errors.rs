use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use thiserror::Error;

use crate::types::ErrorBody;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("Upstream service error: {0}")]
    Upstream(String),
    
    #[error("Invalid request: {0}")]
    BadRequest(String),
    
    #[error("Not found: {0}")]
    NotFound(String),
    
    #[error("Rate limit exceeded")]
    RateLimit,
    
    #[error("Internal server error: {0}")]
    Internal(String),
    
    #[error("Cache error: {0}")]
    Cache(String),
    
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),
    
    #[error("Configuration error: {0}")]
    Config(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, error_body) = match self {
            AppError::Upstream(msg) => (
                StatusCode::BAD_GATEWAY,
                ErrorBody {
                    code: "upstream_error".to_string(),
                    message: msg,
                },
            ),
            AppError::BadRequest(msg) => (
                StatusCode::BAD_REQUEST,
                ErrorBody {
                    code: "bad_request".to_string(),
                    message: msg,
                },
            ),
            AppError::NotFound(msg) => (
                StatusCode::NOT_FOUND,
                ErrorBody {
                    code: "not_found".to_string(),
                    message: msg,
                },
            ),
            AppError::RateLimit => (
                StatusCode::TOO_MANY_REQUESTS,
                ErrorBody {
                    code: "rate_limit".to_string(),
                    message: "Too many requests. Please try again later.".to_string(),
                },
            ),
            AppError::Internal(msg) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                ErrorBody {
                    code: "internal_error".to_string(),
                    message: msg,
                },
            ),
            AppError::Cache(msg) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                ErrorBody {
                    code: "cache_error".to_string(),
                    message: msg,
                },
            ),
            AppError::Serialization(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                ErrorBody {
                    code: "serialization_error".to_string(),
                    message: e.to_string(),
                },
            ),
            AppError::Network(e) => (
                StatusCode::BAD_GATEWAY,
                ErrorBody {
                    code: "network_error".to_string(),
                    message: format!("Network error: {}", e),
                },
            ),
            AppError::Config(msg) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                ErrorBody {
                    code: "config_error".to_string(),
                    message: msg,
                },
            ),
        };
        
        // Log the error
        match status {
            StatusCode::INTERNAL_SERVER_ERROR => {
                tracing::error!("Internal error: {:?}", self);
            }
            StatusCode::BAD_GATEWAY => {
                tracing::warn!("Upstream error: {:?}", self);
            }
            _ => {
                tracing::debug!("Request error: {:?}", self);
            }
        }
        
        (status, Json(error_body)).into_response()
    }
}

// Helper type for Result
pub type AppResult<T> = Result<T, AppError>;

// Conversion helpers
impl From<String> for AppError {
    fn from(s: String) -> Self {
        AppError::Internal(s)
    }
}

impl From<&str> for AppError {
    fn from(s: &str) -> Self {
        AppError::Internal(s.to_string())
    }
}
