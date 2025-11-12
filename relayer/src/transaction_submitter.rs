use crate::{
    config::{EthereumConfig, SolanaConfig},
    db::Database,
    error::{RelayerError, Result},
    types::{Chain, RelayerTransaction, TransactionStatus, ValidatorSignature},
    validator_client::ValidatorClient,
};
use alloy::{
    contract::CallBuilder,
    network::{Ethereum, EthereumWallet, TransactionBuilder},
    primitives::{Address, Bytes, U256},
    providers::{Provider, ProviderBuilder, RootProvider},
    rpc::types::TransactionRequest,
    signers::local::PrivateKeySigner,
    sol,
    transports::http::{Client, Http},
};
use borsh::BorshSerialize;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    transaction::Transaction,
};
use std::str::FromStr;
use tracing::{error, info, warn};

// TODO: Implement actual transaction submission for both chains
// TODO: Add gas estimation and nonce management for Ethereum
// TODO: Properly serialize Solana instructions

sol! {
    #[allow(missing_docs)]
    #[sol(rpc)]
    contract SolanaBridge {
        function mintWrapped(
            address recipient,
            uint256 amount,
            uint64 nonce,
            string memory solanaSender,
            bytes[] memory signatures
        ) external;
    }
}

pub struct TransactionSubmitter {
    solana_client: RpcClient,
    ethereum_provider: RootProvider<Http<Client>>,
    solana_config: SolanaConfig,
    ethereum_config: EthereumConfig,
    db: Database,
    validator_client: ValidatorClient,
    relayer_eth_signer: Option<PrivateKeySigner>,
    relayer_sol_keypair: Option<Keypair>,
}

impl TransactionSubmitter {
    pub fn new(
        solana_config: SolanaConfig,
        ethereum_config: EthereumConfig,
        db: Database,
        validator_client: ValidatorClient,
    ) -> Result<Self> {
        let solana_client = RpcClient::new_with_commitment(
            solana_config.rpc_url.clone(),
            CommitmentConfig::from_str(&solana_config.commitment)
                .map_err(|e| RelayerError::ConfigError(format!("Invalid commitment: {}", e)))?,
        );

        let ethereum_provider = ProviderBuilder::new().on_http(
            ethereum_config
                .rpc_url
                .parse()
                .map_err(|e| RelayerError::ConfigError(format!("Invalid RPC URL: {:?}", e)))?,
        );

        Ok(Self {
            solana_client,
            ethereum_provider,
            solana_config,
            ethereum_config,
            db,
            validator_client,
            relayer_eth_signer: None,
            relayer_sol_keypair: None,
        })
    }


    pub fn set_ethereum_signer(&mut self, private_key: &str) -> Result<()> {
        let signer = PrivateKeySigner::from_str(private_key)
            .map_err(|e| RelayerError::ConfigError(format!("Invalid private key: {}", e)))?;

        self.relayer_eth_signer = Some(signer);
        Ok(())
    }

 
    pub fn set_solana_keypair(&mut self, keypair_bytes: &[u8]) -> Result<()> {
        let keypair = Keypair::from_bytes(keypair_bytes)
            .map_err(|e| RelayerError::ConfigError(format!("Invalid keypair: {}", e)))?;

        self.relayer_sol_keypair = Some(keypair);
        Ok(())
    }


    pub async fn process_transaction(&self, tx: &RelayerTransaction) -> Result<()> {
        info!("Processing transaction: nonce={}, status={}", tx.nonce, tx.status);

        match tx.status {
            TransactionStatus::Pending => {
                self.collect_signatures(tx).await?;
            }
            TransactionStatus::SignaturesCollected => {
                self.submit_to_destination(tx).await?;
            }
            TransactionStatus::Submitted => {
                self.check_confirmation(tx).await?;
            }
            TransactionStatus::Confirmed => {
                info!("Transaction {} already confirmed", tx.nonce);
            }
            TransactionStatus::Failed => {
                warn!("Transaction {} has failed status", tx.nonce);
            }
        }

        Ok(())
    }


    async fn collect_signatures(&self, tx: &RelayerTransaction) -> Result<()> {
        info!("Collecting signatures for nonce {}", tx.nonce);

        let signatures = match tx.to_chain {
            Chain::Ethereum => {
                let recipient = Address::from_str(&tx.recipient)
                    .map_err(|e| RelayerError::ParseError(format!("Invalid recipient address: {}", e)))?;

                self.validator_client
                    .collect_signatures_for_ethereum_mint(
                        recipient,
                        U256::from(tx.amount as u64),
                        tx.nonce as u64,
                        &tx.sender,
                    )
                    .await?
            }
            Chain::Solana => {
                self.validator_client
                    .collect_signatures_for_solana_unlock(
                        &tx.recipient,
                        tx.amount as u64,
                        tx.nonce as u64,
                        &tx.sender,
                    )
                    .await?
            }
            Chain::Sui => {
                return Err(RelayerError::InvalidChain("Sui not implemented".to_string()));
            }
        };

        // Store signatures in database
        let signatures_json = serde_json::to_string(&signatures)
            .map_err(|e| RelayerError::SerializationError(e))?;

        self.db.update_signatures(tx.id, &signatures_json).await?;

        info!("Collected {} signatures for nonce {}", signatures.len(), tx.nonce);
        Ok(())
    }

    async fn submit_to_destination(&self, tx: &RelayerTransaction) -> Result<()> {
        info!("Submitting transaction to {} for nonce {}", tx.to_chain, tx.nonce);

      
        let signatures: Vec<ValidatorSignature> = if let Some(ref sig_json) = tx.signatures {
            serde_json::from_str(sig_json)?
        } else {
            return Err(RelayerError::InvalidSignature("No signatures found".to_string()));
        };

        let tx_hash = match tx.to_chain {
            Chain::Ethereum => {
                self.submit_to_ethereum(tx, signatures).await?
            }
            Chain::Solana => {
                self.submit_to_solana(tx, signatures).await?
            }
            Chain::Sui => {
                return Err(RelayerError::InvalidChain("Sui not implemented".to_string()));
            }
        };

 
        self.db
            .update_transaction_status(
                tx.id,
                TransactionStatus::Submitted,
                Some(&tx_hash),
                None,
            )
            .await?;

        info!("Transaction submitted: {}", tx_hash);
        Ok(())
    }

   
    async fn submit_to_ethereum(
        &self,
        tx: &RelayerTransaction,
        signatures: Vec<ValidatorSignature>,
    ) -> Result<String> {
        info!("Submitting mint to Ethereum for nonce {}", tx.nonce);

    
        let signer = self
            .relayer_eth_signer
            .as_ref()
            .ok_or_else(|| RelayerError::ConfigError("Ethereum signer not configured".to_string()))?;


        let recipient = Address::from_str(&tx.recipient)
            .map_err(|e| RelayerError::ParseError(format!("Invalid recipient: {}", e)))?;
        let amount = U256::from(tx.amount as u64);
        let nonce = tx.nonce as u64;

  
        let signature_bytes: Vec<Bytes> = signatures
            .iter()
            .map(|s| Bytes::from(hex::decode(s.signature.trim_start_matches("0x")).unwrap_or_default()))
            .collect();


        let wallet = EthereumWallet::from(signer.clone());
        let provider = ProviderBuilder::new()
            .with_recommended_fillers()
            .wallet(wallet)
            .on_http(
                self.ethereum_config
                    .rpc_url
                    .parse()
                    .map_err(|e| RelayerError::ConfigError(format!("Invalid RPC URL: {:?}", e)))?,
            );


        let bridge_address = Address::from_str(&self.ethereum_config.bridge_contract)
            .map_err(|e| RelayerError::ConfigError(format!("Invalid bridge address: {}", e)))?;


        let contract = SolanaBridge::new(bridge_address, &provider);

    
        info!("Calling mintWrapped: recipient={}, amount={}, nonce={}", recipient, amount, nonce);

        let _call = contract.mintWrapped(
            recipient,
            amount,
            nonce,
            tx.sender.clone(),
            signature_bytes,
        );

        warn!("Ethereum transaction submission placeholder");
        Ok(format!("0x{}", hex::encode(&tx.nonce.to_le_bytes())))
    }


    async fn submit_to_solana(
        &self,
        tx: &RelayerTransaction,
        signatures: Vec<ValidatorSignature>,
    ) -> Result<String> {
        info!("Submitting unlock to Solana for nonce {}", tx.nonce);

        let _keypair = self
            .relayer_sol_keypair
            .as_ref()
            .ok_or_else(|| RelayerError::ConfigError("Solana keypair not configured".to_string()))?;

        let _user_pubkey = Pubkey::from_str(&tx.recipient)
            .map_err(|e| RelayerError::ParseError(format!("Invalid recipient pubkey: {}", e)))?;

        let _program_id = Pubkey::from_str(&self.solana_config.bridge_program_id)
            .map_err(|e| RelayerError::ParseError(format!("Invalid program ID: {}", e)))?;

        info!("Creating unlock instruction for nonce {}", tx.nonce);

        let mut _sig_bytes = Vec::new();
        for sig in &signatures {
            let sig_data = hex::decode(sig.signature.trim_start_matches("0x"))
                .unwrap_or_default();
            _sig_bytes.extend_from_slice(&sig_data);
        }

        let mut _instruction_data = vec![2u8];
        _instruction_data.extend_from_slice(&(tx.nonce as u64).to_le_bytes());

        warn!("Solana transaction submission placeholder");
        Ok(format!("solana_tx_{}", tx.nonce))
    }


    async fn check_confirmation(&self, tx: &RelayerTransaction) -> Result<()> {
        info!("Checking confirmation for nonce {}", tx.nonce);

        if let Some(ref tx_hash) = tx.to_tx_hash {
            let is_confirmed = match tx.to_chain {
                Chain::Ethereum => self.check_ethereum_confirmation(tx_hash).await?,
                Chain::Solana => self.check_solana_confirmation(tx_hash).await?,
                Chain::Sui => {
                    return Err(RelayerError::InvalidChain("Sui not implemented".to_string()));
                }
            };

            if is_confirmed {
                info!("Transaction {} confirmed!", tx.nonce);
                self.db
                    .update_transaction_status(tx.id, TransactionStatus::Confirmed, None, None)
                    .await?;
            } else {
                info!("Transaction {} not yet confirmed", tx.nonce);
            }
        }

        Ok(())
    }


    async fn check_ethereum_confirmation(&self, tx_hash: &str) -> Result<bool> {
        let _tx_hash_bytes = hex::decode(tx_hash.trim_start_matches("0x"))
            .map_err(|e| RelayerError::ParseError(format!("Invalid tx hash: {}", e)))?;

        warn!("Ethereum confirmation checking placeholder");
        Ok(false)
    }

    async fn check_solana_confirmation(&self, tx_hash: &str) -> Result<bool> {

        let signature = solana_sdk::signature::Signature::from_str(tx_hash)
            .map_err(|e| RelayerError::ParseError(format!("Invalid signature: {}", e)))?;


        match self.solana_client.get_signature_status(&signature).await {
            Ok(Some(status)) => {
                if let Err(e) = status {
                    error!("Transaction {} failed: {:?}", tx_hash, e);
                    return Ok(false);
                }
                Ok(true)
            }
            Ok(None) => {
                info!("Transaction {} not found yet", tx_hash);
                Ok(false)
            }
            Err(e) => {
                error!("Error checking transaction status: {}", e);
                Ok(false)
            }
        }
    }
}