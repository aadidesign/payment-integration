use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq)]
#[sqlx(type_name = "webhook_source", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum WebhookSource {
    Razorpay,
    Blockchain,
    Lightning,
    Internal,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq)]
#[sqlx(type_name = "webhook_status", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum WebhookStatus {
    Received,
    Processing,
    Processed,
    Failed,
    Ignored,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct WebhookEvent {
    pub id: Uuid,
    pub source: WebhookSource,
    pub event_type: String,
    pub event_id: Option<String>,
    pub payment_id: Option<Uuid>,
    pub status: WebhookStatus,
    pub payload: serde_json::Value,
    pub headers: Option<serde_json::Value>,
    pub signature: Option<String>,
    pub signature_verified: bool,
    pub error_message: Option<String>,
    pub processed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

impl WebhookEvent {
    pub fn new(source: WebhookSource, event_type: String, payload: serde_json::Value) -> Self {
        Self {
            id: Uuid::new_v4(),
            source,
            event_type,
            event_id: None,
            payment_id: None,
            status: WebhookStatus::Received,
            payload,
            headers: None,
            signature: None,
            signature_verified: false,
            error_message: None,
            processed_at: None,
            created_at: Utc::now(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RazorpayWebhookPayload {
    pub entity: String,
    pub account_id: String,
    pub event: String,
    pub contains: Vec<String>,
    pub payload: RazorpayPaymentPayload,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RazorpayPaymentPayload {
    pub payment: Option<RazorpayPaymentEntity>,
    pub order: Option<RazorpayOrderEntity>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RazorpayPaymentEntity {
    pub entity: RazorpayPaymentData,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RazorpayPaymentData {
    pub id: String,
    pub entity: String,
    pub amount: i64,
    pub currency: String,
    pub status: String,
    pub order_id: Option<String>,
    pub method: Option<String>,
    pub description: Option<String>,
    pub email: Option<String>,
    pub contact: Option<String>,
    pub error_code: Option<String>,
    pub error_description: Option<String>,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RazorpayOrderEntity {
    pub entity: RazorpayOrderData,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RazorpayOrderData {
    pub id: String,
    pub entity: String,
    pub amount: i64,
    pub amount_paid: i64,
    pub amount_due: i64,
    pub currency: String,
    pub status: String,
    pub created_at: i64,
}
