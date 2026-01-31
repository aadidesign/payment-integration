use crate::error::{AppError, AppResult};
use crate::models::ChainType;

/// Address validation utilities for different blockchain networks
pub struct AddressValidator;

impl AddressValidator {
    /// Validate an address for a specific chain
    pub fn validate(address: &str, chain: &ChainType) -> bool {
        match chain {
            ChainType::Ethereum | ChainType::Polygon | ChainType::Bsc | ChainType::Arbitrum => {
                Self::is_valid_evm_address(address)
            }
            ChainType::Solana => Self::is_valid_solana_address(address),
            ChainType::Bitcoin => Self::is_valid_bitcoin_address(address),
        }
    }

    /// Validate Ethereum/EVM address (0x-prefixed, 40 hex chars)
    pub fn is_valid_evm_address(address: &str) -> bool {
        if !address.starts_with("0x") {
            return false;
        }

        let hex_part = &address[2..];
        if hex_part.len() != 40 {
            return false;
        }

        hex_part.chars().all(|c| c.is_ascii_hexdigit())
    }

    /// Validate Solana address (Base58, 32-44 chars)
    pub fn is_valid_solana_address(address: &str) -> bool {
        if address.len() < 32 || address.len() > 44 {
            return false;
        }

        // Check for valid Base58 characters
        address.chars().all(|c| {
            c.is_ascii_alphanumeric() && c != '0' && c != 'O' && c != 'I' && c != 'l'
        })
    }

    /// Validate Bitcoin address (P2PKH, P2SH, or Bech32)
    pub fn is_valid_bitcoin_address(address: &str) -> bool {
        let len = address.len();

        // P2PKH (starts with 1) or P2SH (starts with 3)
        if (address.starts_with('1') || address.starts_with('3')) && len >= 25 && len <= 34 {
            return Self::is_valid_base58check(address);
        }

        // Bech32 (starts with bc1)
        if address.starts_with("bc1") && len >= 42 && len <= 62 {
            return Self::is_valid_bech32(address);
        }

        // Testnet addresses
        if (address.starts_with('m') || address.starts_with('n') || address.starts_with('2'))
            && len >= 25
            && len <= 34
        {
            return Self::is_valid_base58check(address);
        }

        if address.starts_with("tb1") && len >= 42 && len <= 62 {
            return Self::is_valid_bech32(address);
        }

        false
    }

    fn is_valid_base58check(address: &str) -> bool {
        // Base58 alphabet (no 0, O, I, l)
        address
            .chars()
            .all(|c| c.is_ascii_alphanumeric() && c != '0' && c != 'O' && c != 'I' && c != 'l')
    }

    fn is_valid_bech32(address: &str) -> bool {
        // Bech32 alphabet (lowercase alphanumeric, no 1, b, i, o)
        let data = &address[3..]; // Skip "bc1" or "tb1"
        data.chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
            && !data.contains('1')
            && !data.contains('b')
            && !data.contains('i')
            && !data.contains('o')
    }

    /// Normalize an EVM address to checksum format
    pub fn normalize_evm_address(address: &str) -> AppResult<String> {
        if !Self::is_valid_evm_address(address) {
            return Err(AppError::InvalidAddress("Invalid EVM address".to_string()));
        }

        // For now, just lowercase. In production, implement EIP-55 checksum
        Ok(address.to_lowercase())
    }

    /// Get the chain type from an address (heuristic)
    pub fn detect_chain(address: &str) -> Option<ChainType> {
        if address.starts_with("0x") && address.len() == 42 {
            Some(ChainType::Ethereum) // Could be any EVM chain
        } else if Self::is_valid_solana_address(address) {
            Some(ChainType::Solana)
        } else if Self::is_valid_bitcoin_address(address) {
            Some(ChainType::Bitcoin)
        } else {
            None
        }
    }
}

/// Checksum utilities
pub mod checksum {
    use sha2::{Digest, Sha256};

    /// Double SHA256 hash (used in Bitcoin)
    pub fn double_sha256(data: &[u8]) -> [u8; 32] {
        let first = Sha256::digest(data);
        let second = Sha256::digest(&first);
        let mut result = [0u8; 32];
        result.copy_from_slice(&second);
        result
    }

    /// Keccak256 hash (used in Ethereum)
    pub fn keccak256(data: &[u8]) -> [u8; 32] {
        use sha3::{Digest as Sha3Digest, Keccak256};
        let mut hasher = Keccak256::new();
        hasher.update(data);
        let result = hasher.finalize();
        let mut output = [0u8; 32];
        output.copy_from_slice(&result);
        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_evm_address_validation() {
        assert!(AddressValidator::is_valid_evm_address(
            "0x742d35Cc6634C0532925a3b844Bc9e7595f1E8e4"
        ));
        assert!(!AddressValidator::is_valid_evm_address("invalid"));
        assert!(!AddressValidator::is_valid_evm_address("0x123")); // Too short
    }

    #[test]
    fn test_solana_address_validation() {
        // System program address
        assert!(AddressValidator::is_valid_solana_address(
            "11111111111111111111111111111111"
        ));
        assert!(!AddressValidator::is_valid_solana_address("0xabc")); // Wrong format
    }

    #[test]
    fn test_bitcoin_address_validation() {
        // P2PKH
        assert!(AddressValidator::is_valid_bitcoin_address(
            "1BvBMSEYstWetqTFn5Au4m4GFg7xJaNVN2"
        ));
        // P2SH
        assert!(AddressValidator::is_valid_bitcoin_address(
            "3J98t1WpEZ73CNmQviecrnyiWrnqRhWNLy"
        ));
        // Bech32
        assert!(AddressValidator::is_valid_bitcoin_address(
            "bc1qar0srrr7xfkvy5l643lydnw9re59gtzzwf5mdq"
        ));
    }

    #[test]
    fn test_chain_detection() {
        assert_eq!(
            AddressValidator::detect_chain("0x742d35Cc6634C0532925a3b844Bc9e7595f1E8e4"),
            Some(ChainType::Ethereum)
        );
    }
}
