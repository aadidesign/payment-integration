use serde::{Deserialize, Serialize};

use super::{RazorpayClient, RazorpayPayment};
use crate::error::AppResult;

#[derive(Debug, Clone, Serialize)]
pub struct CapturePaymentRequest {
    pub amount: i64,
    pub currency: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct RefundRequest {
    pub amount: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub speed: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub receipt: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RazorpayRefund {
    pub id: String,
    pub entity: String,
    pub amount: i64,
    pub currency: String,
    pub payment_id: String,
    pub notes: Option<serde_json::Value>,
    pub receipt: Option<String>,
    pub status: String,
    pub speed_requested: String,
    pub speed_processed: String,
    pub created_at: i64,
}

impl RazorpayClient {
    pub async fn get_payment(&self, payment_id: &str) -> AppResult<RazorpayPayment> {
        self.get(&format!("/payments/{}", payment_id)).await
    }

    pub async fn capture_payment(
        &self,
        payment_id: &str,
        request: &CapturePaymentRequest,
    ) -> AppResult<RazorpayPayment> {
        self.post(&format!("/payments/{}/capture", payment_id), request)
            .await
    }

    pub async fn refund_payment(
        &self,
        payment_id: &str,
        request: &RefundRequest,
    ) -> AppResult<RazorpayRefund> {
        self.post(&format!("/payments/{}/refund", payment_id), request)
            .await
    }

    pub async fn get_refund(&self, refund_id: &str) -> AppResult<RazorpayRefund> {
        self.get(&format!("/refunds/{}", refund_id)).await
    }

    pub async fn get_payment_refunds(&self, payment_id: &str) -> AppResult<RefundsResponse> {
        self.get(&format!("/payments/{}/refunds", payment_id)).await
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct RefundsResponse {
    pub entity: String,
    pub count: i32,
    pub items: Vec<RazorpayRefund>,
}
