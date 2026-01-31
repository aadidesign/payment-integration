use crate::error::{AppError, AppResult};
use crate::models::ChainType;

/// WalletConnect signature verification for multi-chain support
/// Supports MetaMask, Trust Wallet, and other WalletConnect-compatible wallets
pub struct WalletConnectVerifier;

impl WalletConnectVerifier {
    /// Verify a message signature from any supported wallet
    pub fn verify_signature(
        address: &str,
        message: &str,
        signature: &str,
        chain: &ChainType,
    ) -> AppResult<bool> {
        match chain {
            ChainType::Ethereum | ChainType::Polygon | ChainType::Bsc | ChainType::Arbitrum => {
                Self::verify_evm_signature(address, message, signature)
            }
            ChainType::Solana => Self::verify_solana_signature(address, message, signature),
            ChainType::Bitcoin => Err(AppError::InvalidSignature(
                "Bitcoin signature verification not supported via WalletConnect".to_string(),
            )),
        }
    }

    /// Verify EIP-191 personal_sign signature (Ethereum and EVM chains)
    fn verify_evm_signature(address: &str, message: &str, signature: &str) -> AppResult<bool> {
        use ethers::types::Address;
        use ethers::core::types::Signature;

        let address: Address = address
            .parse()
            .map_err(|e| AppError::InvalidAddress(format!("Invalid EVM address: {}", e)))?;

        let signature_bytes = hex::decode(signature.trim_start_matches("0x"))
            .map_err(|e| AppError::InvalidSignature(format!("Invalid signature format: {}", e)))?;

        if signature_bytes.len() != 65 {
            return Err(AppError::InvalidSignature(
                "EVM signature must be 65 bytes".to_string(),
            ));
        }

        let signature = Signature::try_from(signature_bytes.as_slice())
            .map_err(|e| AppError::InvalidSignature(format!("Invalid signature: {}", e)))?;

        let recovered = signature
            .recover(message)
            .map_err(|e| AppError::InvalidSignature(format!("Failed to recover address: {}", e)))?;

        Ok(recovered == address)
    }

    /// Verify Solana signature (Phantom wallet compatible)
    fn verify_solana_signature(address: &str, message: &str, signature: &str) -> AppResult<bool> {
        use ed25519_dalek::{Signature as Ed25519Signature, Verifier, VerifyingKey};
        use solana_sdk::pubkey::Pubkey;
        use std::str::FromStr;

        let pubkey = Pubkey::from_str(address)
            .map_err(|e| AppError::InvalidAddress(format!("Invalid Solana address: {}", e)))?;

        let signature_bytes = if signature.starts_with("0x") {
            hex::decode(signature.trim_start_matches("0x"))
        } else {
            // Try base58 decoding (common for Solana)
            bs58::decode(signature).into_vec().or_else(|_| hex::decode(signature))
        }
        .map_err(|e| AppError::InvalidSignature(format!("Invalid signature format: {}", e)))?;

        if signature_bytes.len() != 64 {
            return Err(AppError::InvalidSignature(
                "Solana signature must be 64 bytes".to_string(),
            ));
        }

        let pubkey_bytes: [u8; 32] = pubkey.to_bytes();
        let verifying_key = VerifyingKey::from_bytes(&pubkey_bytes)
            .map_err(|e| AppError::InvalidSignature(format!("Invalid public key: {}", e)))?;

        let sig_bytes: [u8; 64] = signature_bytes.try_into()
            .map_err(|_| AppError::InvalidSignature("Invalid signature length".to_string()))?;
        let sig = Ed25519Signature::from_bytes(&sig_bytes);

        Ok(verifying_key.verify(message.as_bytes(), &sig).is_ok())
    }

    /// Create a sign message for payment verification
    pub fn create_payment_message(
        payment_id: &str,
        amount: &str,
        currency: &str,
        timestamp: i64,
    ) -> String {
        format!(
            "Sign this message to verify your payment:\n\nPayment ID: {}\nAmount: {} {}\nTimestamp: {}",
            payment_id, amount, currency, timestamp
        )
    }

    /// Validate address format for a given chain
    pub fn validate_address(address: &str, chain: &ChainType) -> bool {
        match chain {
            ChainType::Ethereum | ChainType::Polygon | ChainType::Bsc | ChainType::Arbitrum => {
                Self::is_valid_evm_address(address)
            }
            ChainType::Solana => Self::is_valid_solana_address(address),
            ChainType::Bitcoin => Self::is_valid_bitcoin_address(address),
        }
    }

    fn is_valid_evm_address(address: &str) -> bool {
        use ethers::types::Address;
        address.parse::<Address>().is_ok()
    }

    fn is_valid_solana_address(address: &str) -> bool {
        use solana_sdk::pubkey::Pubkey;
        use std::str::FromStr;
        Pubkey::from_str(address).is_ok()
    }

    fn is_valid_bitcoin_address(address: &str) -> bool {
        // Basic validation for Bitcoin addresses
        // P2PKH: starts with 1, 25-34 chars
        // P2SH: starts with 3, 25-34 chars
        // Bech32: starts with bc1, 42-62 chars
        let len = address.len();

        if address.starts_with("bc1") {
            len >= 42 && len <= 62
        } else if address.starts_with('1') || address.starts_with('3') {
            len >= 25 && len <= 34
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_evm_address() {
        assert!(WalletConnectVerifier::validate_address(
            "0x742d35Cc6634C0532925a3b844Bc9e7595f1E8e4",
            &ChainType::Ethereum
        ));
        assert!(!WalletConnectVerifier::validate_address(
            "invalid_address",
            &ChainType::Ethereum
        ));
    }

    #[test]
    fn test_validate_solana_address() {
        assert!(WalletConnectVerifier::validate_address(
            "11111111111111111111111111111111",
            &ChainType::Solana
        ));
    }

    #[test]
    fn test_payment_message_creation() {
        let message = WalletConnectVerifier::create_payment_message(
            "pay_123",
            "1.5",
            "ETH",
            1234567890,
        );
        assert!(message.contains("pay_123"));
        assert!(message.contains("1.5 ETH"));
    }
}
