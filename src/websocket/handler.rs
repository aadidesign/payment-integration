use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::Response,
};
use futures::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::broadcast;
use uuid::Uuid;

use crate::AppState;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum WsMessage {
    #[serde(rename = "subscribe")]
    Subscribe { payment_ids: Vec<Uuid> },
    #[serde(rename = "unsubscribe")]
    Unsubscribe { payment_ids: Vec<Uuid> },
    #[serde(rename = "ping")]
    Ping,
    #[serde(rename = "pong")]
    Pong,
    #[serde(rename = "payment_update")]
    PaymentUpdate(PaymentUpdateData),
    #[serde(rename = "error")]
    Error { message: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentUpdateData {
    pub payment_id: Uuid,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tx_hash: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confirmations: Option<u64>,
    pub timestamp: i64,
}

pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> Response {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: AppState) {
    let (mut sender, mut receiver) = socket.split();

    // Subscribe to payment updates
    let mut rx = if let Some(ref broadcaster) = state.ws_broadcaster {
        broadcaster.subscribe()
    } else {
        // Create a dummy channel if broadcaster is not configured
        let (tx, rx) = broadcast::channel(16);
        rx
    };

    // Track subscribed payment IDs
    let subscribed_payments: Arc<parking_lot::RwLock<std::collections::HashSet<Uuid>>> =
        Arc::new(parking_lot::RwLock::new(std::collections::HashSet::new()));

    let subscribed_clone = subscribed_payments.clone();

    // Task to receive messages from the WebSocket client
    let recv_task = tokio::spawn(async move {
        while let Some(result) = receiver.next().await {
            match result {
                Ok(Message::Text(text)) => {
                    match serde_json::from_str::<WsMessage>(&text) {
                        Ok(WsMessage::Subscribe { payment_ids }) => {
                            let mut subs = subscribed_clone.write();
                            for id in payment_ids {
                                subs.insert(id);
                            }
                            tracing::debug!("Client subscribed to {} payments", subs.len());
                        }
                        Ok(WsMessage::Unsubscribe { payment_ids }) => {
                            let mut subs = subscribed_clone.write();
                            for id in payment_ids {
                                subs.remove(&id);
                            }
                        }
                        Ok(WsMessage::Ping) => {
                            // Will send pong in the send task
                        }
                        Ok(_) => {}
                        Err(e) => {
                            tracing::warn!("Failed to parse WebSocket message: {}", e);
                        }
                    }
                }
                Ok(Message::Close(_)) => {
                    tracing::debug!("WebSocket client disconnected");
                    break;
                }
                Ok(Message::Ping(_)) => {}
                Ok(Message::Pong(_)) => {}
                Ok(Message::Binary(_)) => {}
                Err(e) => {
                    tracing::error!("WebSocket error: {}", e);
                    break;
                }
            }
        }
    });

    // Task to send messages to the WebSocket client
    let send_task = tokio::spawn(async move {
        let mut heartbeat_interval = tokio::time::interval(std::time::Duration::from_secs(30));

        loop {
            tokio::select! {
                // Handle broadcast messages
                result = rx.recv() => {
                    match result {
                        Ok(update) => {
                            // Check if client is subscribed to this payment
                            let is_subscribed = {
                                let subs = subscribed_payments.read();
                                subs.is_empty() || subs.contains(&update.payment_id)
                            };

                            if is_subscribed {
                                let msg = WsMessage::PaymentUpdate(update);
                                if let Ok(json) = serde_json::to_string(&msg) {
                                    if sender.send(Message::Text(json)).await.is_err() {
                                        break;
                                    }
                                }
                            }
                        }
                        Err(broadcast::error::RecvError::Lagged(_)) => {
                            tracing::warn!("WebSocket client lagged behind broadcasts");
                        }
                        Err(broadcast::error::RecvError::Closed) => {
                            break;
                        }
                    }
                }
                // Send heartbeat
                _ = heartbeat_interval.tick() => {
                    let pong = WsMessage::Pong;
                    if let Ok(json) = serde_json::to_string(&pong) {
                        if sender.send(Message::Text(json)).await.is_err() {
                            break;
                        }
                    }
                }
            }
        }
    });

    // Wait for either task to complete
    tokio::select! {
        _ = recv_task => {}
        _ = send_task => {}
    }

    tracing::debug!("WebSocket connection closed");
}
