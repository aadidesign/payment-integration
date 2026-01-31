mod client;
mod orders;
mod payments;
mod webhooks;

pub use client::RazorpayClient;
pub use orders::*;
pub use payments::*;
pub use webhooks::*;

use std::sync::Arc;

use crate::config::RazorpayConfig;

pub struct RazorpayService {
    client: RazorpayClient,
}

impl RazorpayService {
    pub fn new(config: &RazorpayConfig) -> Self {
        Self {
            client: RazorpayClient::new(config),
        }
    }

    pub fn client(&self) -> &RazorpayClient {
        &self.client
    }
}

pub type SharedRazorpayService = Arc<RazorpayService>;
