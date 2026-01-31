use std::sync::Arc;
use tokio::sync::broadcast;

use crate::models::Payment;
use crate::websocket::handler::PaymentUpdateData;

const BROADCAST_CHANNEL_SIZE: usize = 1000;

#[derive(Clone)]
pub struct PaymentBroadcaster {
    sender: broadcast::Sender<PaymentUpdateData>,
}

impl PaymentBroadcaster {
    pub fn new() -> Self {
        let (sender, _) = broadcast::channel(BROADCAST_CHANNEL_SIZE);
        Self { sender }
    }

    /// Subscribe to payment updates
    pub fn subscribe(&self) -> broadcast::Receiver<PaymentUpdateData> {
        self.sender.subscribe()
    }

    /// Broadcast a payment update to all connected clients
    pub async fn broadcast_payment_update(&self, payment: &Payment) -> Result<(), String> {
        let update = PaymentUpdateData {
            payment_id: payment.id,
            status: format!("{:?}", payment.status),
            tx_hash: payment.crypto_tx_hash.clone(),
            confirmations: None, // Would fetch from blockchain
            timestamp: chrono::Utc::now().timestamp(),
        };

        match self.sender.send(update) {
            Ok(count) => {
                tracing::debug!(
                    "Broadcast payment update for {} to {} clients",
                    payment.id,
                    count
                );
                Ok(())
            }
            Err(_) => {
                // No active receivers - this is fine
                Ok(())
            }
        }
    }

    /// Broadcast a custom update
    pub async fn broadcast(&self, update: PaymentUpdateData) -> Result<usize, String> {
        self.sender
            .send(update)
            .map_err(|e| format!("Failed to broadcast: {}", e))
    }

    /// Get the number of active subscribers
    pub fn subscriber_count(&self) -> usize {
        self.sender.receiver_count()
    }
}

impl Default for PaymentBroadcaster {
    fn default() -> Self {
        Self::new()
    }
}

pub type SharedPaymentBroadcaster = Arc<PaymentBroadcaster>;

pub fn create_broadcaster() -> SharedPaymentBroadcaster {
    Arc::new(PaymentBroadcaster::new())
}
