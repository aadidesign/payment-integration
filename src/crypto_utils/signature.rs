use hmac::{Hmac, Mac};
use sha2::Sha256;

use crate::error::{AppError, AppResult};

type HmacSha256 = Hmac<Sha256>;

/// HMAC-SHA256 signature generation and verification
pub struct HmacSignature;

impl HmacSignature {
    /// Generate HMAC-SHA256 signature
    pub fn sign(message: &[u8], secret: &[u8]) -> AppResult<Vec<u8>> {
        let mut mac = HmacSha256::new_from_slice(secret)
            .map_err(|e| AppError::Internal(format!("HMAC initialization failed: {}", e)))?;

        mac.update(message);
        Ok(mac.finalize().into_bytes().to_vec())
    }

    /// Generate HMAC-SHA256 signature as hex string
    pub fn sign_hex(message: &[u8], secret: &[u8]) -> AppResult<String> {
        let signature = Self::sign(message, secret)?;
        Ok(hex::encode(signature))
    }

    /// Verify HMAC-SHA256 signature
    pub fn verify(message: &[u8], signature: &[u8], secret: &[u8]) -> AppResult<bool> {
        let mut mac = HmacSha256::new_from_slice(secret)
            .map_err(|e| AppError::Internal(format!("HMAC initialization failed: {}", e)))?;

        mac.update(message);

        Ok(mac.verify_slice(signature).is_ok())
    }

    /// Verify HMAC-SHA256 signature from hex string
    pub fn verify_hex(message: &[u8], signature_hex: &str, secret: &[u8]) -> AppResult<bool> {
        let signature = hex::decode(signature_hex)
            .map_err(|e| AppError::InvalidSignature(format!("Invalid hex signature: {}", e)))?;

        Self::verify(message, &signature, secret)
    }
}

/// Encrypt sensitive data using AES-256-GCM
pub mod encryption {
    use ring::aead::{self, Aad, BoundKey, Nonce, NonceSequence, NONCE_LEN, UnboundKey};
    use ring::error::Unspecified;
    use ring::rand::{SecureRandom, SystemRandom};

    use crate::error::{AppError, AppResult};

    struct CounterNonceSequence(u64);

    impl NonceSequence for CounterNonceSequence {
        fn advance(&mut self) -> Result<Nonce, Unspecified> {
            let mut nonce_bytes = [0u8; NONCE_LEN];
            nonce_bytes[4..].copy_from_slice(&self.0.to_be_bytes());
            self.0 += 1;
            Nonce::try_assume_unique_for_key(&nonce_bytes)
        }
    }

    /// Generate a random encryption key (32 bytes for AES-256)
    pub fn generate_key() -> AppResult<[u8; 32]> {
        let rng = SystemRandom::new();
        let mut key = [0u8; 32];
        rng.fill(&mut key)
            .map_err(|_| AppError::Internal("Failed to generate random key".to_string()))?;
        Ok(key)
    }

    /// Encrypt data using AES-256-GCM
    pub fn encrypt(plaintext: &[u8], key: &[u8]) -> AppResult<Vec<u8>> {
        if key.len() != 32 {
            return Err(AppError::Internal("Key must be 32 bytes".to_string()));
        }

        let rng = SystemRandom::new();

        // Generate random nonce
        let mut nonce = [0u8; NONCE_LEN];
        rng.fill(&mut nonce)
            .map_err(|_| AppError::Internal("Failed to generate nonce".to_string()))?;

        let unbound_key = UnboundKey::new(&aead::AES_256_GCM, key)
            .map_err(|_| AppError::Internal("Failed to create encryption key".to_string()))?;

        let mut sealing_key = aead::SealingKey::new(unbound_key, CounterNonceSequence(0));

        let mut in_out = plaintext.to_vec();
        sealing_key
            .seal_in_place_append_tag(Aad::empty(), &mut in_out)
            .map_err(|_| AppError::Internal("Encryption failed".to_string()))?;

        // Prepend nonce to ciphertext
        let mut result = nonce.to_vec();
        result.extend(in_out);

        Ok(result)
    }

    /// Decrypt data using AES-256-GCM
    pub fn decrypt(ciphertext: &[u8], key: &[u8]) -> AppResult<Vec<u8>> {
        if key.len() != 32 {
            return Err(AppError::Internal("Key must be 32 bytes".to_string()));
        }

        if ciphertext.len() < NONCE_LEN {
            return Err(AppError::Internal("Ciphertext too short".to_string()));
        }

        let (nonce, encrypted) = ciphertext.split_at(NONCE_LEN);

        let unbound_key = UnboundKey::new(&aead::AES_256_GCM, key)
            .map_err(|_| AppError::Internal("Failed to create decryption key".to_string()))?;

        let nonce = Nonce::try_assume_unique_for_key(nonce)
            .map_err(|_| AppError::Internal("Invalid nonce".to_string()))?;

        let mut opening_key = aead::OpeningKey::new(unbound_key, CounterNonceSequence(0));

        let mut in_out = encrypted.to_vec();
        let decrypted = opening_key
            .open_in_place(Aad::empty(), &mut in_out)
            .map_err(|_| AppError::Internal("Decryption failed".to_string()))?;

        Ok(decrypted.to_vec())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hmac_sign_verify() {
        let message = b"test message";
        let secret = b"secret key";

        let signature = HmacSignature::sign(message, secret).unwrap();
        assert!(HmacSignature::verify(message, &signature, secret).unwrap());
    }

    #[test]
    fn test_hmac_hex() {
        let message = b"test message";
        let secret = b"secret key";

        let signature_hex = HmacSignature::sign_hex(message, secret).unwrap();
        assert!(HmacSignature::verify_hex(message, &signature_hex, secret).unwrap());
    }
}
