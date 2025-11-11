use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub solana: SolanaConfig,
    pub ethereum: EthereumConfig,
    pub relayer: RelayerConfig,
    pub database: DatabaseConfig,
    pub validators: Vec<ValidatorConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SolanaConfig {
    pub rpc_url: String,
    pub ws_url: String,
    pub bridge_program_id: String,
    pub commitment: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EthereumConfig {
    pub rpc_url: String,
    pub ws_url: String,
    pub chain_id: u64,
    pub bridge_contract: String,
    pub wrapped_sol_contract: String,
    pub validator_registry_contract: String,
    pub confirmations: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelayerConfig {
    pub poll_interval_ms: u64,
    pub max_retries: u32,
    pub retry_delay_ms: u64,
    pub gas_price_multiplier: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    pub url: String,
    pub max_connections: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidatorConfig {
    pub name: String,
    pub eth_address: String,
    pub sol_public_key: String,
    pub endpoint: Option<String>,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        dotenv::dotenv().ok();

        let config = Config {
            solana: SolanaConfig {
                rpc_url: std::env::var("SOLANA_RPC_URL")
                    .unwrap_or_else(|_| "https://api.devnet.solana.com".to_string()),
                ws_url: std::env::var("SOLANA_WS_URL")
                    .unwrap_or_else(|_| "wss://api.devnet.solana.com".to_string()),
                bridge_program_id: std::env::var("SOLANA_BRIDGE_PROGRAM_ID")
                    .expect("SOLANA_BRIDGE_PROGRAM_ID must be set"),
                commitment: std::env::var("SOLANA_COMMITMENT")
                    .unwrap_or_else(|_| "confirmed".to_string()),
            },
            ethereum: EthereumConfig {
                rpc_url: std::env::var("ETHEREUM_RPC_URL")
                    .unwrap_or_else(|_| "https://sepolia.infura.io/v3/YOUR_KEY".to_string()),
                ws_url: std::env::var("ETHEREUM_WS_URL")
                    .unwrap_or_else(|_| "wss://sepolia.infura.io/ws/v3/YOUR_KEY".to_string()),
                chain_id: std::env::var("ETHEREUM_CHAIN_ID")
                    .unwrap_or_else(|_| "11155111".to_string())
                    .parse()
                    .expect("Invalid chain ID"),
                bridge_contract: std::env::var("ETHEREUM_BRIDGE_CONTRACT")
                    .expect("ETHEREUM_BRIDGE_CONTRACT must be set"),
                wrapped_sol_contract: std::env::var("ETHEREUM_WRAPPED_SOL_CONTRACT")
                    .expect("ETHEREUM_WRAPPED_SOL_CONTRACT must be set"),
                validator_registry_contract: std::env::var("ETHEREUM_VALIDATOR_REGISTRY_CONTRACT")
                    .expect("ETHEREUM_VALIDATOR_REGISTRY_CONTRACT must be set"),
                confirmations: std::env::var("ETHEREUM_CONFIRMATIONS")
                    .unwrap_or_else(|_| "12".to_string())
                    .parse()
                    .unwrap_or(12),
            },
            relayer: RelayerConfig {
                poll_interval_ms: std::env::var("POLL_INTERVAL_MS")
                    .unwrap_or_else(|_| "5000".to_string())
                    .parse()
                    .unwrap_or(5000),
                max_retries: std::env::var("MAX_RETRIES")
                    .unwrap_or_else(|_| "3".to_string())
                    .parse()
                    .unwrap_or(3),
                retry_delay_ms: std::env::var("RETRY_DELAY_MS")
                    .unwrap_or_else(|_| "2000".to_string())
                    .parse()
                    .unwrap_or(2000),
                gas_price_multiplier: std::env::var("GAS_PRICE_MULTIPLIER")
                    .unwrap_or_else(|_| "1.2".to_string())
                    .parse()
                    .unwrap_or(1.2),
            },
            database: DatabaseConfig {
                url: std::env::var("DATABASE_URL")
                    .unwrap_or_else(|_| "sqlite://relayer.db".to_string()),
                max_connections: std::env::var("DB_MAX_CONNECTIONS")
                    .unwrap_or_else(|_| "10".to_string())
                    .parse()
                    .unwrap_or(10),
            },
            validators: vec![
                // Default validators (should be configured via env)
                ValidatorConfig {
                    name: "Validator1".to_string(),
                    eth_address: std::env::var("VALIDATOR1_ETH_ADDRESS").unwrap_or_default(),
                    sol_public_key: std::env::var("VALIDATOR1_SOL_PUBKEY").unwrap_or_default(),
                    endpoint: std::env::var("VALIDATOR1_ENDPOINT").ok(),
                },
                ValidatorConfig {
                    name: "Validator2".to_string(),
                    eth_address: std::env::var("VALIDATOR2_ETH_ADDRESS").unwrap_or_default(),
                    sol_public_key: std::env::var("VALIDATOR2_SOL_PUBKEY").unwrap_or_default(),
                    endpoint: std::env::var("VALIDATOR2_ENDPOINT").ok(),
                },
                ValidatorConfig {
                    name: "Validator3".to_string(),
                    eth_address: std::env::var("VALIDATOR3_ETH_ADDRESS").unwrap_or_default(),
                    sol_public_key: std::env::var("VALIDATOR3_SOL_PUBKEY").unwrap_or_default(),
                    endpoint: std::env::var("VALIDATOR3_ENDPOINT").ok(),
                },
            ],
        };

        Ok(config)
    }

    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: Config = toml::from_str(&content)?;
        Ok(config)
    }
}
