use serde::{Deserialize, Serialize};

use super::RazorpayClient;
use crate::error::AppResult;

#[derive(Debug, Clone, Serialize)]
pub struct CreateOrderRequest {
    pub amount: i64,
    pub currency: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub receipt: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub partial_payment: Option<bool>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RazorpayOrder {
    pub id: String,
    pub entity: String,
    pub amount: i64,
    pub amount_paid: i64,
    pub amount_due: i64,
    pub currency: String,
    pub receipt: Option<String>,
    pub status: String,
    pub attempts: i32,
    pub notes: Option<serde_json::Value>,
    pub created_at: i64,
}

impl RazorpayClient {
    pub async fn create_order(&self, request: &CreateOrderRequest) -> AppResult<RazorpayOrder> {
        self.post("/orders", request).await
    }

    pub async fn get_order(&self, order_id: &str) -> AppResult<RazorpayOrder> {
        self.get(&format!("/orders/{}", order_id)).await
    }

    pub async fn get_order_payments(&self, order_id: &str) -> AppResult<OrderPaymentsResponse> {
        self.get(&format!("/orders/{}/payments", order_id)).await
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct OrderPaymentsResponse {
    pub entity: String,
    pub count: i32,
    pub items: Vec<RazorpayPayment>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RazorpayPayment {
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
    pub fee: Option<i64>,
    pub tax: Option<i64>,
    pub error_code: Option<String>,
    pub error_description: Option<String>,
    pub created_at: i64,
}
