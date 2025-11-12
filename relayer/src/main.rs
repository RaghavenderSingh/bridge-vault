mod config;
mod db;
mod error;
mod ethereum_monitor;
mod solana_monitor;
mod transaction_submitter;
mod types;
mod validator_client;

use anyhow::Result;
use config::Config;
use db::Database;
use ethereum_monitor::EthereumMonitor;
use solana_monitor::SolanaMonitor;
use tokio::time::{interval, Duration};
use tracing::{error, info, warn};
use transaction_submitter::TransactionSubmitter;
use validator_client::ValidatorClient;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    info!("Multi-Chain Bridge Relayer starting...");
    info!("Version: {}", env!("CARGO_PKG_VERSION"));

    info!("Loading configuration...");
    let config = Config::from_env()?;
    info!("Configuration loaded");

    info!("Connecting to database...");
    let db = Database::new(&config.database.url, config.database.max_connections).await?;
    info!("Database connected");

    info!("Solana RPC: {}", config.solana.rpc_url);
    info!("Ethereum RPC: {}", config.ethereum.rpc_url);
    info!(
        "Bridge contracts - Solana: {}, Ethereum: {}",
        config.solana.bridge_program_id, config.ethereum.bridge_contract
    );
    info!("Validators: {}", config.validators.len());

    match db.get_stats().await {
        Ok(stats) => {
            info!("Transaction Statistics:");
            info!("  Total: {}", stats.total);
            info!("  Pending: {}", stats.pending);
            info!("  Signatures Collected: {}", stats.signatures_collected);
            info!("  Submitted: {}", stats.submitted);
            info!("  Confirmed: {}", stats.confirmed);
            info!("  Failed: {}", stats.failed);
        }
        Err(e) => warn!("Could not fetch stats: {}", e),
    }

    // Create monitors and submitter
    info!("Initializing chain monitors...");
    let solana_monitor = SolanaMonitor::new(&config.solana, db.clone())?;
    let ethereum_monitor = EthereumMonitor::new(&config.ethereum, db.clone())?;

    info!("Initializing validator client...");
    let validator_client = ValidatorClient::new(config.validators.clone());

    info!("Initializing transaction submitter...");
    let transaction_submitter = TransactionSubmitter::new(
        config.solana.clone(),
        config.ethereum.clone(),
        db.clone(),
        validator_client,
    )?;

    let shutdown = tokio::signal::ctrl_c();

    info!("Relayer is running!");
    info!("");
    info!("Monitoring chains:");
    info!("  Solana:   {}", config.solana.rpc_url);
    info!("  Ethereum: {}", config.ethereum.rpc_url);
    info!("");
    info!("Press Ctrl+C to stop");

    // Clone config for async blocks
    let solana_config = config.solana.clone();
    let ethereum_config = config.ethereum.clone();
    let relayer_config = config.relayer.clone();
    let db_clone1 = db.clone();
    let db_clone2 = db.clone();
    let db_clone3 = db.clone();

    tokio::select! {
        _ = shutdown => {
            info!("Shutdown signal received...");
        }
        result = async {
            // Start all tasks concurrently
            tokio::join!(
                // Monitor Solana for TokensLocked events
                async {
                    info!("Starting Solana monitor task...");
                    if let Err(e) = solana_monitor.start().await {
                        error!("Solana monitor error: {}", e);
                    }
                },
                // Monitor Ethereum for TokensBurned events
                async {
                    info!("Starting Ethereum monitor task...");
                    if let Err(e) = ethereum_monitor.start().await {
                        error!("Ethereum monitor error: {}", e);
                    }
                },
                // Process pending transactions
                async {
                    info!("Starting transaction processor task...");
                    if let Err(e) = process_transactions(db_clone3, transaction_submitter, relayer_config).await {
                        error!("Transaction processor error: {}", e);
                    }
                }
            )
        } => {}
    }

    info!("Performing cleanup...");
    info!("Relayer stopped gracefully");

    Ok(())
}

/// Process pending transactions from the database
async fn process_transactions(
    db: Database,
    submitter: TransactionSubmitter,
    config: config::RelayerConfig,
) -> Result<()> {
    let mut tick = interval(Duration::from_millis(config.poll_interval_ms));

    loop {
        tick.tick().await;

        match db.get_pending_transactions().await {
            Ok(pending) if !pending.is_empty() => {
                info!("Processing {} pending transactions", pending.len());

                for tx in pending {
                    match submitter.process_transaction(&tx).await {
                        Ok(_) => {
                            info!("Successfully processed transaction nonce={}", tx.nonce);
                        }
                        Err(e) => {
                            error!("Error processing transaction nonce={}: {}", tx.nonce, e);

                            // Update transaction as failed after max retries
                            // TODO: Implement retry counter logic
                        }
                    }
                }
            }
            Ok(_) => {
                // No pending transactions
            }
            Err(e) => {
                error!("Error fetching pending transactions: {}", e);
            }
        }
    }
}
