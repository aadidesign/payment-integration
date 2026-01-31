use ethers::{
    prelude::*,
    providers::{Http, Provider, Ws},
    types::{Address, TransactionReceipt, H256, U256},
    utils::{format_ether, parse_ether},
};
use std::sync::Arc;

use crate::config::{ChainConfig, EthereumConfig};
use crate::error::{AppError, AppResult};
use crate::models::ChainType;

#[derive(Clone)]
pub struct EthereumService {
    provider: Arc<Provider<Http>>,
    chain_id: u64,
    chain_type: ChainType,
}

impl EthereumService {
    pub async fn new(config: &EthereumConfig) -> AppResult<Self> {
        let provider = Provider::<Http>::try_from(&config.rpc_url)
            .map_err(|e| AppError::Ethereum(format!("Failed to create provider: {}", e)))?;

        Ok(Self {
            provider: Arc::new(provider),
            chain_id: config.chain_id,
            chain_type: ChainType::Ethereum,
        })
    }

    pub async fn new_for_chain(config: &ChainConfig, chain_type: ChainType) -> AppResult<Self> {
        let provider = Provider::<Http>::try_from(&config.rpc_url)
            .map_err(|e| AppError::Ethereum(format!("Failed to create provider: {}", e)))?;

        Ok(Self {
            provider: Arc::new(provider),
            chain_id: config.chain_id,
            chain_type,
        })
    }

    pub fn chain_type(&self) -> &ChainType {
        &self.chain_type
    }

    pub fn chain_id(&self) -> u64 {
        self.chain_id
    }

    /// Get the current block number
    pub async fn get_block_number(&self) -> AppResult<u64> {
        self.provider
            .get_block_number()
            .await
            .map(|n| n.as_u64())
            .map_err(|e| AppError::Ethereum(format!("Failed to get block number: {}", e)))
    }

    /// Get balance of an address in Wei
    pub async fn get_balance(&self, address: &str) -> AppResult<U256> {
        let address: Address = address
            .parse()
            .map_err(|e| AppError::InvalidAddress(format!("Invalid Ethereum address: {}", e)))?;

        self.provider
            .get_balance(address, None)
            .await
            .map_err(|e| AppError::Ethereum(format!("Failed to get balance: {}", e)))
    }

    /// Get balance formatted as ETH string
    pub async fn get_balance_eth(&self, address: &str) -> AppResult<String> {
        let balance = self.get_balance(address).await?;
        Ok(format_ether(balance))
    }

    /// Get transaction by hash
    pub async fn get_transaction(&self, tx_hash: &str) -> AppResult<Option<Transaction>> {
        let hash: H256 = tx_hash
            .parse()
            .map_err(|e| AppError::Ethereum(format!("Invalid transaction hash: {}", e)))?;

        self.provider
            .get_transaction(hash)
            .await
            .map_err(|e| AppError::Ethereum(format!("Failed to get transaction: {}", e)))
    }

    /// Get transaction receipt
    pub async fn get_transaction_receipt(
        &self,
        tx_hash: &str,
    ) -> AppResult<Option<TransactionReceipt>> {
        let hash: H256 = tx_hash
            .parse()
            .map_err(|e| AppError::Ethereum(format!("Invalid transaction hash: {}", e)))?;

        self.provider
            .get_transaction_receipt(hash)
            .await
            .map_err(|e| AppError::Ethereum(format!("Failed to get receipt: {}", e)))
    }

    /// Get number of confirmations for a transaction
    pub async fn get_confirmations(&self, tx_hash: &str) -> AppResult<u64> {
        let receipt = self.get_transaction_receipt(tx_hash).await?;

        match receipt {
            Some(r) => {
                if let Some(block_number) = r.block_number {
                    let current_block = self.get_block_number().await?;
                    Ok(current_block.saturating_sub(block_number.as_u64()))
                } else {
                    Ok(0)
                }
            }
            None => Ok(0),
        }
    }

    /// Verify a payment transaction
    pub async fn verify_payment(
        &self,
        tx_hash: &str,
        expected_to: &str,
        expected_amount_wei: U256,
    ) -> AppResult<PaymentVerification> {
        let tx = self
            .get_transaction(tx_hash)
            .await?
            .ok_or_else(|| AppError::Ethereum("Transaction not found".to_string()))?;

        let receipt = self.get_transaction_receipt(tx_hash).await?;

        let to_address: Address = expected_to
            .parse()
            .map_err(|e| AppError::InvalidAddress(format!("Invalid address: {}", e)))?;

        // Verify transaction details
        let to_matches = tx.to == Some(to_address);
        let amount_matches = tx.value >= expected_amount_wei;
        let is_successful = receipt
            .as_ref()
            .map(|r| r.status == Some(1.into()))
            .unwrap_or(false);

        let confirmations = if let Some(ref r) = receipt {
            if let Some(block_number) = r.block_number {
                let current_block = self.get_block_number().await?;
                current_block.saturating_sub(block_number.as_u64())
            } else {
                0
            }
        } else {
            0
        };

        Ok(PaymentVerification {
            is_valid: to_matches && amount_matches && is_successful,
            to_matches,
            amount_matches,
            is_successful,
            confirmations,
            from_address: tx.from.to_string(),
            to_address: tx.to.map(|a| a.to_string()),
            actual_amount: tx.value,
            block_number: receipt.and_then(|r| r.block_number.map(|b| b.as_u64())),
        })
    }

    /// Validate an Ethereum address
    pub fn validate_address(address: &str) -> bool {
        address.parse::<Address>().is_ok()
    }

    /// Convert ETH to Wei
    pub fn eth_to_wei(eth: &str) -> AppResult<U256> {
        parse_ether(eth).map_err(|e| AppError::Ethereum(format!("Invalid ETH amount: {}", e)))
    }

    /// Convert Wei to ETH
    pub fn wei_to_eth(wei: U256) -> String {
        format_ether(wei)
    }

    /// Verify a signature (EIP-191 personal sign)
    pub fn verify_signature(
        address: &str,
        message: &str,
        signature: &str,
    ) -> AppResult<bool> {
        let address: Address = address
            .parse()
            .map_err(|e| AppError::InvalidAddress(format!("Invalid address: {}", e)))?;

        let signature_bytes = hex::decode(signature.trim_start_matches("0x"))
            .map_err(|e| AppError::InvalidSignature(format!("Invalid signature format: {}", e)))?;

        if signature_bytes.len() != 65 {
            return Err(AppError::InvalidSignature(
                "Signature must be 65 bytes".to_string(),
            ));
        }

        let signature = Signature::try_from(signature_bytes.as_slice())
            .map_err(|e| AppError::InvalidSignature(format!("Invalid signature: {}", e)))?;

        let recovered = signature
            .recover(message)
            .map_err(|e| AppError::InvalidSignature(format!("Failed to recover address: {}", e)))?;

        Ok(recovered == address)
    }

    /// Get required confirmations based on chain and amount
    pub fn get_required_confirmations(&self, amount_wei: U256) -> i32 {
        // Higher amounts require more confirmations
        let eth_amount = Self::wei_to_eth(amount_wei)
            .parse::<f64>()
            .unwrap_or(0.0);

        match self.chain_type {
            ChainType::Ethereum => {
                if eth_amount > 10.0 {
                    12
                } else if eth_amount > 1.0 {
                    6
                } else {
                    3
                }
            }
            ChainType::Polygon | ChainType::Bsc => {
                // Faster block times, need more confirmations
                if eth_amount > 10.0 {
                    50
                } else if eth_amount > 1.0 {
                    25
                } else {
                    10
                }
            }
            ChainType::Arbitrum => {
                // L2, faster finality
                if eth_amount > 10.0 {
                    20
                } else {
                    5
                }
            }
            _ => 3,
        }
    }
}

#[derive(Debug, Clone)]
pub struct PaymentVerification {
    pub is_valid: bool,
    pub to_matches: bool,
    pub amount_matches: bool,
    pub is_successful: bool,
    pub confirmations: u64,
    pub from_address: String,
    pub to_address: Option<String>,
    pub actual_amount: U256,
    pub block_number: Option<u64>,
}

/// ERC20 Token interactions
impl EthereumService {
    /// Get ERC20 token balance
    pub async fn get_token_balance(
        &self,
        token_address: &str,
        wallet_address: &str,
    ) -> AppResult<U256> {
        // ERC20 balanceOf(address) function signature
        let balance_of_sig = "70a08231";

        let token: Address = token_address
            .parse()
            .map_err(|e| AppError::InvalidAddress(format!("Invalid token address: {}", e)))?;

        let wallet: Address = wallet_address
            .parse()
            .map_err(|e| AppError::InvalidAddress(format!("Invalid wallet address: {}", e)))?;

        // Encode the call data
        let data = format!(
            "0x{}000000000000000000000000{}",
            balance_of_sig,
            hex::encode(wallet.as_bytes())
        );

        let call = TransactionRequest::new()
            .to(token)
            .data(hex::decode(data.trim_start_matches("0x")).map_err(|e| {
                AppError::Ethereum(format!("Failed to encode call data: {}", e))
            })?);

        let result = self
            .provider
            .call(&call.into(), None)
            .await
            .map_err(|e| AppError::Ethereum(format!("Failed to call token contract: {}", e)))?;

        if result.len() >= 32 {
            Ok(U256::from_big_endian(&result[..32]))
        } else {
            Err(AppError::Ethereum("Invalid response from token contract".to_string()))
        }
    }
}
