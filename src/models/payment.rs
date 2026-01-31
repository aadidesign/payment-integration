use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq)]
#[sqlx(type_name = "payment_status", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum PaymentStatus {
    Pending,
    Processing,
    Completed,
    Failed,
    Cancelled,
    Refunded,
    Expired,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq)]
#[sqlx(type_name = "payment_method", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum PaymentMethod {
    Card,
    Upi,
    NetBanking,
    Wallet,
    Emi,
    Ethereum,
    Polygon,
    Bsc,
    Arbitrum,
    Solana,
    Lightning,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq)]
#[sqlx(type_name = "currency_type", rename_all = "UPPERCASE")]
#[serde(rename_all = "UPPERCASE")]
pub enum CurrencyType {
    INR,
    USD,
    EUR,
    ETH,
    MATIC,
    BNB,
    SOL,
    BTC,
    USDT,
    USDC,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Payment {
    pub id: Uuid,
    pub external_id: Option<String>,
    pub order_id: Option<String>,
    pub amount: i64,
    pub currency: CurrencyType,
    pub status: PaymentStatus,
    pub method: PaymentMethod,
    pub description: Option<String>,
    pub customer_email: Option<String>,
    pub customer_phone: Option<String>,
    pub metadata: Option<serde_json::Value>,
    pub razorpay_payment_id: Option<String>,
    pub razorpay_order_id: Option<String>,
    pub razorpay_signature: Option<String>,
    pub crypto_tx_hash: Option<String>,
    pub crypto_from_address: Option<String>,
    pub crypto_to_address: Option<String>,
    pub crypto_chain: Option<String>,
    pub lightning_invoice: Option<String>,
    pub lightning_payment_hash: Option<String>,
    pub expires_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatePaymentRequest {
    pub amount: i64,
    pub currency: CurrencyType,
    pub method: PaymentMethod,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub customer_email: Option<String>,
    #[serde(default)]
    pub customer_phone: Option<String>,
    #[serde(default)]
    pub metadata: Option<serde_json::Value>,
    #[serde(default)]
    pub callback_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentResponse {
    pub id: Uuid,
    pub status: PaymentStatus,
    pub amount: i64,
    pub currency: CurrencyType,
    pub method: PaymentMethod,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub razorpay_order_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub crypto_address: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lightning_invoice: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

impl From<Payment> for PaymentResponse {
    fn from(payment: Payment) -> Self {
        Self {
            id: payment.id,
            status: payment.status,
            amount: payment.amount,
            currency: payment.currency,
            method: payment.method,
            razorpay_order_id: payment.razorpay_order_id,
            crypto_address: payment.crypto_to_address,
            lightning_invoice: payment.lightning_invoice,
            expires_at: payment.expires_at,
            created_at: payment.created_at,
        }
    }
}
