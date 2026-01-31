use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    // Database errors
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    // Validation errors
    #[error("Validation error: {0}")]
    Validation(String),

    // Authentication errors
    #[error("Authentication failed: {0}")]
    Authentication(String),

    #[error("Unauthorized: {0}")]
    Unauthorized(String),

    // Payment errors
    #[error("Payment error: {0}")]
    Payment(String),

    #[error("Razorpay error: {0}")]
    Razorpay(String),

    // Crypto errors
    #[error("Ethereum error: {0}")]
    Ethereum(String),

    #[error("Solana error: {0}")]
    Solana(String),

    #[error("Lightning error: {0}")]
    Lightning(String),

    #[error("Invalid signature: {0}")]
    InvalidSignature(String),

    #[error("Invalid address: {0}")]
    InvalidAddress(String),

    // HTTP errors
    #[error("HTTP client error: {0}")]
    HttpClient(#[from] reqwest::Error),

    // Not found
    #[error("Resource not found: {0}")]
    NotFound(String),

    // Rate limiting
    #[error("Rate limit exceeded")]
    RateLimitExceeded,

    // Configuration errors
    #[error("Configuration error: {0}")]
    Config(String),

    // Serialization errors
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    // Internal errors
    #[error("Internal server error: {0}")]
    Internal(String),

    // Webhook errors
    #[error("Webhook verification failed: {0}")]
    WebhookVerification(String),
}

#[derive(Serialize)]
pub struct ErrorResponse {
    pub success: bool,
    pub error: ErrorDetail,
}

#[derive(Serialize)]
pub struct ErrorDetail {
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, code, message) = match &self {
            AppError::Database(e) => {
                tracing::error!("Database error: {:?}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "DATABASE_ERROR",
                    "An internal database error occurred".to_string(),
                )
            }
            AppError::Validation(msg) => (StatusCode::BAD_REQUEST, "VALIDATION_ERROR", msg.clone()),
            AppError::Authentication(msg) => {
                (StatusCode::UNAUTHORIZED, "AUTHENTICATION_FAILED", msg.clone())
            }
            AppError::Unauthorized(msg) => (StatusCode::FORBIDDEN, "UNAUTHORIZED", msg.clone()),
            AppError::Payment(msg) => (StatusCode::BAD_REQUEST, "PAYMENT_ERROR", msg.clone()),
            AppError::Razorpay(msg) => (StatusCode::BAD_REQUEST, "RAZORPAY_ERROR", msg.clone()),
            AppError::Ethereum(msg) => (StatusCode::BAD_REQUEST, "ETHEREUM_ERROR", msg.clone()),
            AppError::Solana(msg) => (StatusCode::BAD_REQUEST, "SOLANA_ERROR", msg.clone()),
            AppError::Lightning(msg) => (StatusCode::BAD_REQUEST, "LIGHTNING_ERROR", msg.clone()),
            AppError::InvalidSignature(msg) => {
                (StatusCode::BAD_REQUEST, "INVALID_SIGNATURE", msg.clone())
            }
            AppError::InvalidAddress(msg) => {
                (StatusCode::BAD_REQUEST, "INVALID_ADDRESS", msg.clone())
            }
            AppError::HttpClient(e) => {
                tracing::error!("HTTP client error: {:?}", e);
                (
                    StatusCode::BAD_GATEWAY,
                    "EXTERNAL_SERVICE_ERROR",
                    "Failed to communicate with external service".to_string(),
                )
            }
            AppError::NotFound(msg) => (StatusCode::NOT_FOUND, "NOT_FOUND", msg.clone()),
            AppError::RateLimitExceeded => (
                StatusCode::TOO_MANY_REQUESTS,
                "RATE_LIMIT_EXCEEDED",
                "Too many requests, please try again later".to_string(),
            ),
            AppError::Config(msg) => {
                tracing::error!("Configuration error: {}", msg);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "CONFIG_ERROR",
                    "Server configuration error".to_string(),
                )
            }
            AppError::Serialization(e) => {
                tracing::error!("Serialization error: {:?}", e);
                (
                    StatusCode::BAD_REQUEST,
                    "SERIALIZATION_ERROR",
                    "Invalid request format".to_string(),
                )
            }
            AppError::Internal(msg) => {
                tracing::error!("Internal error: {}", msg);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "INTERNAL_ERROR",
                    "An internal error occurred".to_string(),
                )
            }
            AppError::WebhookVerification(msg) => {
                (StatusCode::UNAUTHORIZED, "WEBHOOK_VERIFICATION_FAILED", msg.clone())
            }
        };

        let body = Json(ErrorResponse {
            success: false,
            error: ErrorDetail {
                code: code.to_string(),
                message,
                details: None,
            },
        });

        (status, body).into_response()
    }
}

pub type AppResult<T> = Result<T, AppError>;
