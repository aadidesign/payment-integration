use serde::Deserialize;
use std::sync::Arc;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub razorpay: RazorpayConfig,
    pub ethereum: EthereumConfig,
    pub polygon: ChainConfig,
    pub bsc: ChainConfig,
    pub arbitrum: ChainConfig,
    pub solana: SolanaConfig,
    pub lightning: LightningConfig,
    pub security: SecurityConfig,
    pub websocket: WebSocketConfig,
    pub rate_limit: RateLimitConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DatabaseConfig {
    pub url: String,
    pub max_connections: u32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RazorpayConfig {
    pub key_id: String,
    pub key_secret: String,
    pub webhook_secret: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct EthereumConfig {
    pub rpc_url: String,
    pub ws_url: Option<String>,
    pub chain_id: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ChainConfig {
    pub rpc_url: String,
    pub chain_id: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SolanaConfig {
    pub rpc_url: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LightningConfig {
    pub node_url: String,
    pub macaroon_path: Option<String>,
    pub tls_cert_path: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SecurityConfig {
    pub api_key_hash_secret: String,
    pub jwt_secret: String,
    pub encryption_key: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct WebSocketConfig {
    pub heartbeat_interval: u64,
    pub client_timeout: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RateLimitConfig {
    pub requests_per_second: u32,
    pub burst_size: u32,
}

impl Config {
    pub fn from_env() -> Result<Self, config::ConfigError> {
        let config = config::Config::builder()
            .set_default("server.host", "0.0.0.0")?
            .set_default("server.port", 8080)?
            .set_default("database.max_connections", 10)?
            .set_default("websocket.heartbeat_interval", 30)?
            .set_default("websocket.client_timeout", 60)?
            .set_default("rate_limit.requests_per_second", 100)?
            .set_default("rate_limit.burst_size", 200)?
            .add_source(config::Environment::default().separator("_").try_parsing(true))
            .build()?;

        // Manual construction due to environment variable naming
        Ok(Config {
            server: ServerConfig {
                host: config.get_string("host").unwrap_or_else(|_| "0.0.0.0".to_string()),
                port: config.get_int("port").unwrap_or(8080) as u16,
            },
            database: DatabaseConfig {
                url: config.get_string("database.url")?,
                max_connections: config.get_int("database.max_connections").unwrap_or(10) as u32,
            },
            razorpay: RazorpayConfig {
                key_id: config.get_string("razorpay.key.id")?,
                key_secret: config.get_string("razorpay.key.secret")?,
                webhook_secret: config.get_string("razorpay.webhook.secret")?,
            },
            ethereum: EthereumConfig {
                rpc_url: config.get_string("eth.rpc.url")?,
                ws_url: config.get_string("eth.ws.url").ok(),
                chain_id: config.get_int("eth.chain.id").unwrap_or(1) as u64,
            },
            polygon: ChainConfig {
                rpc_url: config.get_string("polygon.rpc.url").unwrap_or_default(),
                chain_id: config.get_int("polygon.chain.id").unwrap_or(137) as u64,
            },
            bsc: ChainConfig {
                rpc_url: config.get_string("bsc.rpc.url").unwrap_or_default(),
                chain_id: config.get_int("bsc.chain.id").unwrap_or(56) as u64,
            },
            arbitrum: ChainConfig {
                rpc_url: config.get_string("arbitrum.rpc.url").unwrap_or_default(),
                chain_id: config.get_int("arbitrum.chain.id").unwrap_or(42161) as u64,
            },
            solana: SolanaConfig {
                rpc_url: config.get_string("solana.rpc.url")?,
            },
            lightning: LightningConfig {
                node_url: config.get_string("lightning.node.url").unwrap_or_default(),
                macaroon_path: config.get_string("lightning.macaroon.path").ok(),
                tls_cert_path: config.get_string("lightning.tls.cert.path").ok(),
            },
            security: SecurityConfig {
                api_key_hash_secret: config.get_string("api.key.hash.secret")?,
                jwt_secret: config.get_string("jwt.secret")?,
                encryption_key: config.get_string("encryption.key")?,
            },
            websocket: WebSocketConfig {
                heartbeat_interval: config.get_int("ws.heartbeat.interval").unwrap_or(30) as u64,
                client_timeout: config.get_int("ws.client.timeout").unwrap_or(60) as u64,
            },
            rate_limit: RateLimitConfig {
                requests_per_second: config.get_int("rate.limit.requests.per.second").unwrap_or(100) as u32,
                burst_size: config.get_int("rate.limit.burst.size").unwrap_or(200) as u32,
            },
        })
    }
}

pub type SharedConfig = Arc<Config>;
