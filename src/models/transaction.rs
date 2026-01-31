use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq)]
#[sqlx(type_name = "transaction_type", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum TransactionType {
    Payment,
    Refund,
    Transfer,
    Withdrawal,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq)]
#[sqlx(type_name = "transaction_status", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum TransactionStatus {
    Pending,
    Confirming,
    Confirmed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Transaction {
    pub id: Uuid,
    pub payment_id: Uuid,
    pub tx_type: TransactionType,
    pub status: TransactionStatus,
    pub amount: i64,
    pub fee: Option<i64>,
    pub currency: String,
    pub tx_hash: Option<String>,
    pub block_number: Option<i64>,
    pub confirmations: i32,
    pub required_confirmations: i32,
    pub from_address: Option<String>,
    pub to_address: Option<String>,
    pub chain: Option<String>,
    pub raw_data: Option<serde_json::Value>,
    pub error_message: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionResponse {
    pub id: Uuid,
    pub payment_id: Uuid,
    pub tx_type: TransactionType,
    pub status: TransactionStatus,
    pub amount: i64,
    pub currency: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tx_hash: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub block_number: Option<i64>,
    pub confirmations: i32,
    pub required_confirmations: i32,
    pub created_at: DateTime<Utc>,
}

impl From<Transaction> for TransactionResponse {
    fn from(tx: Transaction) -> Self {
        Self {
            id: tx.id,
            payment_id: tx.payment_id,
            tx_type: tx.tx_type,
            status: tx.status,
            amount: tx.amount,
            currency: tx.currency,
            tx_hash: tx.tx_hash,
            block_number: tx.block_number,
            confirmations: tx.confirmations,
            required_confirmations: tx.required_confirmations,
            created_at: tx.created_at,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifyTransactionRequest {
    pub payment_id: Uuid,
    pub tx_hash: String,
    pub chain: String,
}
