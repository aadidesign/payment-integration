use std::sync::Arc;
use sqlx::PgPool;
use uuid::Uuid;

use crate::config::Config;
use crate::db::repositories::{AddressRepository, PaymentRepository, TransactionRepository};
use crate::error::{AppError, AppResult};
use crate::models::{
    ChainType, CreatePaymentRequest, CurrencyType, Payment, PaymentMethod, PaymentStatus,
    TransactionType,
};
use crate::services::{
    EthereumService, LightningService, RazorpayService, SolanaService,
};
use crate::services::razorpay::CreateOrderRequest;

pub struct PaymentProcessor {
    razorpay: Arc<RazorpayService>,
    ethereum: Arc<EthereumService>,
    polygon: Option<Arc<EthereumService>>,
    bsc: Option<Arc<EthereumService>>,
    arbitrum: Option<Arc<EthereumService>>,
    solana: Arc<SolanaService>,
    lightning: Arc<LightningService>,
}

impl PaymentProcessor {
    pub async fn new(config: &Config) -> AppResult<Self> {
        let razorpay = Arc::new(RazorpayService::new(&config.razorpay));
        let ethereum = Arc::new(EthereumService::new(&config.ethereum).await?);
        let solana = Arc::new(SolanaService::new(&config.solana));
        let lightning = Arc::new(LightningService::new(&config.lightning));

        // Initialize other EVM chains if configured
        let polygon = if !config.polygon.rpc_url.is_empty() {
            Some(Arc::new(
                EthereumService::new_for_chain(&config.polygon, ChainType::Polygon).await?,
            ))
        } else {
            None
        };

        let bsc = if !config.bsc.rpc_url.is_empty() {
            Some(Arc::new(
                EthereumService::new_for_chain(&config.bsc, ChainType::Bsc).await?,
            ))
        } else {
            None
        };

        let arbitrum = if !config.arbitrum.rpc_url.is_empty() {
            Some(Arc::new(
                EthereumService::new_for_chain(&config.arbitrum, ChainType::Arbitrum).await?,
            ))
        } else {
            None
        };

        Ok(Self {
            razorpay,
            ethereum,
            polygon,
            bsc,
            arbitrum,
            solana,
            lightning,
        })
    }

    pub fn razorpay(&self) -> &RazorpayService {
        &self.razorpay
    }

    pub fn ethereum(&self) -> &EthereumService {
        &self.ethereum
    }

    pub fn solana(&self) -> &SolanaService {
        &self.solana
    }

    pub fn lightning(&self) -> &LightningService {
        &self.lightning
    }

    pub fn get_evm_service(&self, chain: &ChainType) -> Option<&EthereumService> {
        match chain {
            ChainType::Ethereum => Some(&self.ethereum),
            ChainType::Polygon => self.polygon.as_deref(),
            ChainType::Bsc => self.bsc.as_deref(),
            ChainType::Arbitrum => self.arbitrum.as_deref(),
            _ => None,
        }
    }

    /// Create a payment based on the payment method
    pub async fn create_payment(
        &self,
        pool: &PgPool,
        request: &CreatePaymentRequest,
    ) -> AppResult<PaymentCreationResult> {
        // Create the payment record
        let payment = PaymentRepository::create(pool, request).await?;

        match &request.method {
            PaymentMethod::Card
            | PaymentMethod::Upi
            | PaymentMethod::NetBanking
            | PaymentMethod::Wallet
            | PaymentMethod::Emi => {
                self.create_razorpay_payment(pool, &payment, request).await
            }
            PaymentMethod::Ethereum
            | PaymentMethod::Polygon
            | PaymentMethod::Bsc
            | PaymentMethod::Arbitrum => {
                self.create_evm_payment(pool, &payment, request).await
            }
            PaymentMethod::Solana => {
                self.create_solana_payment(pool, &payment, request).await
            }
            PaymentMethod::Lightning => {
                self.create_lightning_payment(pool, &payment, request).await
            }
        }
    }

    async fn create_razorpay_payment(
        &self,
        pool: &PgPool,
        payment: &Payment,
        request: &CreatePaymentRequest,
    ) -> AppResult<PaymentCreationResult> {
        let currency = match request.currency {
            CurrencyType::INR => "INR",
            CurrencyType::USD => "USD",
            CurrencyType::EUR => "EUR",
            _ => return Err(AppError::Payment("Invalid currency for Razorpay".to_string())),
        };

        let order_request = CreateOrderRequest {
            amount: request.amount,
            currency: currency.to_string(),
            receipt: Some(payment.id.to_string()),
            notes: request.metadata.clone(),
            partial_payment: Some(false),
        };

        let order = self.razorpay.client().create_order(&order_request).await?;

        // Update payment with Razorpay order ID
        PaymentRepository::update_razorpay_details(
            pool,
            payment.id,
            &order.id,
            None,
            None,
        )
        .await?;

        Ok(PaymentCreationResult {
            payment_id: payment.id,
            status: PaymentStatus::Pending,
            razorpay_order_id: Some(order.id),
            razorpay_key_id: Some(self.razorpay.client().webhook_secret().to_string()),
            crypto_address: None,
            lightning_invoice: None,
            chain: None,
            expires_at: None,
        })
    }

    async fn create_evm_payment(
        &self,
        pool: &PgPool,
        payment: &Payment,
        request: &CreatePaymentRequest,
    ) -> AppResult<PaymentCreationResult> {
        let chain_type = match request.method {
            PaymentMethod::Ethereum => ChainType::Ethereum,
            PaymentMethod::Polygon => ChainType::Polygon,
            PaymentMethod::Bsc => ChainType::Bsc,
            PaymentMethod::Arbitrum => ChainType::Arbitrum,
            _ => return Err(AppError::Payment("Invalid EVM chain".to_string())),
        };

        // In production, you would generate a unique deposit address
        // For now, we'll use a placeholder approach
        // You could use HD wallet derivation to generate unique addresses
        let deposit_address = self.generate_deposit_address(&chain_type)?;

        // Create address record
        AddressRepository::create(
            pool,
            &deposit_address,
            chain_type.clone(),
            Some(payment.id),
            Some(request.amount),
            None,
            None,
        )
        .await?;

        // Update payment with crypto details
        PaymentRepository::update_crypto_details(
            pool,
            payment.id,
            None,
            None,
            Some(&deposit_address),
            Some(&chain_type.to_string()),
        )
        .await?;

        // Create transaction record
        let evm_service = self.get_evm_service(&chain_type)
            .ok_or_else(|| AppError::Payment(format!("{} not configured", chain_type)))?;

        let required_confirmations = evm_service.get_required_confirmations(
            ethers::types::U256::from(request.amount as u128),
        );

        TransactionRepository::create(
            pool,
            payment.id,
            TransactionType::Payment,
            request.amount,
            &request.currency.to_string(),
            Some(&chain_type.to_string()),
            required_confirmations,
        )
        .await?;

        Ok(PaymentCreationResult {
            payment_id: payment.id,
            status: PaymentStatus::Pending,
            razorpay_order_id: None,
            razorpay_key_id: None,
            crypto_address: Some(deposit_address),
            lightning_invoice: None,
            chain: Some(chain_type.to_string()),
            expires_at: Some(chrono::Utc::now() + chrono::Duration::hours(1)),
        })
    }

    async fn create_solana_payment(
        &self,
        pool: &PgPool,
        payment: &Payment,
        request: &CreatePaymentRequest,
    ) -> AppResult<PaymentCreationResult> {
        let deposit_address = self.generate_deposit_address(&ChainType::Solana)?;

        // Create address record
        AddressRepository::create(
            pool,
            &deposit_address,
            ChainType::Solana,
            Some(payment.id),
            Some(request.amount),
            None,
            None,
        )
        .await?;

        // Update payment
        PaymentRepository::update_crypto_details(
            pool,
            payment.id,
            None,
            None,
            Some(&deposit_address),
            Some("solana"),
        )
        .await?;

        // Create transaction record
        let required_confirmations = SolanaService::get_required_confirmations(request.amount as u64);

        TransactionRepository::create(
            pool,
            payment.id,
            TransactionType::Payment,
            request.amount,
            &request.currency.to_string(),
            Some("solana"),
            required_confirmations,
        )
        .await?;

        Ok(PaymentCreationResult {
            payment_id: payment.id,
            status: PaymentStatus::Pending,
            razorpay_order_id: None,
            razorpay_key_id: None,
            crypto_address: Some(deposit_address),
            lightning_invoice: None,
            chain: Some("solana".to_string()),
            expires_at: Some(chrono::Utc::now() + chrono::Duration::hours(1)),
        })
    }

    async fn create_lightning_payment(
        &self,
        pool: &PgPool,
        payment: &Payment,
        request: &CreatePaymentRequest,
    ) -> AppResult<PaymentCreationResult> {
        // Amount in satoshis
        let amount_sat = request.amount as u64;

        let description = request
            .description
            .clone()
            .unwrap_or_else(|| format!("Payment {}", payment.id));

        // Create invoice via Lightning node
        let invoice_result = self
            .lightning
            .create_invoice(amount_sat, &description, 3600) // 1 hour expiry
            .await;

        match invoice_result {
            Ok(invoice) => {
                // Update payment with Lightning details
                PaymentRepository::update_lightning_details(
                    pool,
                    payment.id,
                    &invoice.payment_request,
                    &invoice.payment_hash,
                )
                .await?;

                Ok(PaymentCreationResult {
                    payment_id: payment.id,
                    status: PaymentStatus::Pending,
                    razorpay_order_id: None,
                    razorpay_key_id: None,
                    crypto_address: None,
                    lightning_invoice: Some(invoice.payment_request),
                    chain: Some("lightning".to_string()),
                    expires_at: Some(chrono::Utc::now() + chrono::Duration::hours(1)),
                })
            }
            Err(e) => Err(AppError::Lightning(format!(
                "Failed to create Lightning invoice: {}",
                e
            ))),
        }
    }

    fn generate_deposit_address(&self, chain: &ChainType) -> AppResult<String> {
        // In production, implement HD wallet derivation for unique addresses
        // For demo purposes, return a placeholder
        match chain {
            ChainType::Ethereum | ChainType::Polygon | ChainType::Bsc | ChainType::Arbitrum => {
                // Would use ethers-rs to derive new address from HD wallet
                Err(AppError::Payment(
                    "Configure HD wallet for address generation".to_string(),
                ))
            }
            ChainType::Solana => {
                // Would use solana-sdk to derive new address
                Err(AppError::Payment(
                    "Configure HD wallet for address generation".to_string(),
                ))
            }
            ChainType::Bitcoin => Err(AppError::Payment(
                "Bitcoin address generation not implemented".to_string(),
            )),
        }
    }

    /// Verify a crypto payment
    pub async fn verify_crypto_payment(
        &self,
        pool: &PgPool,
        payment_id: Uuid,
        tx_hash: &str,
    ) -> AppResult<Payment> {
        let payment = PaymentRepository::find_by_id(pool, payment_id).await?;

        let chain_str = payment
            .crypto_chain
            .as_ref()
            .ok_or_else(|| AppError::Payment("Payment has no chain set".to_string()))?;

        let chain: ChainType = chain_str
            .parse()
            .map_err(|e| AppError::Payment(format!("Invalid chain: {}", e)))?;

        let to_address = payment
            .crypto_to_address
            .as_ref()
            .ok_or_else(|| AppError::Payment("Payment has no deposit address".to_string()))?;

        // Verify based on chain
        match chain {
            ChainType::Ethereum | ChainType::Polygon | ChainType::Bsc | ChainType::Arbitrum => {
                let service = self
                    .get_evm_service(&chain)
                    .ok_or_else(|| AppError::Payment(format!("{} not configured", chain)))?;

                let amount_wei = ethers::types::U256::from(payment.amount as u128);
                let verification = service
                    .verify_payment(tx_hash, to_address, amount_wei)
                    .await?;

                if verification.is_valid {
                    // Update payment status
                    PaymentRepository::update_crypto_details(
                        pool,
                        payment_id,
                        Some(tx_hash),
                        Some(&verification.from_address),
                        None,
                        None,
                    )
                    .await?;

                    let new_status = if verification.confirmations
                        >= service.get_required_confirmations(amount_wei) as u64
                    {
                        PaymentStatus::Completed
                    } else {
                        PaymentStatus::Processing
                    };

                    PaymentRepository::update_status(pool, payment_id, new_status).await
                } else {
                    Err(AppError::Payment("Payment verification failed".to_string()))
                }
            }
            ChainType::Solana => {
                let verification = self
                    .solana
                    .verify_payment(tx_hash, to_address, payment.amount as u64)?;

                if verification.is_valid {
                    PaymentRepository::update_crypto_details(
                        pool,
                        payment_id,
                        Some(tx_hash),
                        None,
                        None,
                        None,
                    )
                    .await?;

                    let required = SolanaService::get_required_confirmations(payment.amount as u64);
                    let new_status = if verification.confirmations >= required as u64 {
                        PaymentStatus::Completed
                    } else {
                        PaymentStatus::Processing
                    };

                    PaymentRepository::update_status(pool, payment_id, new_status).await
                } else {
                    Err(AppError::Payment("Payment verification failed".to_string()))
                }
            }
            _ => Err(AppError::Payment("Unsupported chain for verification".to_string())),
        }
    }
}

#[derive(Debug, Clone)]
pub struct PaymentCreationResult {
    pub payment_id: Uuid,
    pub status: PaymentStatus,
    pub razorpay_order_id: Option<String>,
    pub razorpay_key_id: Option<String>,
    pub crypto_address: Option<String>,
    pub lightning_invoice: Option<String>,
    pub chain: Option<String>,
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
}

pub type SharedPaymentProcessor = Arc<PaymentProcessor>;
