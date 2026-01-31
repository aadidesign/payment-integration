use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq)]
#[sqlx(type_name = "chain_type", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum ChainType {
    Ethereum,
    Polygon,
    Bsc,
    Arbitrum,
    Solana,
    Bitcoin,
}

impl std::fmt::Display for ChainType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ChainType::Ethereum => write!(f, "ethereum"),
            ChainType::Polygon => write!(f, "polygon"),
            ChainType::Bsc => write!(f, "bsc"),
            ChainType::Arbitrum => write!(f, "arbitrum"),
            ChainType::Solana => write!(f, "solana"),
            ChainType::Bitcoin => write!(f, "bitcoin"),
        }
    }
}

impl std::str::FromStr for ChainType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "ethereum" | "eth" => Ok(ChainType::Ethereum),
            "polygon" | "matic" => Ok(ChainType::Polygon),
            "bsc" | "bnb" => Ok(ChainType::Bsc),
            "arbitrum" | "arb" => Ok(ChainType::Arbitrum),
            "solana" | "sol" => Ok(ChainType::Solana),
            "bitcoin" | "btc" => Ok(ChainType::Bitcoin),
            _ => Err(format!("Unknown chain: {}", s)),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct CryptoAddress {
    pub id: Uuid,
    pub payment_id: Option<Uuid>,
    pub address: String,
    pub chain: ChainType,
    pub is_active: bool,
    pub label: Option<String>,
    pub expected_amount: Option<i64>,
    pub received_amount: Option<i64>,
    pub token_address: Option<String>,
    pub last_checked_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerateAddressRequest {
    pub chain: ChainType,
    #[serde(default)]
    pub label: Option<String>,
    #[serde(default)]
    pub token_address: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddressResponse {
    pub address: String,
    pub chain: ChainType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payment_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expected_amount: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BalanceResponse {
    pub address: String,
    pub chain: ChainType,
    pub balance: String,
    pub balance_wei: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token_balance: Option<String>,
}
