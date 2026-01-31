use lightning_invoice::Bolt11Invoice;
use std::str::FromStr;

use crate::config::LightningConfig;
use crate::error::{AppError, AppResult};

#[derive(Clone)]
pub struct LightningService {
    node_url: String,
    #[allow(dead_code)]
    macaroon_path: Option<String>,
    #[allow(dead_code)]
    tls_cert_path: Option<String>,
}

impl LightningService {
    pub fn new(config: &LightningConfig) -> Self {
        Self {
            node_url: config.node_url.clone(),
            macaroon_path: config.macaroon_path.clone(),
            tls_cert_path: config.tls_cert_path.clone(),
        }
    }

    /// Parse and validate a BOLT11 invoice
    pub fn parse_invoice(invoice_str: &str) -> AppResult<InvoiceInfo> {
        let invoice = Bolt11Invoice::from_str(invoice_str)
            .map_err(|e| AppError::Lightning(format!("Invalid invoice: {}", e)))?;

        let amount_msat = invoice.amount_milli_satoshis();
        let payment_hash = hex::encode(invoice.payment_hash().as_ref());
        let description = invoice.description().to_string();
        let expiry = invoice.expiry_time().as_secs();
        let timestamp = invoice.timestamp().duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        Ok(InvoiceInfo {
            payment_hash,
            amount_msat,
            amount_sat: amount_msat.map(|m| m / 1000),
            description,
            expiry_seconds: expiry,
            timestamp,
            is_expired: invoice.is_expired(),
        })
    }

    /// Create a new invoice (requires LND/CLN node connection)
    pub async fn create_invoice(
        &self,
        amount_sat: u64,
        description: &str,
        expiry_seconds: u32,
    ) -> AppResult<CreateInvoiceResponse> {
        // This would connect to your Lightning node (LND, CLN, etc.)
        // Using the node_url and authentication

        // For now, return a placeholder - in production, this would make
        // an API call to your Lightning node
        let _request = CreateInvoiceRequest {
            amount_sat,
            description: description.to_string(),
            expiry_seconds,
        };

        // Example implementation for LND REST API:
        // let client = reqwest::Client::new();
        // let response = client
        //     .post(&format!("{}/v1/invoices", self.node_url))
        //     .header("Grpc-Metadata-macaroon", macaroon_hex)
        //     .json(&request)
        //     .send()
        //     .await?;

        Err(AppError::Lightning(
            "Lightning node connection not configured. Set LIGHTNING_NODE_URL in environment.".to_string()
        ))
    }

    /// Check if an invoice has been paid
    pub async fn check_payment(&self, payment_hash: &str) -> AppResult<PaymentStatus> {
        // This would query your Lightning node to check payment status

        // For LND REST API:
        // GET /v1/invoice/{payment_hash}

        let _ = payment_hash;

        Err(AppError::Lightning(
            "Lightning node connection not configured".to_string()
        ))
    }

    /// Validate a payment hash format
    pub fn validate_payment_hash(hash: &str) -> bool {
        // Payment hash should be 64 hex characters (32 bytes)
        hash.len() == 64 && hash.chars().all(|c| c.is_ascii_hexdigit())
    }

    /// Convert satoshis to BTC
    pub fn sats_to_btc(sats: u64) -> f64 {
        sats as f64 / 100_000_000.0
    }

    /// Convert BTC to satoshis
    pub fn btc_to_sats(btc: f64) -> u64 {
        (btc * 100_000_000.0) as u64
    }

    /// Convert millisatoshis to satoshis
    pub fn msat_to_sat(msat: u64) -> u64 {
        msat / 1000
    }
}

#[derive(Debug, Clone)]
pub struct InvoiceInfo {
    pub payment_hash: String,
    pub amount_msat: Option<u64>,
    pub amount_sat: Option<u64>,
    pub description: String,
    pub expiry_seconds: u64,
    pub timestamp: u64,
    pub is_expired: bool,
}

#[derive(Debug, Clone)]
pub struct CreateInvoiceRequest {
    pub amount_sat: u64,
    pub description: String,
    pub expiry_seconds: u32,
}

#[derive(Debug, Clone)]
pub struct CreateInvoiceResponse {
    pub payment_request: String,
    pub payment_hash: String,
    pub add_index: u64,
}

#[derive(Debug, Clone)]
pub enum PaymentStatus {
    Pending,
    Settled,
    Cancelled,
    Unknown,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_payment_hash() {
        // Valid 32-byte hex string
        let valid_hash = "0001020304050607080910111213141516171819202122232425262728293031";
        assert!(LightningService::validate_payment_hash(valid_hash));

        // Invalid - too short
        let short_hash = "000102030405";
        assert!(!LightningService::validate_payment_hash(short_hash));

        // Invalid - non-hex characters
        let invalid_hash = "000102030405060708091011121314151617181920212223242526272829303g";
        assert!(!LightningService::validate_payment_hash(invalid_hash));
    }

    #[test]
    fn test_sats_conversion() {
        assert_eq!(LightningService::btc_to_sats(1.0), 100_000_000);
        assert_eq!(LightningService::btc_to_sats(0.001), 100_000);

        assert!((LightningService::sats_to_btc(100_000_000) - 1.0).abs() < f64::EPSILON);
        assert!((LightningService::sats_to_btc(100_000) - 0.001).abs() < f64::EPSILON);
    }
}
