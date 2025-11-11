mod config;
mod db;
mod error;
mod types;

use anyhow::Result;
use config::Config;
use db::Database;
use tokio::time::{interval, Duration};
use tracing::{error, info, warn};

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

    let shutdown = tokio::signal::ctrl_c();

    info!("Relayer is running!");
    info!("");
    info!("Monitoring chains:");
    info!("  Solana:   {}", config.solana.rpc_url);
    info!("  Ethereum: {}", config.ethereum.rpc_url);
    info!("");
    info!("Press Ctrl+C to stop");

    let mut tick = interval(Duration::from_millis(config.relayer.poll_interval_ms));

    tokio::select! {
        _ = shutdown => {
            info!("Shutdown signal received...");
        }
        _ = async {
            loop {
                tick.tick().await;

                match db.get_pending_transactions().await {
                    Ok(pending) if !pending.is_empty() => {
                        info!("Processing {} pending transactions", pending.len());
                    }
                    Ok(_) => {}
                    Err(e) => {
                        error!("Error fetching pending transactions: {}", e);
                    }
                }
            }
        } => {}
    }

    info!("Performing cleanup...");
    info!("Relayer stopped gracefully");

    Ok(())
}
