use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

use crate::db::repositories::PaymentRepository;
use crate::error::{AppError, AppResult};
use crate::models::{
    CreatePaymentRequest, CurrencyType, PaymentMethod, PaymentResponse, PaymentStatus,
};
use crate::services::razorpay::RazorpayWebhookVerifier;
use crate::AppState;

#[derive(Debug, Deserialize, Validate)]
pub struct CreateRazorpayOrderRequest {
    #[validate(range(min = 100, message = "Amount must be at least 100 (1 INR in paise)"))]
    pub amount: i64,
    #[validate(length(min = 3, max = 3, message = "Currency must be 3 characters"))]
    pub currency: String,
    #[serde(default)]
    #[validate(length(max = 255, message = "Description too long"))]
    pub description: Option<String>,
    #[serde(default)]
    #[validate(email(message = "Invalid email format"))]
    pub customer_email: Option<String>,
    #[serde(default)]
    #[validate(length(min = 10, max = 15, message = "Invalid phone number"))]
    pub customer_phone: Option<String>,
    #[serde(default)]
    pub metadata: Option<serde_json::Value>,
    #[serde(default)]
    pub method: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct CreateOrderResponse {
    pub success: bool,
    pub payment_id: Uuid,
    pub razorpay_order_id: String,
    pub razorpay_key_id: String,
    pub amount: i64,
    pub currency: String,
}

pub async fn create_order(
    State(state): State<AppState>,
    Json(request): Json<CreateRazorpayOrderRequest>,
) -> AppResult<Json<CreateOrderResponse>> {
    // Validate request
    request.validate().map_err(|e| {
        AppError::Validation(format!("Invalid request: {}", e))
    })?;

    // Validate amount is positive
    if request.amount <= 0 {
        return Err(AppError::Validation("Amount must be positive".to_string()));
    }

    // Validate currency
    let currency = match request.currency.to_uppercase().as_str() {
        "INR" => CurrencyType::INR,
        "USD" => CurrencyType::USD,
        "EUR" => CurrencyType::EUR,
        _ => return Err(AppError::Validation(
            "Unsupported currency. Supported: INR, USD, EUR".to_string()
        )),
    };

    // Determine payment method
    let method = match request.method.as_deref() {
        Some("card") => PaymentMethod::Card,
        Some("upi") => PaymentMethod::Upi,
        Some("netbanking") | Some("net_banking") => PaymentMethod::NetBanking,
        Some("wallet") => PaymentMethod::Wallet,
        Some("emi") => PaymentMethod::Emi,
        Some(m) => return Err(AppError::Validation(
            format!("Invalid payment method: {}. Supported: card, upi, netbanking, wallet, emi", m)
        )),
        None => PaymentMethod::Card,
    };

    let payment_request = CreatePaymentRequest {
        amount: request.amount,
        currency,
        method,
        description: request.description,
        customer_email: request.customer_email,
        customer_phone: request.customer_phone,
        metadata: request.metadata,
        callback_url: None,
    };

    let result = state
        .payment_processor
        .create_payment(&state.db, &payment_request)
        .await?;

    tracing::info!(
        payment_id = %result.payment_id,
        amount = request.amount,
        "Razorpay order created successfully"
    );

    Ok(Json(CreateOrderResponse {
        success: true,
        payment_id: result.payment_id,
        razorpay_order_id: result
            .razorpay_order_id
            .ok_or_else(|| AppError::Internal("Failed to create Razorpay order".to_string()))?,
        razorpay_key_id: state.config.razorpay.key_id.clone(),
        amount: request.amount,
        currency: request.currency,
    }))
}

#[derive(Debug, Deserialize, Validate)]
pub struct VerifyPaymentRequest {
    #[validate(length(min = 10, max = 50, message = "Invalid order ID format"))]
    pub razorpay_order_id: String,
    #[validate(length(min = 10, max = 50, message = "Invalid payment ID format"))]
    pub razorpay_payment_id: String,
    #[validate(length(min = 64, max = 128, message = "Invalid signature format"))]
    pub razorpay_signature: String,
}

#[derive(Debug, Serialize)]
pub struct VerifyPaymentResponse {
    pub success: bool,
    pub payment_id: Uuid,
    pub status: PaymentStatus,
    pub message: String,
}

pub async fn verify_payment(
    State(state): State<AppState>,
    Json(request): Json<VerifyPaymentRequest>,
) -> AppResult<Json<VerifyPaymentResponse>> {
    // Validate request
    request.validate().map_err(|e| {
        AppError::Validation(format!("Invalid request: {}", e))
    })?;

    // Verify HMAC signature - critical security check
    RazorpayWebhookVerifier::verify_payment_signature(
        &request.razorpay_order_id,
        &request.razorpay_payment_id,
        &request.razorpay_signature,
        &state.config.razorpay.key_secret,
    )?;

    // Find payment by Razorpay order ID
    let payment = PaymentRepository::find_by_razorpay_order_id(
        &state.db,
        &request.razorpay_order_id,
    )
    .await?
    .ok_or_else(|| AppError::NotFound("Payment not found".to_string()))?;

    // Check if payment is in valid state for verification
    if payment.status != PaymentStatus::Pending && payment.status != PaymentStatus::Processing {
        return Err(AppError::Payment(format!(
            "Payment cannot be verified in current state: {:?}",
            payment.status
        )));
    }

    // Update payment with Razorpay details
    let updated_payment = PaymentRepository::update_razorpay_details(
        &state.db,
        payment.id,
        &request.razorpay_order_id,
        Some(&request.razorpay_payment_id),
        Some(&request.razorpay_signature),
    )
    .await?;

    // Update payment status to completed
    let final_payment = PaymentRepository::update_status(
        &state.db,
        updated_payment.id,
        PaymentStatus::Completed,
    )
    .await?;

    // Broadcast payment update via WebSocket
    if let Some(ref broadcaster) = state.ws_broadcaster {
        let _ = broadcaster.broadcast_payment_update(&final_payment).await;
    }

    tracing::info!(
        payment_id = %final_payment.id,
        razorpay_payment_id = %request.razorpay_payment_id,
        "Payment verified successfully"
    );

    Ok(Json(VerifyPaymentResponse {
        success: true,
        payment_id: final_payment.id,
        status: final_payment.status,
        message: "Payment verified successfully".to_string(),
    }))
}

pub async fn get_payment(
    State(state): State<AppState>,
    axum::extract::Path(payment_id): axum::extract::Path<Uuid>,
) -> AppResult<Json<PaymentResponse>> {
    let payment = PaymentRepository::find_by_id(&state.db, payment_id).await?;
    Ok(Json(payment.into()))
}

#[derive(Debug, Deserialize, Validate)]
pub struct RefundRequest {
    pub payment_id: Uuid,
    #[serde(default)]
    #[validate(range(min = 1, message = "Refund amount must be positive"))]
    pub amount: Option<i64>,
    #[serde(default)]
    pub notes: Option<serde_json::Value>,
    #[serde(default)]
    #[validate(length(max = 255, message = "Reason too long"))]
    pub reason: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct RefundResponse {
    pub success: bool,
    pub refund_id: String,
    pub payment_id: Uuid,
    pub amount: i64,
    pub status: String,
}

pub async fn process_refund(
    State(state): State<AppState>,
    Json(request): Json<RefundRequest>,
) -> AppResult<Json<RefundResponse>> {
    // Validate request
    request.validate().map_err(|e| {
        AppError::Validation(format!("Invalid request: {}", e))
    })?;

    let payment = PaymentRepository::find_by_id(&state.db, request.payment_id).await?;

    // Check if payment can be refunded
    if payment.status != PaymentStatus::Completed {
        return Err(AppError::Payment(format!(
            "Cannot refund payment in state: {:?}. Only completed payments can be refunded.",
            payment.status
        )));
    }

    // Validate refund amount doesn't exceed payment amount
    if let Some(refund_amount) = request.amount {
        if refund_amount > payment.amount {
            return Err(AppError::Validation(
                "Refund amount cannot exceed payment amount".to_string()
            ));
        }
    }

    let razorpay_payment_id = payment
        .razorpay_payment_id
        .ok_or_else(|| AppError::Payment("No Razorpay payment ID found".to_string()))?;

    let refund_request = crate::services::razorpay::RefundRequest {
        amount: request.amount,
        speed: Some("normal".to_string()),
        notes: request.notes,
        receipt: Some(format!("refund_{}", payment.id)),
    };

    let refund = state
        .payment_processor
        .razorpay()
        .client()
        .refund_payment(&razorpay_payment_id, &refund_request)
        .await?;

    // Update payment status
    PaymentRepository::update_status(&state.db, payment.id, PaymentStatus::Refunded).await?;

    tracing::info!(
        payment_id = %payment.id,
        refund_id = %refund.id,
        amount = refund.amount,
        "Refund processed successfully"
    );

    Ok(Json(RefundResponse {
        success: true,
        refund_id: refund.id,
        payment_id: payment.id,
        amount: refund.amount,
        status: refund.status,
    }))
}
