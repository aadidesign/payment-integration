use axum::{
    middleware,
    routing::{get, post},
    Router,
};
use tower_http::{
    compression::CompressionLayer,
    cors::{Any, CorsLayer},
    request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer},
    timeout::TimeoutLayer,
    trace::TraceLayer,
};
use std::time::Duration;

use crate::api::handlers;
use crate::api::middleware::{api_key_auth, request_logging};
use crate::AppState;

pub fn create_router(state: AppState) -> Router {
    // Public routes (no auth required)
    let public_routes = Router::new()
        .route("/health", get(handlers::health_check))
        .route("/api/v1/status", get(handlers::service_status));

    // Webhook routes (signature verification instead of API key)
    let webhook_routes = Router::new()
        .route("/webhooks/razorpay", post(handlers::razorpay_webhook))
        .route("/webhooks/blockchain", post(handlers::blockchain_webhook));

    // Razorpay payment routes
    let razorpay_routes = Router::new()
        .route("/orders", post(handlers::create_order))
        .route("/verify", post(handlers::verify_payment))
        .route("/payments/:payment_id", get(handlers::get_payment))
        .route("/refund", post(handlers::process_refund));

    // Crypto payment routes
    let crypto_routes = Router::new()
        .route("/payment", post(handlers::create_crypto_payment))
        .route("/payment/:payment_id", get(handlers::get_crypto_payment))
        .route("/verify", post(handlers::verify_crypto_transaction))
        .route("/address/:chain", get(handlers::generate_address))
        .route("/balance", get(handlers::get_balance))
        .route("/token-balance", get(handlers::get_token_balance))
        .route("/verify-signature", post(handlers::verify_wallet_signature));

    // Protected API routes
    let api_routes = Router::new()
        .nest("/razorpay", razorpay_routes)
        .nest("/crypto", crypto_routes)
        .layer(middleware::from_fn_with_state(state.clone(), api_key_auth));

    // WebSocket routes
    let ws_routes = Router::new()
        .route("/payments", get(crate::websocket::ws_handler));

    // Combine all routes
    Router::new()
        .merge(public_routes)
        .merge(webhook_routes)
        .nest("/api/v1", api_routes)
        .nest("/ws", ws_routes)
        .layer(CompressionLayer::new())
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        )
        .layer(TimeoutLayer::new(Duration::from_secs(30)))
        .layer(TraceLayer::new_for_http())
        .layer(PropagateRequestIdLayer::x_request_id())
        .layer(SetRequestIdLayer::x_request_id(MakeRequestUuid))
        .layer(middleware::from_fn(request_logging))
        .with_state(state)
}
