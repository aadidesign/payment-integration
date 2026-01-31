pub mod api;
pub mod config;
pub mod crypto_utils;
pub mod db;
pub mod error;
pub mod models;
pub mod services;
pub mod websocket;

use std::sync::Arc;

use sqlx::PgPool;

use config::Config;
use services::PaymentProcessor;
use websocket::PaymentBroadcaster;

/// Shared application state
#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Config>,
    pub db: Arc<PgPool>,
    pub payment_processor: Arc<PaymentProcessor>,
    pub ws_broadcaster: Option<Arc<PaymentBroadcaster>>,
}

impl AppState {
    pub fn new(
        config: Config,
        db: PgPool,
        payment_processor: PaymentProcessor,
        ws_broadcaster: Option<PaymentBroadcaster>,
    ) -> Self {
        Self {
            config: Arc::new(config),
            db: Arc::new(db),
            payment_processor: Arc::new(payment_processor),
            ws_broadcaster: ws_broadcaster.map(Arc::new),
        }
    }
}
