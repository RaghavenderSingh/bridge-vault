use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// CLI configuration structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CliConfig {
    pub solana_rpc: String,
    pub ethereum_rpc: String,
    pub bridge_program_id: String,
    pub eth_contract: String,
    pub keypair_path: String,
    pub relayer_url: String,
}

impl Default for CliConfig {
    fn default() -> Self {
        Self {
            solana_rpc: "https://api.devnet.solana.com".to_string(),
            ethereum_rpc: "https://sepolia.infura.io/v3/YOUR_KEY".to_string(),
            bridge_program_id: "BridgeProgramId111111111111111111111111111111".to_string(),
            eth_contract: "0x0000000000000000000000000000000000000000".to_string(),
            keypair_path: "~/.config/solana/id.json".to_string(),
            relayer_url: "http://localhost:8080".to_string(),
        }
    }
}

impl CliConfig {
    /// Load config from file, or return default if file doesn't exist
    pub fn load() -> Result<Self> {
        let config_path = Self::config_path()?;

        if config_path.exists() {
            let content = std::fs::read_to_string(&config_path)
                .with_context(|| format!("Failed to read config from {:?}", config_path))?;
            let config: CliConfig = toml::from_str(&content)
                .with_context(|| format!("Failed to parse config from {:?}", config_path))?;
            Ok(config)
        } else {
            Ok(CliConfig::default())
        }
    }

    /// Save config to file
    pub fn save(&self) -> Result<()> {
        let config_path = Self::config_path()?;
        let config_dir = config_path
            .parent()
            .context("Failed to get config directory")?;

        // Create directory if it doesn't exist
        std::fs::create_dir_all(config_dir)
            .with_context(|| format!("Failed to create config directory {:?}", config_dir))?;

        let content = toml::to_string_pretty(self)
            .context("Failed to serialize config to TOML")?;

        std::fs::write(&config_path, content)
            .with_context(|| format!("Failed to write config to {:?}", config_path))?;

        Ok(())
    }

    /// Get the config file path (~/.bridge/config.toml)
    pub fn config_path() -> Result<PathBuf> {
        let home_dir = dirs::home_dir().context("Failed to get home directory")?;
        Ok(home_dir.join(".bridge").join("config.toml"))
    }

    /// Check if config file exists
    pub fn exists() -> Result<bool> {
        Ok(Self::config_path()?.exists())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = CliConfig::default();
        assert!(!config.solana_rpc.is_empty());
        assert!(!config.ethereum_rpc.is_empty());
    }
}
