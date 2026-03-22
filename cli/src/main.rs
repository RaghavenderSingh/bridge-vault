mod config;

use clap::{Parser, Subcommand};
use anyhow::{Context, Result};
use colored::Colorize;
use config::CliConfig;
use dialoguer::{Confirm, Input};
use serde::{Deserialize, Serialize};

#[derive(Parser)]
#[command(name = "bridge")]
#[command(about = "Multi-chain bridge CLI - Solana, Ethereum, Sui", long_about = None)]
#[command(version = "0.1.0")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize bridge configuration
    Init,

    /// Bridge tokens from one chain to another
    Lock {
        #[arg(long)]
        from: String,

        #[arg(long)]
        to: String,

        #[arg(long)]
        amount: f64,

        #[arg(long)]
        dest: String,
    },

    /// Check bridge transaction status
    Status {
        #[arg(long)]
        nonce: u64,
    },

    /// View transaction history
    History {
        #[arg(long)]
        user: Option<String>,
    },
}

// API response types matching the relayer API
#[derive(Debug, Deserialize)]
struct ApiResponse<T> {
    success: bool,
    data: Option<T>,
    error: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
struct RelayerTransaction {
    id: i64,
    nonce: i64,
    from_chain: String,
    to_chain: String,
    from_tx_hash: String,
    to_tx_hash: Option<String>,
    sender: String,
    recipient: String,
    amount: i64,
    status: String,
    signatures: Option<String>,
    error_message: Option<String>,
    created_at: String,
    updated_at: String,
}

#[derive(Debug, Deserialize)]
struct TransactionListResponse {
    transactions: Vec<RelayerTransaction>,
    total: usize,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Init => cmd_init().await,
        Commands::Lock { from, to, amount, dest } => {
            cmd_lock(from, to, amount, dest).await
        }
        Commands::Status { nonce } => cmd_status(nonce).await,
        Commands::History { user } => cmd_history(user).await,
    }
}

async fn cmd_init() -> Result<()> {
    println!("{}", "Initializing bridge configuration...".bold().green());

    // Check if config already exists
    if CliConfig::exists()? {
        let overwrite = Confirm::new()
            .with_prompt("Configuration already exists. Overwrite?")
            .default(false)
            .interact()?;

        if !overwrite {
            println!("{}", "Configuration initialization cancelled.".yellow());
            return Ok(());
        }
    }

    // Prompt for configuration values
    let solana_rpc: String = Input::new()
        .with_prompt("Solana RPC URL")
        .default("https://api.devnet.solana.com".to_string())
        .interact_text()?;

    let ethereum_rpc: String = Input::new()
        .with_prompt("Ethereum RPC URL")
        .default("https://sepolia.infura.io/v3/YOUR_KEY".to_string())
        .interact_text()?;

    let bridge_program_id: String = Input::new()
        .with_prompt("Bridge Program ID (Solana)")
        .default("BridgeProgramId111111111111111111111111111111".to_string())
        .interact_text()?;

    let eth_contract: String = Input::new()
        .with_prompt("Bridge Contract Address (Ethereum)")
        .default("0x0000000000000000000000000000000000000000".to_string())
        .interact_text()?;

    let keypair_path: String = Input::new()
        .with_prompt("Keypair path")
        .default("~/.config/solana/id.json".to_string())
        .interact_text()?;

    let relayer_url: String = Input::new()
        .with_prompt("Relayer URL")
        .default("http://localhost:8080".to_string())
        .interact_text()?;

    // Create and save config
    let config = CliConfig {
        solana_rpc,
        ethereum_rpc,
        bridge_program_id,
        eth_contract,
        keypair_path,
        relayer_url,
    };

    config.save()?;

    let config_path = CliConfig::config_path()?;
    println!(
        "{} {}",
        "Configuration created at:".green().bold(),
        config_path.display()
    );

    Ok(())
}

async fn cmd_lock(from: String, to: String, amount: f64, dest: String) -> Result<()> {
    println!("{}", "Bridge Lock".bold().blue());
    println!("  From: {}", from.cyan());
    println!("  To: {}", to.cyan());
    println!("  Amount: {}", amount.to_string().cyan());
    println!("  Destination: {}", dest.cyan());
    println!();
    println!("{}", "This command is not yet implemented.".yellow());
    println!("Use the relayer to process bridge transactions.");

    Ok(())
}

async fn cmd_status(nonce: u64) -> Result<()> {
    let config = CliConfig::load()
        .context("Failed to load configuration. Run 'bridge init' first.")?;

    println!("{}", format!("Checking status for nonce {}...", nonce).bold().blue());

    let client = reqwest::Client::new();
    let url = format!("{}/tx/{}", config.relayer_url, nonce);

    match client.get(&url).send().await {
        Ok(response) => {
            if response.status().is_success() {
                match response.json::<ApiResponse<RelayerTransaction>>().await {
                    Ok(api_response) => {
                        if api_response.success {
                            if let Some(tx) = api_response.data {
                                print_transaction(&tx);
                            } else {
                                println!("{}", "Transaction not found.".red());
                            }
                        } else {
                            println!(
                                "{}",
                                format!("Error: {}", api_response.error.unwrap_or_default()).red()
                            );
                        }
                    }
                    Err(e) => {
                        println!("{}", format!("Failed to parse response: {}", e).red());
                    }
                }
            } else {
                println!(
                    "{}",
                    format!("HTTP Error: {}", response.status()).red()
                );
            }
        }
        Err(e) => {
            println!(
                "{}",
                format!("Failed to connect to relayer: {}", e).red()
            );
            println!("Make sure the relayer is running at {}", config.relayer_url);
        }
    }

    Ok(())
}

async fn cmd_history(_user: Option<String>) -> Result<()> {
    let config = CliConfig::load()
        .context("Failed to load configuration. Run 'bridge init' first.")?;

    println!("{}", "Transaction History".bold().blue());

    let client = reqwest::Client::new();
    let url = format!("{}/txs", config.relayer_url);

    match client.get(&url).send().await {
        Ok(response) => {
            if response.status().is_success() {
                match response.json::<ApiResponse<TransactionListResponse>>().await {
                    Ok(api_response) => {
                        if api_response.success {
                            if let Some(list) = api_response.data {
                                if list.transactions.is_empty() {
                                    println!("{}", "No transactions found.".yellow());
                                } else {
                                    println!("\nTotal transactions: {}\n", list.total);
                                    for tx in &list.transactions {
                                        print_transaction_summary(tx);
                                        println!();
                                    }
                                }
                            }
                        } else {
                            println!(
                                "{}",
                                format!("Error: {}", api_response.error.unwrap_or_default()).red()
                            );
                        }
                    }
                    Err(e) => {
                        println!("{}", format!("Failed to parse response: {}", e).red());
                    }
                }
            } else {
                println!(
                    "{}",
                    format!("HTTP Error: {}", response.status()).red()
                );
            }
        }
        Err(e) => {
            println!(
                "{}",
                format!("Failed to connect to relayer: {}", e).red()
            );
            println!("Make sure the relayer is running at {}", config.relayer_url);
        }
    }

    Ok(())
}

fn print_transaction(tx: &RelayerTransaction) {
    println!("\n{}", "Transaction Details".bold().underline());
    println!("  {}: {}", "ID".bold(), tx.id);
    println!("  {}: {}", "Nonce".bold(), tx.nonce);
    println!("  {}: {} → {}", "Chain".bold(), tx.from_chain.cyan(), tx.to_chain.cyan());
    println!("  {}: {}", "Sender".bold(), tx.sender);
    println!("  {}: {}", "Recipient".bold(), tx.recipient);
    println!("  {}: {}", "Amount".bold(), tx.amount);

    let status_color = match tx.status.as_str() {
        "Confirmed" => tx.status.green(),
        "Pending" => tx.status.yellow(),
        "Failed" => tx.status.red(),
        "Submitted" => tx.status.blue(),
        "SignaturesCollected" => tx.status.magenta(),
        _ => tx.status.normal(),
    };
    println!("  {}: {}", "Status".bold(), status_color);

    println!("  {}: {}", "From Tx".bold(), tx.from_tx_hash);
    if let Some(ref to_tx) = tx.to_tx_hash {
        println!("  {}: {}", "To Tx".bold(), to_tx);
    }
    if let Some(ref sigs) = tx.signatures {
        println!("  {}: {}", "Signatures".bold(), sigs);
    }
    if let Some(ref error) = tx.error_message {
        println!("  {}: {}", "Error".bold(), error.red());
    }
    println!("  {}: {}", "Created".bold(), tx.created_at);
    println!("  {}: {}", "Updated".bold(), tx.updated_at);
}

fn print_transaction_summary(tx: &RelayerTransaction) {
    let status_icon = match tx.status.as_str() {
        "Confirmed" => "✓".green(),
        "Pending" => "⏳".yellow(),
        "Failed" => "✗".red(),
        "Submitted" => "→".blue(),
        "SignaturesCollected" => "✎".magenta(),
        _ => "?".normal(),
    };

    println!(
        "  {} Nonce {}: {} → {} | {} → {} | {}",
        status_icon,
        tx.nonce.to_string().bold(),
        tx.from_chain,
        tx.to_chain,
        format!("{:.8}...", tx.sender),
        format!("{:.8}...", tx.recipient),
        tx.status
    );
}
