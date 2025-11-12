use crate::{
    config::EthereumConfig,
    db::Database,
    error::{RelayerError, Result},
    types::{BridgeEvent, Chain},
};
use alloy::{
    primitives::Address,
    providers::{Provider, ProviderBuilder, RootProvider},
    rpc::types::{Filter, Log},
    sol,
    sol_types::SolEvent,
    transports::http::{Client, Http},
};
use std::str::FromStr;
use tracing::{debug, error, info, warn};

// TODO: Use WebSocket subscriptions instead of polling

sol! {
    #[allow(missing_docs)]
    #[sol(rpc)]
    contract SolanaBridge {
        event TokensMinted(
            address indexed recipient,
            uint256 amount,
            uint64 nonce,
            string solanaAddress
        );

        event TokensBurned(
            address indexed sender,
            uint256 amount,
            string solanaAddress,
            uint64 nonce
        );
    }
}

pub struct EthereumMonitor {
    provider: RootProvider<Http<Client>>,
    bridge_contract: Address,
    db: Database,
    confirmations: u64,
}

impl EthereumMonitor {
    pub fn new(config: &EthereumConfig, db: Database) -> Result<Self> {
        let provider = ProviderBuilder::new()
            .on_http(
                config
                    .rpc_url
                    .parse()
                    .map_err(|e| RelayerError::ConfigError(format!("Invalid RPC URL: {:?}", e)))?,
            );

        let bridge_contract = Address::from_str(&config.bridge_contract)
            .map_err(|e| RelayerError::ConfigError(format!("Invalid bridge contract address: {}", e)))?;

        Ok(Self {
            provider,
            bridge_contract,
            db,
            confirmations: config.confirmations,
        })
    }


    pub async fn start(&self) -> Result<()> {
        info!(
            "Starting Ethereum monitor for bridge contract: {}",
            self.bridge_contract
        );

    
        let latest_block = self
            .provider
            .get_block_number()
            .await
            .map_err(|e| RelayerError::EthereumRpcError(format!("Failed to get block number: {}", e)))?;

        info!("Starting from block: {}", latest_block);

        self.poll_for_events(latest_block).await
    }


    async fn poll_for_events(&self, start_block: u64) -> Result<()> {
        let mut last_block = start_block;

        loop {
    
            let current_block = match self.provider.get_block_number().await {
                Ok(block) => block,
                Err(e) => {
                    error!("Error fetching current block: {}", e);
                    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                    continue;
                }
            };


            if current_block > last_block + self.confirmations {
                let to_block = current_block - self.confirmations;

  
                let filter = Filter::new()
                    .address(self.bridge_contract)
                    .event(SolanaBridge::TokensBurned::SIGNATURE)
                    .from_block(last_block + 1)
                    .to_block(to_block);

                match self.provider.get_logs(&filter).await {
                    Ok(logs) => {
                        for log in logs {
                            if let Err(e) = self.process_log(log).await {
                                error!("Error processing log: {}", e);
                            }
                        }
                        last_block = to_block;
                    }
                    Err(e) => {
                        error!("Error fetching logs: {}", e);
                    }
                }
            }

         
            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
        }
    }


    async fn process_log(&self, log: Log) -> Result<()> {
        debug!("Processing log: {:?}", log);

        let alloy_log = alloy::primitives::Log::new(
            log.address(),
            log.topics().to_vec(),
            log.data().data.clone(),
        ).ok_or_else(|| RelayerError::ParseError("Failed to create log".to_string()))?;

        let decoded = SolanaBridge::TokensBurned::decode_log(&alloy_log, true)
            .map_err(|e| RelayerError::ParseError(format!("Failed to decode log: {}", e)))?;
        let event = decoded.data;

        info!(
            "TokensBurned event: sender={}, amount={}, solana_address={}, nonce={}",
            event.sender, event.amount, event.solanaAddress, event.nonce
        );

    
        if self.db.is_nonce_processed(event.nonce).await? {
            warn!("Nonce {} already processed, skipping", event.nonce);
            return Ok(());
        }

     
        let tx_hash = log
            .transaction_hash
            .ok_or_else(|| RelayerError::ParseError("Missing transaction hash".to_string()))?;

       
        let amount = event
            .amount
            .to::<u64>();

       
        let bridge_event = BridgeEvent::TokensBurned {
            from_chain: Chain::Ethereum,
            to_chain: Chain::Solana,
            sender: format!("{:?}", event.sender),
            recipient: event.solanaAddress.clone(),
            amount,
            nonce: event.nonce,
            tx_hash: format!("{:?}", tx_hash),
        };

        self.handle_event(bridge_event).await?;

        Ok(())
    }


    async fn handle_event(&self, event: BridgeEvent) -> Result<()> {
        match &event {
            BridgeEvent::TokensBurned {
                from_chain,
                to_chain,
                sender,
                recipient,
                amount,
                nonce,
                tx_hash,
            } => {
                info!(
                    "Processing TokensBurned event: nonce={}, amount={}, from={} to={}",
                    nonce, amount, from_chain, to_chain
                );

                let tx_id = self
                    .db
                    .create_transaction(
                        *nonce,
                        *from_chain,
                        *to_chain,
                        tx_hash,
                        sender,
                        recipient,
                        *amount,
                    )
                    .await?;

                info!("Created relayer transaction with ID: {}", tx_id);
            }
            BridgeEvent::TokensLocked { .. } => {

                warn!("Unexpected TokensLocked event from Ethereum");
            }
        }

        Ok(())
    }
}
