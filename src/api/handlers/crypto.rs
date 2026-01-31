use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

use crate::db::repositories::PaymentRepository;
use crate::error::{AppError, AppResult};
use crate::models::{
    BalanceResponse, ChainType, CreatePaymentRequest, CurrencyType,
    PaymentMethod, PaymentResponse, PaymentStatus,
};
use crate::services::crypto::WalletConnectVerifier;
use crate::AppState;

#[derive(Debug, Deserialize, Validate)]
pub struct CreateCryptoPaymentRequest {
    #[validate(range(min = 1, message = "Amount must be positive"))]
    pub amount: i64,
    #[validate(length(min = 2, max = 10, message = "Invalid currency format"))]
    pub currency: String,
    #[validate(length(min = 3, max = 20, message = "Invalid chain format"))]
    pub chain: String,
    #[serde(default)]
    #[validate(length(max = 255, message = "Description too long"))]
    pub description: Option<String>,
    #[serde(default)]
    #[validate(email(message = "Invalid email format"))]
    pub customer_email: Option<String>,
    #[serde(default)]
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Serialize)]
pub struct CreateCryptoPaymentResponse {
    pub success: bool,
    pub payment_id: Uuid,
    pub chain: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub address: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lightning_invoice: Option<String>,
    pub amount: i64,
    pub currency: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
    pub instructions: String,
}

pub async fn create_crypto_payment(
    State(state): State<AppState>,
    Json(request): Json<CreateCryptoPaymentRequest>,
) -> AppResult<Json<CreateCryptoPaymentResponse>> {
    // Validate request
    request.validate().map_err(|e| {
        AppError::Validation(format!("Invalid request: {}", e))
    })?;

    // Parse and validate chain
    let chain_type: ChainType = request
        .chain
        .parse()
        .map_err(|e| AppError::Validation(format!(
            "Invalid chain: {}. Supported: ethereum, polygon, bsc, arbitrum, solana, bitcoin", e
        )))?;

    // Determine currency and payment method based on chain
    let (currency, method) = match chain_type {
        ChainType::Ethereum => (CurrencyType::ETH, PaymentMethod::Ethereum),
        ChainType::Polygon => (CurrencyType::MATIC, PaymentMethod::Polygon),
        ChainType::Bsc => (CurrencyType::BNB, PaymentMethod::Bsc),
        ChainType::Arbitrum => (CurrencyType::ETH, PaymentMethod::Arbitrum),
        ChainType::Solana => (CurrencyType::SOL, PaymentMethod::Solana),
        ChainType::Bitcoin => (CurrencyType::BTC, PaymentMethod::Lightning),
    };

    let payment_request = CreatePaymentRequest {
        amount: request.amount,
        currency,
        method,
        description: request.description,
        customer_email: request.customer_email,
        customer_phone: None,
        metadata: request.metadata,
        callback_url: None,
    };

    let result = state
        .payment_processor
        .create_payment(&state.db, &payment_request)
        .await?;

    let instructions = match chain_type {
        ChainType::Bitcoin => "Pay the Lightning invoice using any Lightning-compatible wallet".to_string(),
        _ => format!(
            "Send exactly {} {} to the provided address. Transaction will be confirmed after required block confirmations.",
            request.amount, request.currency
        ),
    };

    tracing::info!(
        payment_id = %result.payment_id,
        chain = %chain_type,
        amount = request.amount,
        "Crypto payment created"
    );

    Ok(Json(CreateCryptoPaymentResponse {
        success: true,
        payment_id: result.payment_id,
        chain: chain_type.to_string(),
        address: result.crypto_address,
        lightning_invoice: result.lightning_invoice,
        amount: request.amount,
        currency: request.currency,
        expires_at: result.expires_at,
        instructions,
    }))
}

pub async fn get_crypto_payment(
    State(state): State<AppState>,
    axum::extract::Path(payment_id): axum::extract::Path<Uuid>,
) -> AppResult<Json<PaymentResponse>> {
    let payment = PaymentRepository::find_by_id(&state.db, payment_id).await?;
    Ok(Json(payment.into()))
}

#[derive(Debug, Deserialize, Validate)]
pub struct VerifyCryptoTransactionRequest {
    pub payment_id: Uuid,
    #[validate(length(min = 32, max = 128, message = "Invalid transaction hash format"))]
    pub tx_hash: String,
}

#[derive(Debug, Serialize)]
pub struct VerifyCryptoTransactionResponse {
    pub success: bool,
    pub payment_id: Uuid,
    pub status: String,
    pub confirmations: u64,
    pub required_confirmations: u64,
    pub message: String,
}

pub async fn verify_crypto_transaction(
    State(state): State<AppState>,
    Json(request): Json<VerifyCryptoTransactionRequest>,
) -> AppResult<Json<VerifyCryptoTransactionResponse>> {
    // Validate request
    request.validate().map_err(|e| {
        AppError::Validation(format!("Invalid request: {}", e))
    })?;

    // Validate transaction hash format (hex for EVM, base58 for Solana)
    let tx_hash = request.tx_hash.trim();
    if tx_hash.starts_with("0x") {
        // EVM transaction hash - should be 66 chars (0x + 64 hex)
        if tx_hash.len() != 66 || !tx_hash[2..].chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(AppError::Validation("Invalid EVM transaction hash format".to_string()));
        }
    }

    // Get payment to check it exists and is in valid state
    let existing_payment = PaymentRepository::find_by_id(&state.db, request.payment_id).await?;

    if existing_payment.status == PaymentStatus::Completed {
        return Err(AppError::Payment("Payment already completed".to_string()));
    }

    if existing_payment.status == PaymentStatus::Failed || existing_payment.status == PaymentStatus::Cancelled {
        return Err(AppError::Payment(format!(
            "Cannot verify payment in state: {:?}",
            existing_payment.status
        )));
    }

    let payment = state
        .payment_processor
        .verify_crypto_payment(&state.db, request.payment_id, tx_hash)
        .await?;

    // Broadcast update
    if let Some(ref broadcaster) = state.ws_broadcaster {
        let _ = broadcaster.broadcast_payment_update(&payment).await;
    }

    let (confirmations, required) = match payment.status {
        PaymentStatus::Completed => (12, 12), // Placeholder - would fetch actual
        PaymentStatus::Processing => (3, 12),
        _ => (0, 12),
    };

    tracing::info!(
        payment_id = %payment.id,
        tx_hash = %tx_hash,
        status = ?payment.status,
        "Crypto transaction verified"
    );

    Ok(Json(VerifyCryptoTransactionResponse {
        success: true,
        payment_id: payment.id,
        status: format!("{:?}", payment.status).to_lowercase(),
        confirmations,
        required_confirmations: required,
        message: match payment.status {
            PaymentStatus::Completed => "Payment confirmed".to_string(),
            PaymentStatus::Processing => "Payment received, waiting for confirmations".to_string(),
            _ => "Transaction verification in progress".to_string(),
        },
    }))
}

#[derive(Debug, Deserialize, Validate)]
pub struct GetBalanceParams {
    #[validate(length(min = 20, max = 100, message = "Invalid address format"))]
    pub address: String,
    #[validate(length(min = 3, max = 20, message = "Invalid chain format"))]
    pub chain: String,
}

pub async fn get_balance(
    State(state): State<AppState>,
    axum::extract::Query(params): axum::extract::Query<GetBalanceParams>,
) -> AppResult<Json<BalanceResponse>> {
    // Validate params
    params.validate().map_err(|e| {
        AppError::Validation(format!("Invalid parameters: {}", e))
    })?;

    let chain_type: ChainType = params
        .chain
        .parse()
        .map_err(|e| AppError::Validation(format!("Invalid chain: {}", e)))?;

    // Validate address format for the specific chain
    if !WalletConnectVerifier::validate_address(&params.address, &chain_type) {
        return Err(AppError::InvalidAddress(format!(
            "Invalid {} address format",
            chain_type
        )));
    }

    let (balance, balance_wei) = match chain_type {
        ChainType::Ethereum | ChainType::Polygon | ChainType::Bsc | ChainType::Arbitrum => {
            let service = state
                .payment_processor
                .get_evm_service(&chain_type)
                .ok_or_else(|| AppError::Payment(format!("{} RPC not configured", chain_type)))?;

            let balance_wei = service.get_balance(&params.address).await?;
            let balance = crate::services::EthereumService::wei_to_eth(balance_wei);
            (balance, balance_wei.to_string())
        }
        ChainType::Solana => {
            let balance_lamports = state
                .payment_processor
                .solana()
                .get_balance(&params.address)?;
            let balance = crate::services::SolanaService::lamports_to_sol(balance_lamports);
            (balance, balance_lamports.to_string())
        }
        ChainType::Bitcoin => {
            return Err(AppError::Payment(
                "Bitcoin balance check requires Lightning node. Use /api/v1/crypto/lightning/balance instead.".to_string(),
            ))
        }
    };

    Ok(Json(BalanceResponse {
        address: params.address,
        chain: chain_type,
        balance,
        balance_wei,
        token_balance: None,
    }))
}

#[derive(Debug, Deserialize, Validate)]
pub struct WalletSignatureVerifyRequest {
    #[validate(length(min = 20, max = 100, message = "Invalid address format"))]
    pub address: String,
    #[validate(length(min = 1, max = 1000, message = "Message too long"))]
    pub message: String,
    #[validate(length(min = 64, max = 200, message = "Invalid signature format"))]
    pub signature: String,
    #[validate(length(min = 3, max = 20, message = "Invalid chain format"))]
    pub chain: String,
}

#[derive(Debug, Serialize)]
pub struct VerifySignatureResponse {
    pub valid: bool,
    pub address: String,
    pub chain: String,
    pub recovered_address: Option<String>,
}

pub async fn verify_wallet_signature(
    State(_state): State<AppState>,
    Json(request): Json<WalletSignatureVerifyRequest>,
) -> AppResult<Json<VerifySignatureResponse>> {
    // Validate request
    request.validate().map_err(|e| {
        AppError::Validation(format!("Invalid request: {}", e))
    })?;

    let chain_type: ChainType = request
        .chain
        .parse()
        .map_err(|e| AppError::Validation(format!("Invalid chain: {}", e)))?;

    // Validate address format
    if !WalletConnectVerifier::validate_address(&request.address, &chain_type) {
        return Err(AppError::InvalidAddress(format!(
            "Invalid {} address format",
            chain_type
        )));
    }

    let is_valid = WalletConnectVerifier::verify_signature(
        &request.address,
        &request.message,
        &request.signature,
        &chain_type,
    )?;

    tracing::debug!(
        address = %request.address,
        chain = %chain_type,
        valid = is_valid,
        "Wallet signature verification"
    );

    Ok(Json(VerifySignatureResponse {
        valid: is_valid,
        address: request.address.clone(),
        chain: chain_type.to_string(),
        recovered_address: if is_valid { Some(request.address) } else { None },
    }))
}

#[derive(Debug, Deserialize, Validate)]
pub struct TokenBalanceParams {
    #[validate(length(min = 20, max = 100, message = "Invalid wallet address format"))]
    pub wallet_address: String,
    #[validate(length(min = 20, max = 100, message = "Invalid token address format"))]
    pub token_address: String,
    #[validate(length(min = 3, max = 20, message = "Invalid chain format"))]
    pub chain: String,
}

pub async fn get_token_balance(
    State(state): State<AppState>,
    axum::extract::Query(params): axum::extract::Query<TokenBalanceParams>,
) -> AppResult<Json<BalanceResponse>> {
    // Validate params
    params.validate().map_err(|e| {
        AppError::Validation(format!("Invalid parameters: {}", e))
    })?;

    let chain_type: ChainType = params
        .chain
        .parse()
        .map_err(|e| AppError::Validation(format!("Invalid chain: {}", e)))?;

    // Validate wallet address
    if !WalletConnectVerifier::validate_address(&params.wallet_address, &chain_type) {
        return Err(AppError::InvalidAddress("Invalid wallet address format".to_string()));
    }

    // Validate token address for EVM chains
    if matches!(chain_type, ChainType::Ethereum | ChainType::Polygon | ChainType::Bsc | ChainType::Arbitrum) {
        if !WalletConnectVerifier::validate_address(&params.token_address, &chain_type) {
            return Err(AppError::InvalidAddress("Invalid token contract address format".to_string()));
        }
    }

    match chain_type {
        ChainType::Ethereum | ChainType::Polygon | ChainType::Bsc | ChainType::Arbitrum => {
            let service = state
                .payment_processor
                .get_evm_service(&chain_type)
                .ok_or_else(|| AppError::Payment(format!("{} RPC not configured", chain_type)))?;

            let token_balance = service
                .get_token_balance(&params.token_address, &params.wallet_address)
                .await?;

            Ok(Json(BalanceResponse {
                address: params.wallet_address,
                chain: chain_type,
                balance: "0".to_string(),
                balance_wei: "0".to_string(),
                token_balance: Some(token_balance.to_string()),
            }))
        }
        ChainType::Solana => {
            let balance = state
                .payment_processor
                .solana()
                .get_token_balance(&params.token_address)?;

            Ok(Json(BalanceResponse {
                address: params.wallet_address,
                chain: chain_type,
                balance: "0".to_string(),
                balance_wei: "0".to_string(),
                token_balance: Some(balance.to_string()),
            }))
        }
        _ => Err(AppError::Payment(
            "Token balance not supported for this chain".to_string(),
        )),
    }
}

pub async fn generate_address(
    State(_state): State<AppState>,
    axum::extract::Path(chain): axum::extract::Path<String>,
) -> AppResult<Json<serde_json::Value>> {
    let chain_type: ChainType = chain
        .parse()
        .map_err(|e| AppError::Validation(format!("Invalid chain: {}", e)))?;

    // Address generation requires HD wallet configuration
    // This would be implemented with proper key derivation in production
    Err(AppError::Payment(format!(
        "Dynamic address generation for {} requires HD wallet configuration. Contact administrator to set up merchant wallet.",
        chain_type
    )))
}
