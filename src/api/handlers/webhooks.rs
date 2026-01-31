use axum::{
    body::Bytes,
    extract::State,
    http::{HeaderMap, StatusCode},
    Json,
};
use serde::Serialize;

use crate::db::repositories::{PaymentRepository, WebhookRepository};
use crate::error::{AppError, AppResult};
use crate::models::{
    PaymentStatus, RazorpayWebhookPayload, WebhookSource, WebhookStatus,
};
use crate::services::razorpay::RazorpayWebhookVerifier;
use crate::AppState;

#[derive(Debug, Serialize)]
pub struct WebhookResponse {
    pub success: bool,
    pub message: String,
}

pub async fn razorpay_webhook(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Json<WebhookResponse>, (StatusCode, Json<WebhookResponse>)> {
    // Extract signature from headers
    let signature = headers
        .get("X-Razorpay-Signature")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| {
            (
                StatusCode::BAD_REQUEST,
                Json(WebhookResponse {
                    success: false,
                    message: "Missing signature header".to_string(),
                }),
            )
        })?;

    // Parse the webhook payload
    let payload: serde_json::Value = serde_json::from_slice(&body).map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            Json(WebhookResponse {
                success: false,
                message: format!("Invalid JSON: {}", e),
            }),
        )
    })?;

    // Store the webhook event
    let event_type = payload
        .get("event")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");

    let webhook_event = WebhookRepository::create(
        &state.db,
        WebhookSource::Razorpay,
        event_type,
        payload.clone(),
        Some(serde_json::to_value(&headers_to_map(&headers)).unwrap_or_default()),
        Some(signature),
    )
    .await
    .map_err(|e| {
        tracing::error!("Failed to store webhook event: {}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(WebhookResponse {
                success: false,
                message: "Internal error".to_string(),
            }),
        )
    })?;

    // Verify signature
    let signature_valid = RazorpayWebhookVerifier::verify_webhook_signature(
        &body,
        signature,
        &state.config.razorpay.webhook_secret,
    )
    .is_ok();

    if !signature_valid {
        WebhookRepository::update_status(
            &state.db,
            webhook_event.id,
            WebhookStatus::Failed,
            false,
            None,
            Some("Invalid signature"),
        )
        .await
        .ok();

        return Err((
            StatusCode::UNAUTHORIZED,
            Json(WebhookResponse {
                success: false,
                message: "Invalid signature".to_string(),
            }),
        ));
    }

    // Process the webhook
    let result = process_razorpay_webhook(&state, &payload).await;

    match result {
        Ok(payment_id) => {
            WebhookRepository::update_status(
                &state.db,
                webhook_event.id,
                WebhookStatus::Processed,
                true,
                payment_id,
                None,
            )
            .await
            .ok();

            Ok(Json(WebhookResponse {
                success: true,
                message: "Webhook processed successfully".to_string(),
            }))
        }
        Err(e) => {
            WebhookRepository::update_status(
                &state.db,
                webhook_event.id,
                WebhookStatus::Failed,
                true,
                None,
                Some(&e.to_string()),
            )
            .await
            .ok();

            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(WebhookResponse {
                    success: false,
                    message: e.to_string(),
                }),
            ))
        }
    }
}

async fn process_razorpay_webhook(
    state: &AppState,
    payload: &serde_json::Value,
) -> AppResult<Option<uuid::Uuid>> {
    let webhook: RazorpayWebhookPayload = serde_json::from_value(payload.clone())
        .map_err(|e| AppError::Serialization(e))?;

    match webhook.event.as_str() {
        "payment.authorized" | "payment.captured" => {
            if let Some(ref payment_entity) = webhook.payload.payment {
                let razorpay_payment = &payment_entity.entity;

                if let Some(ref order_id) = razorpay_payment.order_id {
                    let payment =
                        PaymentRepository::find_by_razorpay_order_id(&state.db, order_id).await?;

                    if let Some(payment) = payment {
                        // Update with Razorpay payment ID
                        PaymentRepository::update_razorpay_details(
                            &state.db,
                            payment.id,
                            order_id,
                            Some(&razorpay_payment.id),
                            None,
                        )
                        .await?;

                        // Update status based on event
                        let new_status = if webhook.event == "payment.captured" {
                            PaymentStatus::Completed
                        } else {
                            PaymentStatus::Processing
                        };

                        let updated =
                            PaymentRepository::update_status(&state.db, payment.id, new_status)
                                .await?;

                        // Broadcast update
                        if let Some(ref broadcaster) = state.ws_broadcaster {
                            let _ = broadcaster.broadcast_payment_update(&updated).await;
                        }

                        return Ok(Some(payment.id));
                    }
                }
            }
        }
        "payment.failed" => {
            if let Some(ref payment_entity) = webhook.payload.payment {
                let razorpay_payment = &payment_entity.entity;

                if let Some(ref order_id) = razorpay_payment.order_id {
                    let payment =
                        PaymentRepository::find_by_razorpay_order_id(&state.db, order_id).await?;

                    if let Some(payment) = payment {
                        let updated = PaymentRepository::update_status(
                            &state.db,
                            payment.id,
                            PaymentStatus::Failed,
                        )
                        .await?;

                        if let Some(ref broadcaster) = state.ws_broadcaster {
                            let _ = broadcaster.broadcast_payment_update(&updated).await;
                        }

                        return Ok(Some(payment.id));
                    }
                }
            }
        }
        "refund.created" | "refund.processed" => {
            // Handle refund events
            tracing::info!("Received refund webhook: {}", webhook.event);
        }
        _ => {
            tracing::info!("Unhandled webhook event: {}", webhook.event);
        }
    }

    Ok(None)
}

fn headers_to_map(headers: &HeaderMap) -> std::collections::HashMap<String, String> {
    headers
        .iter()
        .filter_map(|(k, v)| {
            v.to_str()
                .ok()
                .map(|val| (k.as_str().to_string(), val.to_string()))
        })
        .collect()
}

// Blockchain webhook for monitoring incoming transactions
#[derive(Debug, serde::Deserialize)]
pub struct BlockchainWebhookPayload {
    pub chain: String,
    pub tx_hash: String,
    pub from_address: String,
    pub to_address: String,
    pub amount: String,
    pub block_number: u64,
    pub confirmations: u32,
}

pub async fn blockchain_webhook(
    State(state): State<AppState>,
    Json(payload): Json<BlockchainWebhookPayload>,
) -> AppResult<Json<WebhookResponse>> {
    // Store webhook event
    let event = WebhookRepository::create(
        &state.db,
        WebhookSource::Blockchain,
        "transaction.received",
        serde_json::to_value(&payload).unwrap_or_default(),
        None,
        None,
    )
    .await?;

    // Find payment by destination address
    let payment = PaymentRepository::find_pending_by_crypto_address(
        &state.db,
        &payload.to_address,
        &payload.chain,
    )
    .await?;

    if let Some(payment) = payment {
        // Verify the transaction
        let verified = state
            .payment_processor
            .verify_crypto_payment(&state.db, payment.id, &payload.tx_hash)
            .await?;

        WebhookRepository::update_status(
            &state.db,
            event.id,
            WebhookStatus::Processed,
            true,
            Some(payment.id),
            None,
        )
        .await?;

        // Broadcast update
        if let Some(ref broadcaster) = state.ws_broadcaster {
            let _ = broadcaster.broadcast_payment_update(&verified).await;
        }
    }

    Ok(Json(WebhookResponse {
        success: true,
        message: "Webhook received".to_string(),
    }))
}
