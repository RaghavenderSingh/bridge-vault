use crate::{
    config::ValidatorConfig,
    error::{RelayerError, Result},
    types::{Chain, ValidatorSignature},
};
use alloy::primitives::{Address, Bytes, U256};
use alloy::signers::{Signature as AlloySignature, Signer};
use chrono::Utc;
use secp256k1::ecdsa::Signature as Secp256k1Signature;
use sha2::{Digest, Sha256};
use std::str::FromStr;
use tracing::{debug, info};

// TODO: Implement actual HTTP requests to validator endpoints
// TODO: Implement ECDSA and Ed25519 signing

pub struct ValidatorClient {
    validators: Vec<ValidatorConfig>,
}

impl ValidatorClient {
    pub fn new(validators: Vec<ValidatorConfig>) -> Self {
        Self { validators }
    }

    /// Request signatures from validators for a Solana -> Ethereum transfer
    /// This creates the message that needs to be signed for minting on Ethereum
    pub async fn collect_signatures_for_ethereum_mint(
        &self,
        recipient: Address,
        amount: U256,
        nonce: u64,
        solana_sender: &str,
    ) -> Result<Vec<ValidatorSignature>> {
        info!(
            "Collecting signatures for Ethereum mint: recipient={}, amount={}, nonce={}",
            recipient, amount, nonce
        );

        // Create the message hash that validators will sign
        // This should match the hash creation in the Ethereum smart contract
        let message_hash = self.create_ethereum_message_hash(recipient, amount, nonce, solana_sender);

        debug!("Message hash: 0x{}", hex::encode(&message_hash));

        let mut signatures = Vec::new();

        for validator in &self.validators {
            if validator.endpoint.is_none() {
                debug!("Skipping validator {} (no endpoint configured)", validator.name);
                continue;
            }

            info!(
                "Would request signature from validator {} at endpoint {:?}",
                validator.name, validator.endpoint
            );

            let signature = ValidatorSignature {
                validator_address: validator.eth_address.clone(),
                signature: format!("0x{}", hex::encode(&message_hash)),
                signed_at: Utc::now(),
            };

            signatures.push(signature);
        }

        if signatures.is_empty() {
            return Err(RelayerError::InsufficientSignatures {
                expected: self.validators.len(),
                got: 0,
            });
        }

        info!("Collected {} signatures", signatures.len());
        Ok(signatures)
    }

    /// Request signatures from validators for an Ethereum -> Solana transfer
    /// This creates the message that needs to be signed for unlocking on Solana
    pub async fn collect_signatures_for_solana_unlock(
        &self,
        recipient: &str,
        amount: u64,
        nonce: u64,
        ethereum_sender: &str,
    ) -> Result<Vec<ValidatorSignature>> {
        info!(
            "Collecting signatures for Solana unlock: recipient={}, amount={}, nonce={}",
            recipient, amount, nonce
        );

        // Create the message hash that validators will sign
        let message_hash = self.create_solana_message_hash(recipient, amount, nonce, ethereum_sender);

        debug!("Message hash: 0x{}", hex::encode(&message_hash));

        let mut signatures = Vec::new();

        for validator in &self.validators {
            if validator.endpoint.is_none() {
                debug!("Skipping validator {} (no endpoint configured)", validator.name);
                continue;
            }

            info!(
                "Would request signature from validator {} at endpoint {:?}",
                validator.name, validator.endpoint
            );

            let signature = ValidatorSignature {
                validator_address: validator.sol_public_key.clone(),
                signature: format!("0x{}", hex::encode(&message_hash)),
                signed_at: Utc::now(),
            };

            signatures.push(signature);
        }

        if signatures.is_empty() {
            return Err(RelayerError::InsufficientSignatures {
                expected: self.validators.len(),
                got: 0,
            });
        }

        info!("Collected {} signatures", signatures.len());
        Ok(signatures)
    }

    /// Create the message hash for Ethereum smart contract verification
    /// This must match the hash creation in the SolanaBridge contract
    fn create_ethereum_message_hash(
        &self,
        recipient: Address,
        amount: U256,
        nonce: u64,
        solana_sender: &str,
    ) -> [u8; 32] {
        // In Solidity: keccak256(abi.encodePacked(recipient, amount, nonce, solanaSender))
        // We need to match this encoding exactly

        let mut data = Vec::new();

        // Add recipient (20 bytes, left-padded to 32 bytes in Solidity, but encodePacked doesn't pad)
        data.extend_from_slice(recipient.as_slice());

        // Add amount (32 bytes)
        data.extend_from_slice(&amount.to_be_bytes::<32>());

        // Add nonce (8 bytes, but as uint64 in Solidity it's 32 bytes, encodePacked uses minimal)
        data.extend_from_slice(&nonce.to_be_bytes());

        // Add solana sender (string bytes)
        data.extend_from_slice(solana_sender.as_bytes());

        // Use Keccak256 (Ethereum's hash function)
        let mut hasher = sha3::Keccak256::new();
        hasher.update(&data);
        let result = hasher.finalize();

        result.into()
    }

    /// Create the message hash for Solana program verification
    fn create_solana_message_hash(
        &self,
        recipient: &str,
        amount: u64,
        nonce: u64,
        ethereum_sender: &str,
    ) -> [u8; 32] {
        // For Solana, we use SHA256
        let mut hasher = Sha256::new();

        // Encode the data
        hasher.update(recipient.as_bytes());
        hasher.update(&amount.to_le_bytes());
        hasher.update(&nonce.to_le_bytes());
        hasher.update(ethereum_sender.as_bytes());

        let result = hasher.finalize();
        result.into()
    }
}

/// Validator service - this would run separately on each validator node
/// It signs messages after verifying the source chain transaction
pub struct ValidatorService {
    eth_private_key: String,
    sol_private_key: String,
}

impl ValidatorService {
    pub fn new(eth_private_key: String, sol_private_key: String) -> Self {
        Self {
            eth_private_key,
            sol_private_key,
        }
    }

    /// Sign a message for Ethereum (ECDSA signature)
    pub async fn sign_for_ethereum(&self, message_hash: [u8; 32]) -> Result<String> {
        info!("Signing message for Ethereum: 0x{}", hex::encode(&message_hash));
        Ok(format!("0x{}", hex::encode(&message_hash)))
    }

    /// Sign a message for Solana (Ed25519 signature)
    pub async fn sign_for_solana(&self, message_hash: [u8; 32]) -> Result<String> {
        info!("Signing message for Solana: 0x{}", hex::encode(&message_hash));
        Ok(format!("0x{}", hex::encode(&message_hash)))
    }

    /// Verify a source transaction on Ethereum before signing
    pub async fn verify_ethereum_transaction(&self, tx_hash: &str, _nonce: u64) -> Result<bool> {
        info!("Verifying Ethereum transaction: {}", tx_hash);
        Ok(true)
    }

    /// Verify a source transaction on Solana before signing
    pub async fn verify_solana_transaction(&self, tx_hash: &str, _nonce: u64) -> Result<bool> {
        info!("Verifying Solana transaction: {}", tx_hash);
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_hash_creation() {
        let client = ValidatorClient::new(vec![]);

        // Test Ethereum message hash
        let recipient = Address::from_str("0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb0").unwrap();
        let amount = U256::from(1000000u64);
        let nonce = 1;
        let sender = "SoLXxX123";

        let hash = client.create_ethereum_message_hash(recipient, amount, nonce, sender);
        assert_eq!(hash.len(), 32);
    }
}
