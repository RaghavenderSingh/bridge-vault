use crate::{
    config::SolanaConfig,
    db::Database,
    error::{RelayerError, Result},
    types::{BridgeEvent, Chain},
};
use solana_client::{
    nonblocking::rpc_client::RpcClient,
    rpc_config::{RpcTransactionConfig, RpcTransactionLogsConfig, RpcTransactionLogsFilter},
    rpc_response::{Response, RpcLogsResponse},
};
use solana_sdk::{commitment_config::CommitmentConfig, pubkey::Pubkey, signature::Signature};
use solana_transaction_status::UiTransactionEncoding;
use std::str::FromStr;
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

// TODO: Use WebSocket subscriptions instead of polling

pub struct SolanaMonitor {
    rpc_client: RpcClient,
    program_id: Pubkey,
    db: Database,
    commitment: CommitmentConfig,
}

impl SolanaMonitor {
    pub fn new(config: &SolanaConfig, db: Database) -> Result<Self> {
        let rpc_client = RpcClient::new_with_commitment(
            config.rpc_url.clone(),
            CommitmentConfig::from_str(&config.commitment)
                .map_err(|e| RelayerError::ConfigError(format!("Invalid commitment: {}", e)))?,
        );

        let program_id = Pubkey::from_str(&config.bridge_program_id)
            .map_err(|e| RelayerError::ConfigError(format!("Invalid program ID: {}", e)))?;

        Ok(Self {
            rpc_client,
            program_id,
            db,
            commitment: CommitmentConfig::from_str(&config.commitment).unwrap(),
        })
    }

    /// Start monitoring Solana for bridge events
    pub async fn start(&self) -> Result<()> {
        info!("Starting Solana monitor for program: {}", self.program_id);

        // Get the current slot to start monitoring from
        let slot = self
            .rpc_client
            .get_slot()
            .await
            .map_err(|e| RelayerError::SolanaRpcError(format!("Failed to get slot: {}", e)))?;

        info!("Starting from slot: {}", slot);

        self.poll_for_transactions().await
    }


    async fn poll_for_transactions(&self) -> Result<()> {
        let mut last_signature: Option<Signature> = None;

        loop {
            match self
                .rpc_client
                .get_signatures_for_address(&self.program_id)
                .await
            {
                Ok(signatures) => {

                    for sig_info in signatures.iter().rev() {
                        let signature = Signature::from_str(&sig_info.signature)
                            .map_err(|e| RelayerError::ParseError(format!("Invalid signature: {}", e)))?;


                        if let Some(ref last_sig) = last_signature {
                            if signature == *last_sig {
                                continue;
                            }
                        }

            
                        if let Err(e) = self.process_transaction(&signature).await {
                            error!("Error processing transaction {}: {}", signature, e);
                        }

                        last_signature = Some(signature);
                    }
                }
                Err(e) => {
                    error!("Error fetching signatures: {}", e);
                }
            }

            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
        }
    }


    async fn process_transaction(&self, signature: &Signature) -> Result<()> {
        debug!("Processing transaction: {}", signature);

      
        let tx = self
            .rpc_client
            .get_transaction_with_config(
                signature,
                RpcTransactionConfig {
                    encoding: Some(UiTransactionEncoding::Json),
                    commitment: Some(self.commitment),
                    max_supported_transaction_version: Some(0),
                },
            )
            .await
            .map_err(|e| RelayerError::SolanaRpcError(format!("Failed to get transaction: {}", e)))?;

       
        if tx.transaction.meta.as_ref().and_then(|m| m.err.as_ref()).is_some() {
            debug!("Transaction {} failed, skipping", signature);
            return Ok(());
        }

     
        if let Some(meta) = tx.transaction.meta {
            let log_messages: Option<Vec<String>> = meta.log_messages.into();
            if let Some(log_messages) = log_messages {
                if let Some(event) = self.parse_logs(&log_messages, signature.to_string())? {
                    info!("Found bridge event: {:?}", event);
                    self.handle_event(event).await?;
                }
            }
        }

        Ok(())
    }


    fn parse_logs(&self, logs: &[String], tx_hash: String) -> Result<Option<BridgeEvent>> {
        let mut in_tokens_locked_event = false;
        let mut user: Option<String> = None;
        let mut amount: Option<u64> = None;
        let mut destination_chain: Option<u8> = None;
        let mut destination_address: Option<Vec<u8>> = None;
        let mut nonce: Option<u64> = None;

        for log in logs {
            if log.contains("EVENT: TokensLocked") {
                in_tokens_locked_event = true;
                continue;
            }

            if in_tokens_locked_event {
                if log.contains("user:") {
                    if let Some(value) = extract_value(log, "user:") {
                        user = Some(value);
                    }
                } else if log.contains("amount:") {
                    if let Some(value) = extract_value(log, "amount:") {
                        amount = value.parse().ok();
                    }
                } else if log.contains("destination_chain:") {
                    if let Some(value) = extract_value(log, "destination_chain:") {
                        destination_chain = value.parse().ok();
                    }
                } else if log.contains("destination_address:") {
                    if let Some(value) = extract_value(log, "destination_address:") {
                        destination_address = parse_address_bytes(&value);
                    }
                } else if log.contains("nonce:") {
                    if let Some(value) = extract_value(log, "nonce:") {
                        nonce = value.parse().ok();
                    }

                    if user.is_some() && amount.is_some() && destination_chain.is_some()
                        && destination_address.is_some() && nonce.is_some()
                    {
                        let user_val = user.take().unwrap();
                        let amount_val = amount.take().unwrap();
                        let dest_chain = destination_chain.take().unwrap();
                        let dest_addr = destination_address.take().unwrap();
                        let nonce_val = nonce.take().unwrap();

                        let to_chain = match dest_chain {
                            1 => Chain::Ethereum,
                            2 => Chain::Sui,
                            _ => return Err(RelayerError::ParseError(format!("Unknown destination chain: {}", dest_chain))),
                        };

                        let recipient = if to_chain == Chain::Ethereum {
                            format!("0x{}", hex::encode(&dest_addr))
                        } else {
                            hex::encode(&dest_addr)
                        };

                        return Ok(Some(BridgeEvent::TokensLocked {
                            from_chain: Chain::Solana,
                            to_chain,
                            sender: user_val,
                            recipient,
                            amount: amount_val,
                            nonce: nonce_val,
                            tx_hash,
                        }));
                    }
                }
            }
        }

        Ok(None)
    }


    async fn handle_event(&self, event: BridgeEvent) -> Result<()> {
        match &event {
            BridgeEvent::TokensLocked {
                from_chain,
                to_chain,
                sender,
                recipient,
                amount,
                nonce,
                tx_hash,
            } => {
                if self.db.is_nonce_processed(*nonce).await? {
                    warn!("Nonce {} already processed, skipping", nonce);
                    return Ok(());
                }

                info!(
                    "Processing TokensLocked event: nonce={}, amount={}, from={} to={}",
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
            BridgeEvent::TokensBurned { .. } => {
                warn!("Unexpected TokensBurned event from Solana");
            }
        }

        Ok(())
    }
}

fn extract_value(log: &str, key: &str) -> Option<String> {
    if let Some(pos) = log.find(key) {
        let after_key = &log[pos + key.len()..];
        Some(after_key.trim().to_string())
    } else {
        None
    }
}

fn parse_address_bytes(s: &str) -> Option<Vec<u8>> {
    let s = s.trim().trim_start_matches('[').trim_end_matches(']');
    let parts: Vec<&str> = s.split(',').map(|p| p.trim()).collect();

    let mut bytes = Vec::new();
    for part in parts {
        if let Ok(byte) = part.parse::<u8>() {
            bytes.push(byte);
        } else {
            return None;
        }
    }

    if bytes.is_empty() {
        None
    } else {
        Some(bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_value() {
        assert_eq!(
            extract_value("  user: 5XqZXqZXqZ", "user:"),
            Some("5XqZXqZXqZ".to_string())
        );
        assert_eq!(
            extract_value("  amount: 1000000", "amount:"),
            Some("1000000".to_string())
        );
    }

    #[test]
    fn test_parse_address_bytes() {
        assert_eq!(
            parse_address_bytes("[1, 2, 3, 4]"),
            Some(vec![1, 2, 3, 4])
        );
        assert_eq!(
            parse_address_bytes("[255, 0, 128]"),
            Some(vec![255, 0, 128])
        );
    }
}
