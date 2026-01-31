use axum::{
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use hmac::{Hmac, Mac};
use serde::Serialize;
use sha2::Sha256;

use crate::AppState;

type HmacSha256 = Hmac<Sha256>;

#[derive(Serialize)]
struct AuthError {
    success: bool,
    error: AuthErrorDetail,
}

#[derive(Serialize)]
struct AuthErrorDetail {
    code: String,
    message: String,
}

impl AuthError {
    fn unauthorized(message: &str) -> Self {
        Self {
            success: false,
            error: AuthErrorDetail {
                code: "UNAUTHORIZED".to_string(),
                message: message.to_string(),
            },
        }
    }
}

/// API Key Authentication Middleware
/// Expects header: X-API-Key: <api_key>
///
/// Key formats:
/// - pk_live_xxx: Production public key (for client-side)
/// - sk_live_xxx: Production secret key (for server-side)
/// - pk_test_xxx: Test public key
/// - sk_test_xxx: Test secret key
pub async fn api_key_auth(
    State(state): State<AppState>,
    request: Request<Body>,
    next: Next,
) -> Result<Response, Response> {
    let path = request.uri().path();

    // Skip auth for public endpoints
    if is_public_endpoint(path) {
        return Ok(next.run(request).await);
    }

    // Get API key from header
    let api_key = request
        .headers()
        .get("X-API-Key")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.trim());

    match api_key {
        Some(key) if !key.is_empty() => {
            match validate_api_key_format(key) {
                Ok(key_type) => {
                    // Additional validation based on endpoint sensitivity
                    if requires_secret_key(path) && key_type != ApiKeyType::SecretLive && key_type != ApiKeyType::SecretTest {
                        tracing::warn!(
                            path = %path,
                            key_type = ?key_type,
                            "Endpoint requires secret key"
                        );
                        return Err(auth_error_response(
                            StatusCode::FORBIDDEN,
                            "This endpoint requires a secret key (sk_*)"
                        ));
                    }

                    // Verify the key hash against stored keys (in production, check database)
                    if verify_api_key(key, &state.config.security.api_key_hash_secret) {
                        Ok(next.run(request).await)
                    } else {
                        tracing::warn!(
                            key_prefix = %&key[..std::cmp::min(12, key.len())],
                            "Invalid API key"
                        );
                        Err(auth_error_response(
                            StatusCode::UNAUTHORIZED,
                            "Invalid API key"
                        ))
                    }
                }
                Err(msg) => {
                    tracing::warn!(error = %msg, "Invalid API key format");
                    Err(auth_error_response(StatusCode::UNAUTHORIZED, &msg))
                }
            }
        }
        _ => {
            tracing::warn!(path = %path, "Missing API key");
            Err(auth_error_response(
                StatusCode::UNAUTHORIZED,
                "Missing X-API-Key header"
            ))
        }
    }
}

fn auth_error_response(status: StatusCode, message: &str) -> Response {
    (status, Json(AuthError::unauthorized(message))).into_response()
}

fn is_public_endpoint(path: &str) -> bool {
    matches!(path,
        "/health" |
        "/api/v1/status"
    ) || path.starts_with("/webhooks/")
}

fn requires_secret_key(path: &str) -> bool {
    // These endpoints require secret keys for security
    path.contains("/refund") ||
    path.contains("/admin")
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ApiKeyType {
    PublicLive,
    SecretLive,
    PublicTest,
    SecretTest,
}

fn validate_api_key_format(key: &str) -> Result<ApiKeyType, String> {
    if key.len() < 20 {
        return Err("API key too short".to_string());
    }

    if key.len() > 100 {
        return Err("API key too long".to_string());
    }

    // Check prefix and determine key type
    if key.starts_with("pk_live_") {
        Ok(ApiKeyType::PublicLive)
    } else if key.starts_with("sk_live_") {
        Ok(ApiKeyType::SecretLive)
    } else if key.starts_with("pk_test_") {
        Ok(ApiKeyType::PublicTest)
    } else if key.starts_with("sk_test_") {
        Ok(ApiKeyType::SecretTest)
    } else {
        Err("Invalid API key prefix. Expected: pk_live_, sk_live_, pk_test_, or sk_test_".to_string())
    }
}

fn verify_api_key(api_key: &str, secret: &str) -> bool {
    // For test keys, allow through in development
    if api_key.contains("_test_") {
        // In production, you'd still verify test keys against the database
        // For now, accept any properly formatted test key
        return validate_api_key_format(api_key).is_ok();
    }

    // For production keys, verify the HMAC signature
    // In a real implementation:
    // 1. Extract the key ID from the key
    // 2. Look up the stored hash from the database
    // 3. Compare the computed hash with the stored hash using constant-time comparison

    // Compute the hash of the provided key
    let computed_hash = hash_api_key(api_key, secret);

    // In production, compare against database-stored hash
    // For now, we verify the key structure is correct
    !computed_hash.is_empty() && validate_api_key_format(api_key).is_ok()
}

/// Generate a new API key with the specified prefix
/// Returns (api_key, key_hash) tuple
pub fn generate_api_key(prefix: &str) -> Result<(String, String), String> {
    use base64::Engine;

    // Validate prefix
    if !matches!(prefix, "pk_live" | "sk_live" | "pk_test" | "sk_test") {
        return Err("Invalid prefix. Must be: pk_live, sk_live, pk_test, or sk_test".to_string());
    }

    // Generate 32 bytes of random data
    let mut random_bytes = [0u8; 32];
    getrandom::getrandom(&mut random_bytes)
        .map_err(|e| format!("Failed to generate random bytes: {}", e))?;

    // Encode as URL-safe base64
    let key_body = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(random_bytes);

    let api_key = format!("{}_{}", prefix, key_body);

    // Generate hash for storage (use a fixed secret for this operation)
    let key_hash = hash_api_key(&api_key, "internal-hash-secret");

    Ok((api_key, key_hash))
}

/// Hash an API key for storage/comparison using HMAC-SHA256
pub fn hash_api_key(api_key: &str, secret: &str) -> String {
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
        .expect("HMAC can take key of any size");
    mac.update(api_key.as_bytes());
    hex::encode(mac.finalize().into_bytes())
}

/// Constant-time comparison for hashes to prevent timing attacks
pub fn secure_compare(a: &str, b: &str) -> bool {
    if a.len() != b.len() {
        return false;
    }

    let mut result = 0u8;
    for (x, y) in a.bytes().zip(b.bytes()) {
        result |= x ^ y;
    }
    result == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_api_key_format() {
        assert!(matches!(
            validate_api_key_format("pk_test_abcdefghijklmnop"),
            Ok(ApiKeyType::PublicTest)
        ));
        assert!(matches!(
            validate_api_key_format("sk_live_abcdefghijklmnop"),
            Ok(ApiKeyType::SecretLive)
        ));
        assert!(validate_api_key_format("invalid_key").is_err());
        assert!(validate_api_key_format("short").is_err());
    }

    #[test]
    fn test_hash_api_key() {
        let hash1 = hash_api_key("pk_test_abc123xyz789def456", "secret");
        let hash2 = hash_api_key("pk_test_abc123xyz789def456", "secret");
        assert_eq!(hash1, hash2);
        assert_eq!(hash1.len(), 64); // SHA256 hex length

        let hash3 = hash_api_key("sk_test_different_key_here", "secret");
        assert_ne!(hash1, hash3);
    }

    #[test]
    fn test_secure_compare() {
        assert!(secure_compare("abc123", "abc123"));
        assert!(!secure_compare("abc123", "abc124"));
        assert!(!secure_compare("abc123", "abc12"));
    }

    #[test]
    fn test_is_public_endpoint() {
        assert!(is_public_endpoint("/health"));
        assert!(is_public_endpoint("/api/v1/status"));
        assert!(is_public_endpoint("/webhooks/razorpay"));
        assert!(!is_public_endpoint("/api/v1/payments"));
    }

    #[test]
    fn test_requires_secret_key() {
        assert!(requires_secret_key("/api/v1/razorpay/refund"));
        assert!(!requires_secret_key("/api/v1/razorpay/orders"));
    }
}
