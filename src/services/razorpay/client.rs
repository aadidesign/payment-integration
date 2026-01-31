use reqwest::{Client, StatusCode};
use serde::{de::DeserializeOwned, Serialize};

use crate::config::RazorpayConfig;
use crate::error::{AppError, AppResult};

const RAZORPAY_API_URL: &str = "https://api.razorpay.com/v1";

#[derive(Clone)]
pub struct RazorpayClient {
    http_client: Client,
    key_id: String,
    key_secret: String,
    webhook_secret: String,
}

impl RazorpayClient {
    pub fn new(config: &RazorpayConfig) -> Self {
        let http_client = Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            http_client,
            key_id: config.key_id.clone(),
            key_secret: config.key_secret.clone(),
            webhook_secret: config.webhook_secret.clone(),
        }
    }

    pub fn webhook_secret(&self) -> &str {
        &self.webhook_secret
    }

    pub async fn get<T: DeserializeOwned>(&self, endpoint: &str) -> AppResult<T> {
        let url = format!("{}{}", RAZORPAY_API_URL, endpoint);

        let response = self
            .http_client
            .get(&url)
            .basic_auth(&self.key_id, Some(&self.key_secret))
            .send()
            .await?;

        self.handle_response(response).await
    }

    pub async fn post<T: DeserializeOwned, B: Serialize>(
        &self,
        endpoint: &str,
        body: &B,
    ) -> AppResult<T> {
        let url = format!("{}{}", RAZORPAY_API_URL, endpoint);

        let response = self
            .http_client
            .post(&url)
            .basic_auth(&self.key_id, Some(&self.key_secret))
            .json(body)
            .send()
            .await?;

        self.handle_response(response).await
    }

    async fn handle_response<T: DeserializeOwned>(
        &self,
        response: reqwest::Response,
    ) -> AppResult<T> {
        let status = response.status();
        let body = response.text().await?;

        if status.is_success() {
            serde_json::from_str(&body).map_err(|e| {
                tracing::error!("Failed to parse Razorpay response: {} - Body: {}", e, body);
                AppError::Razorpay(format!("Failed to parse response: {}", e))
            })
        } else {
            tracing::error!("Razorpay API error: {} - {}", status, body);

            let error_msg = match status {
                StatusCode::BAD_REQUEST => {
                    if let Ok(error) = serde_json::from_str::<RazorpayError>(&body) {
                        error.error.description
                    } else {
                        "Bad request".to_string()
                    }
                }
                StatusCode::UNAUTHORIZED => "Invalid API credentials".to_string(),
                StatusCode::NOT_FOUND => "Resource not found".to_string(),
                StatusCode::TOO_MANY_REQUESTS => "Rate limit exceeded".to_string(),
                _ => format!("API error: {}", status),
            };

            Err(AppError::Razorpay(error_msg))
        }
    }
}

#[derive(Debug, serde::Deserialize)]
struct RazorpayError {
    error: RazorpayErrorDetail,
}

#[derive(Debug, serde::Deserialize)]
struct RazorpayErrorDetail {
    code: String,
    description: String,
}
