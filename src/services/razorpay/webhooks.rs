use hmac::{Hmac, Mac};
use sha2::Sha256;

use crate::error::{AppError, AppResult};

type HmacSha256 = Hmac<Sha256>;

pub struct RazorpayWebhookVerifier;

impl RazorpayWebhookVerifier {
    /// Verify webhook signature from Razorpay
    /// Razorpay sends signature in X-Razorpay-Signature header
    pub fn verify_webhook_signature(
        payload: &[u8],
        signature: &str,
        secret: &str,
    ) -> AppResult<bool> {
        let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
            .map_err(|e| AppError::Internal(format!("HMAC initialization failed: {}", e)))?;

        mac.update(payload);

        let expected_signature = hex::encode(mac.finalize().into_bytes());

        if expected_signature == signature {
            Ok(true)
        } else {
            Err(AppError::WebhookVerification(
                "Invalid webhook signature".to_string(),
            ))
        }
    }

    /// Verify payment signature for checkout verification
    /// signature = HMAC-SHA256(order_id + "|" + payment_id, secret)
    pub fn verify_payment_signature(
        order_id: &str,
        payment_id: &str,
        signature: &str,
        secret: &str,
    ) -> AppResult<bool> {
        let payload = format!("{}|{}", order_id, payment_id);

        let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
            .map_err(|e| AppError::Internal(format!("HMAC initialization failed: {}", e)))?;

        mac.update(payload.as_bytes());

        let expected_signature = hex::encode(mac.finalize().into_bytes());

        if expected_signature == signature {
            Ok(true)
        } else {
            Err(AppError::InvalidSignature(
                "Payment signature verification failed".to_string(),
            ))
        }
    }

    /// Verify subscription signature
    /// signature = HMAC-SHA256(payment_id + "|" + subscription_id, secret)
    pub fn verify_subscription_signature(
        payment_id: &str,
        subscription_id: &str,
        signature: &str,
        secret: &str,
    ) -> AppResult<bool> {
        let payload = format!("{}|{}", payment_id, subscription_id);

        let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
            .map_err(|e| AppError::Internal(format!("HMAC initialization failed: {}", e)))?;

        mac.update(payload.as_bytes());

        let expected_signature = hex::encode(mac.finalize().into_bytes());

        if expected_signature == signature {
            Ok(true)
        } else {
            Err(AppError::InvalidSignature(
                "Subscription signature verification failed".to_string(),
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_payment_signature_verification() {
        // Test with known values
        let order_id = "order_DBJOWzybf0sJbb";
        let payment_id = "pay_DGR9FPNxfgIqvp";
        let secret = "EnAtY1HnJlrGZfbVJqKMKfVP";

        // Calculate expected signature
        let payload = format!("{}|{}", order_id, payment_id);
        let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).unwrap();
        mac.update(payload.as_bytes());
        let signature = hex::encode(mac.finalize().into_bytes());

        let result =
            RazorpayWebhookVerifier::verify_payment_signature(order_id, payment_id, &signature, secret);

        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[test]
    fn test_invalid_signature() {
        let order_id = "order_test";
        let payment_id = "pay_test";
        let invalid_signature = "invalid_signature";
        let secret = "test_secret";

        let result = RazorpayWebhookVerifier::verify_payment_signature(
            order_id,
            payment_id,
            invalid_signature,
            secret,
        );

        assert!(result.is_err());
    }
}
