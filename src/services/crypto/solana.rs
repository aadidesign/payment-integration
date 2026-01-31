use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    pubkey::Pubkey,
    signature::Signature,
};
use std::str::FromStr;
use std::sync::Arc;

use crate::config::SolanaConfig;
use crate::error::{AppError, AppResult};

#[derive(Clone)]
pub struct SolanaService {
    client: Arc<RpcClient>,
}

impl SolanaService {
    pub fn new(config: &SolanaConfig) -> Self {
        let client = RpcClient::new_with_commitment(
            config.rpc_url.clone(),
            CommitmentConfig::confirmed(),
        );

        Self {
            client: Arc::new(client),
        }
    }

    /// Get the current slot (similar to block number)
    pub fn get_slot(&self) -> AppResult<u64> {
        self.client
            .get_slot()
            .map_err(|e| AppError::Solana(format!("Failed to get slot: {}", e)))
    }

    /// Get balance of an address in lamports (1 SOL = 1_000_000_000 lamports)
    pub fn get_balance(&self, address: &str) -> AppResult<u64> {
        let pubkey = Pubkey::from_str(address)
            .map_err(|e| AppError::InvalidAddress(format!("Invalid Solana address: {}", e)))?;

        self.client
            .get_balance(&pubkey)
            .map_err(|e| AppError::Solana(format!("Failed to get balance: {}", e)))
    }

    /// Get balance formatted as SOL string
    pub fn get_balance_sol(&self, address: &str) -> AppResult<String> {
        let lamports = self.get_balance(address)?;
        Ok(Self::lamports_to_sol(lamports))
    }

    /// Get transaction status
    pub fn get_transaction_status(&self, signature: &str) -> AppResult<Option<SolanaTransactionStatus>> {
        let sig = Signature::from_str(signature)
            .map_err(|e| AppError::Solana(format!("Invalid signature: {}", e)))?;

        let status = self
            .client
            .get_signature_status(&sig)
            .map_err(|e| AppError::Solana(format!("Failed to get transaction status: {}", e)))?;

        match status {
            Some(result) => {
                match result {
                    Ok(_) => Ok(Some(SolanaTransactionStatus {
                        confirmed: true,
                        error: None,
                    })),
                    Err(e) => Ok(Some(SolanaTransactionStatus {
                        confirmed: false,
                        error: Some(e.to_string()),
                    })),
                }
            }
            None => Ok(None),
        }
    }

    /// Get transaction details
    pub fn get_transaction(
        &self,
        signature: &str,
    ) -> AppResult<Option<SolanaTransactionInfo>> {
        let sig = Signature::from_str(signature)
            .map_err(|e| AppError::Solana(format!("Invalid signature: {}", e)))?;

        let tx = self
            .client
            .get_transaction(&sig, solana_transaction_status::UiTransactionEncoding::Json)
            .ok();

        match tx {
            Some(confirmed_tx) => {
                let slot = confirmed_tx.slot;
                let block_time = confirmed_tx.block_time;

                // Extract basic info
                Ok(Some(SolanaTransactionInfo {
                    signature: signature.to_string(),
                    slot,
                    block_time,
                    success: confirmed_tx.transaction.meta
                        .map(|m| m.err.is_none())
                        .unwrap_or(false),
                }))
            }
            None => Ok(None),
        }
    }

    /// Get number of confirmations for a transaction
    pub fn get_confirmations(&self, signature: &str) -> AppResult<u64> {
        let sig = Signature::from_str(signature)
            .map_err(|e| AppError::Solana(format!("Invalid signature: {}", e)))?;

        let statuses = self
            .client
            .get_signature_statuses(&[sig])
            .map_err(|e| AppError::Solana(format!("Failed to get signature status: {}", e)))?;

        if let Some(Some(status)) = statuses.value.first() {
            Ok(status.confirmations.unwrap_or(0) as u64)
        } else {
            Ok(0)
        }
    }

    /// Verify a payment transaction
    pub fn verify_payment(
        &self,
        signature: &str,
        expected_to: &str,
        expected_amount_lamports: u64,
    ) -> AppResult<SolanaPaymentVerification> {
        let sig = Signature::from_str(signature)
            .map_err(|e| AppError::Solana(format!("Invalid signature: {}", e)))?;

        let tx = self
            .client
            .get_transaction(&sig, solana_transaction_status::UiTransactionEncoding::JsonParsed)
            .map_err(|e| AppError::Solana(format!("Transaction not found: {}", e)))?;

        let is_successful = tx.transaction.meta
            .as_ref()
            .map(|m| m.err.is_none())
            .unwrap_or(false);

        let confirmations = self.get_confirmations(signature)?;

        // For full payment verification, you'd parse the transaction instructions
        // This is a simplified version
        Ok(SolanaPaymentVerification {
            is_valid: is_successful,
            is_successful,
            confirmations,
            slot: tx.slot,
        })
    }

    /// Validate a Solana address
    pub fn validate_address(address: &str) -> bool {
        Pubkey::from_str(address).is_ok()
    }

    /// Convert lamports to SOL
    pub fn lamports_to_sol(lamports: u64) -> String {
        let sol = lamports as f64 / 1_000_000_000.0;
        format!("{:.9}", sol)
    }

    /// Convert SOL to lamports
    pub fn sol_to_lamports(sol: f64) -> u64 {
        (sol * 1_000_000_000.0) as u64
    }

    /// Verify an ed25519 signature
    pub fn verify_signature(
        address: &str,
        message: &[u8],
        signature: &[u8],
    ) -> AppResult<bool> {
        use ed25519_dalek::{Signature as Ed25519Signature, Verifier, VerifyingKey};

        let pubkey = Pubkey::from_str(address)
            .map_err(|e| AppError::InvalidAddress(format!("Invalid address: {}", e)))?;

        let pubkey_bytes: [u8; 32] = pubkey.to_bytes();
        let verifying_key = VerifyingKey::from_bytes(&pubkey_bytes)
            .map_err(|e| AppError::InvalidSignature(format!("Invalid public key: {}", e)))?;

        if signature.len() != 64 {
            return Err(AppError::InvalidSignature(
                "Signature must be 64 bytes".to_string(),
            ));
        }

        let sig_bytes: [u8; 64] = signature.try_into()
            .map_err(|_| AppError::InvalidSignature("Invalid signature length".to_string()))?;
        let sig = Ed25519Signature::from_bytes(&sig_bytes);

        Ok(verifying_key.verify(message, &sig).is_ok())
    }

    /// Get required confirmations based on amount
    pub fn get_required_confirmations(amount_lamports: u64) -> i32 {
        let sol = amount_lamports as f64 / 1_000_000_000.0;

        if sol > 100.0 {
            32 // Maximum confirmations
        } else if sol > 10.0 {
            16
        } else {
            1 // Solana has fast finality
        }
    }
}

#[derive(Debug, Clone)]
pub struct SolanaTransactionStatus {
    pub confirmed: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SolanaTransactionInfo {
    pub signature: String,
    pub slot: u64,
    pub block_time: Option<i64>,
    pub success: bool,
}

#[derive(Debug, Clone)]
pub struct SolanaPaymentVerification {
    pub is_valid: bool,
    pub is_successful: bool,
    pub confirmations: u64,
    pub slot: u64,
}

/// SPL Token interactions
impl SolanaService {
    /// Get SPL token balance for an address
    pub fn get_token_balance(
        &self,
        token_account: &str,
    ) -> AppResult<u64> {
        let pubkey = Pubkey::from_str(token_account)
            .map_err(|e| AppError::InvalidAddress(format!("Invalid token account: {}", e)))?;

        let balance = self
            .client
            .get_token_account_balance(&pubkey)
            .map_err(|e| AppError::Solana(format!("Failed to get token balance: {}", e)))?;

        balance
            .amount
            .parse()
            .map_err(|e| AppError::Solana(format!("Failed to parse token balance: {}", e)))
    }
}
