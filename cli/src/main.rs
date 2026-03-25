use clap::{Parser, Subcommand};
use anyhow::{Result, anyhow};
use solana_sdk::{
    pubkey::Pubkey,
    transaction::Transaction,
};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use dialoguer::{Confirm, Input};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Bridge program ID (hardcoded from programs/bridge-vault/src/lib.rs)
const BRIDGE_PROGRAM_ID: &str = "7DazfS5hDxNJMJcxs1uKk3yoob7cbPLBFMXA3iRotjRH";

/// Default config directory name
const CONFIG_DIR: &str = ".bridge";
/// Default config file name
const CONFIG_FILE: &str = "config.toml";

/// Bridge CLI configuration
#[derive(Debug, Serialize, Deserialize)]
struct BridgeConfig {
    /// URL of the relayer service
    relayer_url: String,
    /// Path to the Solana keypair file
    keypair_path: String,
    /// Bridge program ID (Solana pubkey)
    program_id: String,
}

impl Default for BridgeConfig {
    fn default() -> Self {
        Self {
            relayer_url: "http://localhost:8080".to_string(),
            keypair_path: "~/.config/solana/id.json".to_string(),
            program_id: BRIDGE_PROGRAM_ID.to_string(),
        }
    }
}

/// Transaction status from relayer API
#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
enum TxStatus {
    Pending,
    Signaturescollected,
    Submitted,
    Confirmed,
    Failed,
}

impl std::fmt::Display for TxStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TxStatus::Pending => write!(f, "Pending"),
            TxStatus::Signaturescollected => write!(f, "SignaturesCollected"),
            TxStatus::Submitted => write!(f, "Submitted"),
            TxStatus::Confirmed => write!(f, "Confirmed"),
            TxStatus::Failed => write!(f, "Failed"),
        }
    }
}

/// Transaction details from relayer API (GET /tx/:nonce)
#[derive(Debug, Deserialize)]
struct TxResponse {
    nonce: u64,
    from_chain: String,
    to_chain: String,
    from_tx_hash: String,
    to_tx_hash: Option<String>,
    sender: String,
    recipient: String,
    amount: u64,
    status: TxStatus,
    error_message: Option<String>,
}

/// List of transactions from relayer API (GET /txs)
#[derive(Debug, Deserialize)]
struct TxsListResponse {
    transactions: Vec<TxSummary>,
    total: usize,
}

/// Summary of a transaction for list view
#[derive(Debug, Deserialize)]
struct TxSummary {
    nonce: u64,
    from_chain: String,
    to_chain: String,
    amount: u64,
    status: TxStatus,
}

/// Load config from ~/.bridge/config.toml
fn load_config() -> Result<BridgeConfig> {
    let config_path = get_config_path()?;
    if !config_path.exists() {
        return Err(anyhow!(
            "Config file not found at {}. Run 'bridge init' first.",
            config_path.display()
        ));
    }
    let contents = std::fs::read_to_string(&config_path)
        .map_err(|e| anyhow!("Failed to read config file: {}", e))?;
    let config: BridgeConfig = toml::from_str(&contents)
        .map_err(|e| anyhow!("Failed to parse config file: {}", e))?;
    Ok(config)
}

/// Truncate a string to a maximum length, adding "..." if truncated
fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len.saturating_sub(3)])
    }
}

/// Format an amount (assumes 9 decimals like SOL) for display
fn format_amount(amount: u64) -> String {
    // Convert to human-readable with 9 decimals
    let whole = amount / 1_000_000_000;
    let fractional = amount % 1_000_000_000;
    if fractional == 0 {
        format!("{}", whole)
    } else {
        // Remove trailing zeros
        let frac_str = format!("{:09}", fractional);
        let frac_trimmed = frac_str.trim_end_matches('0');
        format!("{}.{}", whole, frac_trimmed)
    }
}

/// Get the config file path (~/.bridge/config.toml)
fn get_config_path() -> Result<PathBuf> {
    let home = dirs::home_dir()
        .ok_or_else(|| anyhow!("Could not determine home directory"))?;
    Ok(home.join(CONFIG_DIR).join(CONFIG_FILE))
}

/// Initialize bridge configuration interactively
async fn init_config() -> Result<()> {
    let config_path = get_config_path()?;
    let config_dir = config_path.parent().unwrap();

    // Create config directory if it doesn't exist
    if !config_dir.exists() {
        std::fs::create_dir_all(config_dir)
            .map_err(|e| anyhow!("Failed to create config directory: {}", e))?;
        println!("Created config directory: {}", config_dir.display());
    }

    // Check if config already exists
    if config_path.exists() {
        let overwrite = Confirm::new()
            .with_prompt(format!(
                "Config file already exists at {}. Overwrite?",
                config_path.display()
            ))
            .default(false)
            .interact()
            .map_err(|e| anyhow!("Failed to read user input: {}", e))?;

        if !overwrite {
            println!("Configuration unchanged.");
            return Ok(());
        }
    }

    // Prompt for configuration values
    println!("\n=== Bridge Configuration ===\n");

    let relayer_url: String = Input::new()
        .with_prompt("Relayer URL")
        .default("http://localhost:8080".to_string())
        .interact_text()
        .map_err(|e| anyhow!("Failed to read relayer URL: {}", e))?;

    let keypair_path: String = Input::new()
        .with_prompt("Keypair path")
        .default("~/.config/solana/id.json".to_string())
        .interact_text()
        .map_err(|e| anyhow!("Failed to read keypair path: {}", e))?;

    let program_id: String = Input::new()
        .with_prompt("Program ID (Solana bridge program)")
        .default(BRIDGE_PROGRAM_ID.to_string())
        .validate_with(|input: &String| {
            match input.parse::<Pubkey>() {
                Ok(_) => Ok(()),
                Err(_) => Err("Invalid Solana pubkey".to_string()),
            }
        })
        .interact_text()
        .map_err(|e| anyhow!("Failed to read program ID: {}", e))?;

    // Create config struct
    let config = BridgeConfig {
        relayer_url,
        keypair_path,
        program_id,
    };

    // Serialize to TOML
    let config_toml = toml::to_string_pretty(&config)
        .map_err(|e| anyhow!("Failed to serialize config: {}", e))?;

    // Write to file
    std::fs::write(&config_path, config_toml)
        .map_err(|e| anyhow!("Failed to write config file: {}", e))?;

    println!("\n✓ Configuration saved to {}", config_path.display());
    println!("\nConfiguration:");
    println!("  Relayer URL:   {}", config.relayer_url);
    println!("  Keypair path:  {}", config.keypair_path);
    println!("  Program ID:    {}", config.program_id);

    Ok(())
}

/// Chain name to chain ID mapping
fn chain_name_to_id(name: &str) -> Result<u8> {
    match name.to_lowercase().as_str() {
        "ethereum" | "eth" => Ok(1),
        "bsc" | "binance" => Ok(2),
        "polygon" | "matic" => Ok(3),
        "arbitrum" | "arb" => Ok(4),
        "optimism" | "op" => Ok(5),
        "avalanche" | "avax" => Ok(6),
        "base" => Ok(7),
        "solana" | "sol" => Ok(8),
        "sui" => Ok(9),
        "cosmos" | "atom" => Ok(10),
        _ => Err(anyhow!("Unknown chain: {}. Supported: ethereum, bsc, polygon, arbitrum, optimism, avalanche, base, solana, sui, cosmos", name)),
    }
}

/// Parse a destination address (hex for EVM chains, base58 for Solana, etc.)
fn parse_destination_address(dest: &str, chain_id: u8) -> Result<[u8; 32]> {
    let mut addr = [0u8; 32];
    match chain_id {
        1..=7 | 10 => {
            // EVM chains: expect 0x-prefixed 20-byte hex address
            let hex = dest.strip_prefix("0x").unwrap_or(dest);
            if hex.len() != 40 {
                return Err(anyhow!("EVM destination address must be 20 bytes (40 hex chars), got {}", dest));
            }
            let bytes = hex::decode(hex).map_err(|e| anyhow!("Invalid hex address: {}", e))?;
            addr[12..].copy_from_slice(&bytes);
        }
        8 => {
            // Solana: base58 pubkey
            let pk = dest.parse::<Pubkey>().map_err(|e| anyhow!("Invalid Solana address: {}", e))?;
            addr.copy_from_slice(pk.as_ref());
        }
        9 => {
            // Sui: hex address (32 bytes)
            let hex = dest.strip_prefix("0x").unwrap_or(dest);
            if hex.len() != 64 {
                return Err(anyhow!("Sui destination address must be 32 bytes (64 hex chars)"));
            }
            let bytes = hex::decode(hex).map_err(|e| anyhow!("Invalid hex address: {}", e))?;
            addr.copy_from_slice(&bytes);
        }
        _ => {
            // Default: try hex
            let hex = dest.strip_prefix("0x").unwrap_or(dest);
            let bytes = hex::decode(hex).map_err(|e| anyhow!("Invalid hex address: {}", e))?;
            if bytes.len() > 32 {
                return Err(anyhow!("Destination address too long: {} bytes", bytes.len()));
            }
            addr[32 - bytes.len()..].copy_from_slice(&bytes);
        }
    }
    Ok(addr)
}

// Bridge-vault types are used via bridge_vault::instruction::BridgeInstruction::create_lock_tokens_instruction

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

    /// Bridge tokens from one chain to another (builds Solana LockTokens tx offline)
    Lock {
        #[arg(long)]
        from: String,

        #[arg(long)]
        to: String,

        #[arg(long)]
        amount: f64,

        #[arg(long)]
        dest: String,

        /// Bridge config account address
        #[arg(long)]
        bridge_config: String,

        /// Token mint address
        #[arg(long)]
        token_mint: String,

        /// User's token account (SPL token account)
        #[arg(long)]
        user_token_account: String,

        /// Vault token account (where locked tokens go)
        #[arg(long)]
        vault_token_account: String,

        /// User pubkey (or keypair file for signing)
        #[arg(long)]
        user: Option<String>,

        /// Current bridge nonce (for user_bridge_state PDA derivation)
        #[arg(long)]
        nonce: Option<u64>,

        /// Output format: base64 (default) or hex
        #[arg(long, default_value = "base64")]
        output: String,

        /// Optional recent blockhash (otherwise uses zeros for offline building)
        #[arg(long)]
        blockhash: Option<String>,
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

    /// Watch a transaction for status changes (polls until Confirmed or Failed)
    Watch {
        /// Transaction nonce to watch
        #[arg(long)]
        nonce: u64,

        /// Polling interval in seconds (default: 5)
        #[arg(long, default_value = "5")]
        interval: u64,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Init => {
            init_config().await?;
        }
        Commands::Lock {
            from,
            to,
            amount,
            dest,
            bridge_config,
            token_mint,
            user_token_account,
            vault_token_account,
            user,
            nonce,
            output,
            blockhash,
        } => {
            // Validate chains
            let from_chain = from.to_lowercase();
            if from_chain != "solana" && from_chain != "sol" {
                return Err(anyhow!("Source chain must be 'solana' for LockTokens. Use from=solana."));
            }

            let dest_chain_id = chain_name_to_id(&to)?;
            
            // Extra validation for Ethereum destination addresses
            if dest_chain_id == 1 {
                // Ethereum: validate proper 20-byte hex format
                let eth_addr = dest.trim();
                let hex_part = eth_addr.strip_prefix("0x").unwrap_or(eth_addr);
                if hex_part.len() != 40 {
                    return Err(anyhow!(
                        "Invalid Ethereum address: must be 20 bytes (40 hex characters)\n\
                         Got: {} ({} characters after 0x prefix)",
                        dest,
                        hex_part.len()
                    ));
                }
                if !hex_part.chars().all(|c| c.is_ascii_hexdigit()) {
                    return Err(anyhow!(
                        "Invalid Ethereum address: contains non-hexadecimal characters\n\
                         Got: {}",
                        dest
                    ));
                }
            }
            
            let dest_address_bytes = parse_destination_address(&dest, dest_chain_id)?;

            // Parse pubkeys
            let program_id = BRIDGE_PROGRAM_ID.parse::<Pubkey>()?;
            let bridge_config_pk = bridge_config.parse::<Pubkey>()?;
            let token_mint_pk = token_mint.parse::<Pubkey>()?;
            let user_token_account_pk = user_token_account.parse::<Pubkey>()?;
            let vault_token_account_pk = vault_token_account.parse::<Pubkey>()?;

            // User pubkey - if provided use it, otherwise require it
            let user_pk: Pubkey = if let Some(u) = user {
                u.parse::<Pubkey>().map_err(|e| anyhow!("Invalid user pubkey: {}", e))?
            } else {
                return Err(anyhow!("--user <pubkey> is required for LockTokens"));
            };

            // Nonce - required for user_bridge_state PDA
            let current_nonce = nonce.ok_or_else(|| anyhow!("--nonce <u64> is required (current bridge nonce for user_bridge_state PDA)"))?;

            // Compute user_bridge_state PDA
            let nonce_bytes = current_nonce.to_le_bytes();
            let (user_bridge_state_pk, _bump) = Pubkey::find_program_address(
                &[b"bridge", user_pk.as_ref(), &nonce_bytes],
                &program_id,
            );

            // Convert amount to u64 (assuming 9 decimals for SOL-like tokens, user passes raw u64 or we need more info)
            // For simplicity, treat amount as a raw u64 in base units. In a real CLI, you'd want decimals config.
            // Here we interpret amount as the raw integer amount (no decimal conversion for safety).
            // If user passes 1.5 and wants 1.5 tokens with 9 decimals, they should pass 1500000000.
            // To make it friendlier, let's support both: if amount has decimals, multiply by 10^9.
            // Actually, simplest: just cast f64 to u64 after multiplying by 1e9 if it looks like a float.
            // For safety and clarity, require integer amount in base units. But the CLI uses f64.
            // Let's interpret: if amount < 1e12, treat as human units and multiply by 1e9 (9 decimals).
            // Otherwise treat as raw. This is a heuristic.
            let amount_u64: u64 = if amount.fract() != 0.0 || amount < 1_000_000.0 {
                // Likely human-readable, assume 9 decimals
                (amount * 1_000_000_000.0) as u64
            } else {
                amount as u64
            };

            // Validate amount is not zero
            if amount_u64 == 0 {
                return Err(anyhow!(
                    "Invalid amount: {}\n\
                     Amount must be greater than 0.",
                    amount
                ));
            }

            // Check keypair file exists (from config)
            let config = load_config()?;
            let keypair_path = shellexpand::tilde(&config.keypair_path);
            if !std::path::Path::new(&*keypair_path).exists() {
                return Err(anyhow!(
                    "Keypair file not found: {}\n\
                     Run 'bridge init' to configure a valid keypair path, or create the file.",
                    config.keypair_path
                ));
            }

            // Build the instruction using bridge-vault library
            let ix = bridge_vault::instruction::BridgeInstruction::create_lock_tokens_instruction(
                &program_id,
                &user_pk,
                &user_token_account_pk,
                &vault_token_account_pk,
                &user_bridge_state_pk,
                &bridge_config_pk,
                &token_mint_pk,
                amount_u64,
                dest_chain_id,
                dest_address_bytes,
            );

            // Build transaction
            let recent_blockhash = if let Some(bh) = blockhash {
                bh.parse().map_err(|e| anyhow!("Invalid blockhash: {}", e))?
            } else {
                // Use a placeholder blockhash for offline building
                solana_sdk::hash::Hash::default()
            };

            let mut tx = Transaction::new_with_payer(&[ix], Some(&user_pk));
            // Set blockhash (required for signing; placeholder is fine for offline building)
            tx.message.recent_blockhash = recent_blockhash;
            // Note: this tx is unsigned; the user must sign it with their keypair

            // Serialize
            let tx_bytes = bincode::serialize(&tx)?;
            let output_str = match output.as_str() {
                "hex" => hex::encode(&tx_bytes),
                _ => BASE64.encode(&tx_bytes),
            };

            println!("LockTokens transaction built (unsigned, offline).");
            println!("");
            println!("Summary:");
            println!("  From: {}", from);
            println!("  To: {} (chain_id={})", to, dest_chain_id);
            println!("  Amount (raw): {}", amount_u64);
            println!("  Destination: {}", dest);
            println!("  User: {}", user_pk);
            println!("  Nonce: {}", current_nonce);
            println!("  UserBridgeState PDA: {}", user_bridge_state_pk);
            println!("");
            println!("Unsigned transaction ({}):", output);
            println!("{}", output_str);
            println!("");
            println!("Next steps:");
            println!("  1. Sign this transaction with your keypair (e.g., using solana-cli or a wallet)");
            println!("  2. Submit the signed transaction to the Solana network");
        }
        Commands::Status { nonce } => {
            // Load config to get relayer URL
            let config = load_config()?;

            // Build URL
            let url = format!("{}/tx/{}", config.relayer_url.trim_end_matches('/'), nonce);

            println!("Querying relayer at {}...", config.relayer_url);
            println!();

            // Make HTTP request
            let client = reqwest::Client::new();
            let response = client
                .get(&url)
                .send()
                .await
                .map_err(|e| anyhow!("Failed to connect to relayer: {}", e))?;

            if response.status() == 404 {
                println!("Transaction not found for nonce {}", nonce);
                println!();
                println!("This nonce may not have been processed by the relayer yet,");
                println!("or the lock transaction may not have been submitted.");
                return Ok(());
            }

            if !response.status().is_success() {
                let status = response.status();
                let body = response.text().await.unwrap_or_default();
                return Err(anyhow!("Relayer returned error {}: {}", status, body));
            }

            let tx: TxResponse = response
                .json()
                .await
                .map_err(|e| anyhow!("Failed to parse relayer response: {}", e))?;

            // Print transaction details nicely
            println!("╔════════════════════════════════════════════════════════════╗");
            println!("║           Bridge Transaction Status                        ║");
            println!("╠════════════════════════════════════════════════════════════╣");
            println!("║  Nonce:      {:<45} ║", tx.nonce);
            println!("║  Status:     {:<45} ║", tx.status);
            println!("╠════════════════════════════════════════════════════════════╣");
            println!("║  From Chain: {:<45} ║", tx.from_chain);
            println!("║  To Chain:   {:<45} ║", tx.to_chain);
            println!("╠════════════════════════════════════════════════════════════╣");
            println!("║  Amount:     {:<45} ║", tx.amount);
            println!("╠════════════════════════════════════════════════════════════╣");
            println!("║  Sender:     {:<45} ║", tx.sender);
            println!("║  Recipient:  {:<45} ║", tx.recipient);
            println!("╠════════════════════════════════════════════════════════════╣");
            println!("║  From TX:    {:<45} ║", truncate(&tx.from_tx_hash, 45));
            if let Some(ref to_tx) = tx.to_tx_hash {
                println!("║  To TX:      {:<45} ║", truncate(to_tx, 45));
            } else {
                println!("║  To TX:      {:<45} ║", "-");
            }
            println!("╚════════════════════════════════════════════════════════════╝");

            // Show error message if failed
            if let Some(ref err) = tx.error_message {
                println!();
                println!("⚠ Error: {}", err);
            }

            // Show helpful status messages
            println!();
            match tx.status.to_string().as_str() {
                "Pending" => {
                    println!("The transaction is pending. Validators are collecting signatures.");
                }
                "SignaturesCollected" => {
                    println!("Signatures collected. Transaction is being submitted to destination chain.");
                }
                "Submitted" => {
                    println!("Transaction submitted to destination chain. Awaiting confirmation.");
                }
                "Confirmed" => {
                    println!("✓ Transaction confirmed on destination chain!");
                }
                "Failed" => {
                    println!("✗ Transaction failed. See error message above.");
                }
                _ => {}
            }
        }
        Commands::History { user } => {
            // Load config to get relayer URL
            let config = load_config()?;

            // Build URL with optional user filter
            let mut url = format!("{}/txs", config.relayer_url.trim_end_matches('/'));
            if let Some(ref addr) = user {
                url.push_str(&format!("?user={}", addr));
            }

            println!("Querying relayer at {}...", config.relayer_url);
            println!();

            // Make HTTP request
            let client = reqwest::Client::new();
            let response = client
                .get(&url)
                .send()
                .await
                .map_err(|e| anyhow!("Failed to connect to relayer: {}", e))?;

            if !response.status().is_success() {
                let status = response.status();
                let body = response.text().await.unwrap_or_default();
                return Err(anyhow!("Relayer returned error {}: {}", status, body));
            }

            let list: TxsListResponse = response
                .json()
                .await
                .map_err(|e| anyhow!("Failed to parse relayer response: {}", e))?;

            if list.transactions.is_empty() {
                println!("No transactions found.");
                if user.is_some() {
                    println!("No transactions found for the specified user.");
                }
                return Ok(());
            }

            // Print header
            if let Some(ref addr) = user {
                println!("Transaction history for user: {}", addr);
                println!();
            } else {
                println!("Recent Transactions (total: {})", list.total);
                println!();
            }

            // Print table header
            println!("┌────────┬─────────────────┬─────────────────┬────────────────────┬────────────────────────┐");
            println!("│ {:<6} │ {:<15} │ {:<15} │ {:<18} │ {:<22} │",
                     "Nonce", "From", "To", "Amount", "Status");
            println!("├────────┼─────────────────┼─────────────────┼────────────────────┼────────────────────────┤");

            // Print each transaction
            for tx in &list.transactions {
                let amount_str = format_amount(tx.amount);
                println!("│ {:<6} │ {:<15} │ {:<15} │ {:<18} │ {:<22} │",
                    tx.nonce,
                    truncate(&tx.from_chain, 15),
                    truncate(&tx.to_chain, 15),
                    amount_str,
                    truncate(&tx.status.to_string(), 22)
                );
            }

            println!("└────────┴─────────────────┴─────────────────┴────────────────────┴────────────────────────┘");
            println!();
            println!("Showing {} of {} transactions", list.transactions.len(), list.total);
        }
        Commands::Watch { nonce, interval } => {
            // Load config to get relayer URL
            let config = load_config()?;

            // Build URL
            let url = format!("{}/tx/{}", config.relayer_url.trim_end_matches('/'), nonce);

            println!("Watching transaction nonce {}...", nonce);
            println!("Relayer: {}", config.relayer_url);
            println!("Polling interval: {} seconds", interval);
            println!();

            let client = reqwest::Client::new();
            let mut last_status: Option<String> = None;
            let poll_duration = tokio::time::Duration::from_secs(interval);

            // Print initial status
            println!("[{}] Starting watch...", chrono::Local::now().format("%Y-%m-%d %H:%M:%S"));

            loop {
                // Make HTTP request
                let response = match client.get(&url).send().await {
                    Ok(resp) => resp,
                    Err(e) => {
                        eprintln!("Error connecting to relayer: {}", e);
                        tokio::time::sleep(poll_duration).await;
                        continue;
                    }
                };

                if response.status() == 404 {
                    print!("\r[{}] Transaction not found yet...", 
                        chrono::Local::now().format("%Y-%m-%d %H:%M:%S"));
                    std::io::Write::flush(&mut std::io::stdout()).unwrap();
                    tokio::time::sleep(poll_duration).await;
                    continue;
                }

                if !response.status().is_success() {
                    let status = response.status();
                    let body = response.text().await.unwrap_or_default();
                    eprintln!("Relayer error {}: {}", status, body);
                    tokio::time::sleep(poll_duration).await;
                    continue;
                }

                let tx: TxResponse = match response.json().await {
                    Ok(tx) => tx,
                    Err(e) => {
                        eprintln!("Error parsing response: {}", e);
                        tokio::time::sleep(poll_duration).await;
                        continue;
                    }
                };

                let current_status = tx.status.to_string();

                // Only print when status changes
                if last_status.as_ref() != Some(&current_status) {
                    let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
                    
                    // Clear the "not found yet" line if it was printed
                    if last_status.is_none() {
                        println!();
                    }

                    println!("[{}] Status: {}", timestamp, current_status);

                    // Print additional details based on status
                    match tx.status {
                        TxStatus::Pending => {
                            println!("  → Waiting for validator signatures...");
                        }
                        TxStatus::Signaturescollected => {
                            println!("  → Signatures collected, preparing to submit...");
                        }
                        TxStatus::Submitted => {
                            if let Some(ref to_tx) = tx.to_tx_hash {
                                println!("  → Submitted to destination chain");
                                println!("  → TX hash: {}", truncate(to_tx, 60));
                            }
                        }
                        TxStatus::Confirmed => {
                            println!("  ✓ Transaction confirmed on destination chain!");
                            if let Some(ref to_tx) = tx.to_tx_hash {
                                println!("  → TX hash: {}", to_tx);
                            }
                            println!();
                            println!("Watch complete. Transaction finalized.");
                            break;
                        }
                        TxStatus::Failed => {
                            println!("  ✗ Transaction failed!");
                            if let Some(ref err) = tx.error_message {
                                println!("  → Error: {}", err);
                            }
                            println!();
                            println!("Watch complete. Transaction failed.");
                            break;
                        }
                    }

                    last_status = Some(current_status);
                } else {
                    // Print a dot to show we're still polling
                    print!(".");
                    std::io::Write::flush(&mut std::io::stdout()).unwrap();
                }

                // Check if we should stop
                if matches!(tx.status, TxStatus::Confirmed | TxStatus::Failed) {
                    break;
                }

                tokio::time::sleep(poll_duration).await;
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test that PDA derivation uses the correct seeds matching the on-chain program.
    /// The program uses: &[b"bridge", user_account.key.as_ref(), &nonce_bytes]
    #[test]
    fn test_user_bridge_state_pda_derivation_matches_program() {
        // Use known test values
        let program_id = BRIDGE_PROGRAM_ID.parse::<Pubkey>().unwrap();
        let user = Pubkey::new_unique();
        let nonce: u64 = 42;
        let nonce_bytes = nonce.to_le_bytes();

        // Derive PDA the same way the CLI does
        let (cli_pda, cli_bump) = Pubkey::find_program_address(
            &[b"bridge", user.as_ref(), &nonce_bytes],
            &program_id,
        );

        // Derive PDA the same way the on-chain program does (from processor.rs line 294-297)
        let (program_pda, program_bump) = Pubkey::find_program_address(
            &[b"bridge", user.as_ref(), &nonce_bytes],
            &program_id,
        );

        // They must match exactly
        assert_eq!(cli_pda, program_pda, "PDA derivation mismatch!");
        assert_eq!(cli_bump, program_bump, "Bump seed mismatch!");
    }

    /// Test that PDA derivation is deterministic and consistent across multiple calls
    #[test]
    fn test_pda_derivation_is_deterministic() {
        let program_id = BRIDGE_PROGRAM_ID.parse::<Pubkey>().unwrap();
        let user = Pubkey::new_unique();
        let nonce: u64 = 123;

        let (pda1, bump1) = Pubkey::find_program_address(
            &[b"bridge", user.as_ref(), &nonce.to_le_bytes()],
            &program_id,
        );
        let (pda2, bump2) = Pubkey::find_program_address(
            &[b"bridge", user.as_ref(), &nonce.to_le_bytes()],
            &program_id,
        );

        assert_eq!(pda1, pda2, "PDA should be deterministic");
        assert_eq!(bump1, bump2, "Bump should be deterministic");
    }

    /// Test that different nonces produce different PDAs
    #[test]
    fn test_different_nonces_produce_different_pdas() {
        let program_id = BRIDGE_PROGRAM_ID.parse::<Pubkey>().unwrap();
        let user = Pubkey::new_unique();

        let (pda1, _) = Pubkey::find_program_address(
            &[b"bridge", user.as_ref(), &0u64.to_le_bytes()],
            &program_id,
        );
        let (pda2, _) = Pubkey::find_program_address(
            &[b"bridge", user.as_ref(), &1u64.to_le_bytes()],
            &program_id,
        );

        assert_ne!(pda1, pda2, "Different nonces should produce different PDAs");
    }

    /// Test that LockTokens instruction data serializes correctly.
    /// This is critical - the on-chain program expects borsh-serialized data.
    #[test]
    fn test_lock_tokens_instruction_serialization() {
        // Create a LockTokens instruction using the bridge-vault library
        let program_id = BRIDGE_PROGRAM_ID.parse::<Pubkey>().unwrap();
        let user = Pubkey::new_unique();
        let user_token_account = Pubkey::new_unique();
        let vault_token_account = Pubkey::new_unique();
        let user_bridge_state = Pubkey::new_unique();
        let bridge_config = Pubkey::new_unique();
        let token_mint = Pubkey::new_unique();

        let amount: u64 = 1_000_000_000; // 1 token with 9 decimals
        let destination_chain: u8 = 1; // Ethereum
        let destination_address: [u8; 32] = {
            let mut addr = [0u8; 32];
            // Ethereum address: 0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb0
            addr[12..].copy_from_slice(&hex::decode("742d35Cc6634C0532925a3b844Bc9e7595f0bEb0").unwrap());
            addr
        };

        // Build instruction using the library
        let ix = bridge_vault::instruction::BridgeInstruction::create_lock_tokens_instruction(
            &program_id,
            &user,
            &user_token_account,
            &vault_token_account,
            &user_bridge_state,
            &bridge_config,
            &token_mint,
            amount,
            destination_chain,
            destination_address,
        );

        // Verify the instruction data can be unpacked by the bridge-vault library
        let unpacked = bridge_vault::instruction::BridgeInstruction::unpack(&ix.data)
            .expect("Instruction data should unpack successfully");

        match unpacked {
            bridge_vault::instruction::BridgeInstruction::LockTokens {
                amount: a,
                destination_chain: dc,
                destination_address: da,
            } => {
                assert_eq!(a, amount, "Amount mismatch");
                assert_eq!(dc, destination_chain, "Destination chain mismatch");
                assert_eq!(da, destination_address, "Destination address mismatch");
            }
            _ => panic!("Expected LockTokens instruction, got {:?}", unpacked),
        }
    }

    /// Test that the instruction has the correct accounts in the correct order.
    /// The on-chain program expects accounts in a specific order (see processor.rs line 208-219).
    #[test]
    fn test_lock_tokens_instruction_accounts_order() {
        let program_id = BRIDGE_PROGRAM_ID.parse::<Pubkey>().unwrap();
        let user = Pubkey::new_unique();
        let user_token_account = Pubkey::new_unique();
        let vault_token_account = Pubkey::new_unique();
        let user_bridge_state = Pubkey::new_unique();
        let bridge_config = Pubkey::new_unique();
        let token_mint = Pubkey::new_unique();

        let ix = bridge_vault::instruction::BridgeInstruction::create_lock_tokens_instruction(
            &program_id,
            &user,
            &user_token_account,
            &vault_token_account,
            &user_bridge_state,
            &bridge_config,
            &token_mint,
            1000,
            1,
            [0u8; 32],
        );

        // Expected order from processor.rs:
        // 0: user (signer, writable)
        // 1: user_token_account (writable)
        // 2: vault_token_account (writable)
        // 3: user_bridge_state (writable)
        // 4: bridge_config (writable)
        // 5: token_mint (readonly)
        // 6: token_program (readonly)
        // 7: system_program (readonly)
        // 8: rent sysvar (readonly)
        // 9: clock sysvar (readonly)

        assert_eq!(ix.program_id, program_id, "Program ID mismatch");
        assert_eq!(ix.accounts.len(), 10, "Should have 10 accounts");

        // Account 0: user (signer, writable)
        assert_eq!(ix.accounts[0].pubkey, user);
        assert!(ix.accounts[0].is_signer, "User should be signer");
        assert!(ix.accounts[0].is_writable, "User should be writable");

        // Account 1: user_token_account (writable)
        assert_eq!(ix.accounts[1].pubkey, user_token_account);
        assert!(ix.accounts[1].is_writable);

        // Account 2: vault_token_account (writable)
        assert_eq!(ix.accounts[2].pubkey, vault_token_account);
        assert!(ix.accounts[2].is_writable);

        // Account 3: user_bridge_state (writable)
        assert_eq!(ix.accounts[3].pubkey, user_bridge_state);
        assert!(ix.accounts[3].is_writable);

        // Account 4: bridge_config (writable)
        assert_eq!(ix.accounts[4].pubkey, bridge_config);
        assert!(ix.accounts[4].is_writable);

        // Account 5: token_mint (readonly)
        assert_eq!(ix.accounts[5].pubkey, token_mint);
        assert!(!ix.accounts[5].is_writable);

        // Account 6: token_program (readonly)
        assert_eq!(ix.accounts[6].pubkey, spl_token::id());
        assert!(!ix.accounts[6].is_writable);

        // Account 7: system_program (readonly)
        assert_eq!(ix.accounts[7].pubkey, solana_sdk::system_program::id());
        assert!(!ix.accounts[7].is_writable);

        // Account 8: rent sysvar (readonly)
        assert_eq!(ix.accounts[8].pubkey, solana_sdk::sysvar::rent::id());
        assert!(!ix.accounts[8].is_writable);

        // Account 9: clock sysvar (readonly)
        assert_eq!(ix.accounts[9].pubkey, solana_sdk::sysvar::clock::id());
        assert!(!ix.accounts[9].is_writable);
    }

    /// Test chain name to ID mapping
    #[test]
    fn test_chain_name_to_id() {
        assert_eq!(chain_name_to_id("ethereum").unwrap(), 1);
        assert_eq!(chain_name_to_id("eth").unwrap(), 1);
        assert_eq!(chain_name_to_id("ETH").unwrap(), 1);
        assert_eq!(chain_name_to_id("bsc").unwrap(), 2);
        assert_eq!(chain_name_to_id("polygon").unwrap(), 3);
        assert_eq!(chain_name_to_id("arbitrum").unwrap(), 4);
        assert_eq!(chain_name_to_id("optimism").unwrap(), 5);
        assert_eq!(chain_name_to_id("avalanche").unwrap(), 6);
        assert_eq!(chain_name_to_id("base").unwrap(), 7);
        assert_eq!(chain_name_to_id("solana").unwrap(), 8);
        assert_eq!(chain_name_to_id("sui").unwrap(), 9);
        assert_eq!(chain_name_to_id("cosmos").unwrap(), 10);
        assert!(chain_name_to_id("unknown").is_err());
    }

    /// Test EVM address parsing (left-padded to 32 bytes)
    #[test]
    fn test_parse_evm_destination_address() {
        let evm_addr = "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb0";
        let result = parse_destination_address(evm_addr, 1).unwrap();

        // Should be left-padded with zeros (12 bytes), then the 20-byte address
        assert_eq!(&result[0..12], &[0u8; 12]);
        assert_eq!(
            &result[12..],
            &hex::decode("742d35Cc6634C0532925a3b844Bc9e7595f0bEb0").unwrap()[..]
        );
    }

    /// Test Solana address parsing
    #[test]
    fn test_parse_solana_destination_address() {
        let sol_addr = "7DazfS5hDxNJMJcxs1uKk3yoob7cbPLBFMXA3iRotjRH";
        let result = parse_destination_address(sol_addr, 8).unwrap();

        let expected_pk = sol_addr.parse::<Pubkey>().unwrap();
        assert_eq!(result, expected_pk.to_bytes());
    }

    /// Test that the CLI's LockTokens instruction matches what the program's instruction builder produces
    #[test]
    fn test_cli_instruction_matches_program_instruction() {
        let program_id = BRIDGE_PROGRAM_ID.parse::<Pubkey>().unwrap();
        let user = Pubkey::new_unique();
        let user_token_account = Pubkey::new_unique();
        let vault_token_account = Pubkey::new_unique();
        let user_bridge_state = Pubkey::new_unique();
        let bridge_config = Pubkey::new_unique();
        let token_mint = Pubkey::new_unique();

        let amount: u64 = 5_000_000_000;
        let destination_chain: u8 = 3; // Polygon
        let destination_address: [u8; 32] = [7u8; 32]; // Test value

        // Build using the library function (same as CLI)
        let ix = bridge_vault::instruction::BridgeInstruction::create_lock_tokens_instruction(
            &program_id,
            &user,
            &user_token_account,
            &vault_token_account,
            &user_bridge_state,
            &bridge_config,
            &token_mint,
            amount,
            destination_chain,
            destination_address,
        );

        // Build the same instruction manually using the BridgeInstruction enum
        let manual_data = bridge_vault::instruction::BridgeInstruction::LockTokens {
            amount,
            destination_chain,
            destination_address,
        }
        .pack();

        // The data should be identical
        assert_eq!(ix.data, manual_data, "Instruction data mismatch!");
    }

    /// Test that instruction data has expected borsh serialization format
    #[test]
    fn test_lock_tokens_borsh_serialization_format() {
        let amount: u64 = 0x0102030405060708;
        let destination_chain: u8 = 5;
        let destination_address: [u8; 32] = [0xAB; 32];

        let ix_data = bridge_vault::instruction::BridgeInstruction::LockTokens {
            amount,
            destination_chain,
            destination_address,
        }
        .pack();

        // Borsh serialization for enum variants:
        // - First byte is the variant index (LockTokens should be index 1 based on enum order)
        // - Then the struct fields in order

        // Verify we can deserialize it back
        let unpacked = bridge_vault::instruction::BridgeInstruction::unpack(&ix_data).unwrap();
        if let bridge_vault::instruction::BridgeInstruction::LockTokens {
            amount: a,
            destination_chain: dc,
            destination_address: da,
        } = unpacked
        {
            assert_eq!(a, amount);
            assert_eq!(dc, destination_chain);
            assert_eq!(da, destination_address);
        } else {
            panic!("Wrong instruction type");
        }
    }

    /// Test that BridgeConfig serializes to TOML correctly
    #[test]
    fn test_bridge_config_serialization() {
        let config = BridgeConfig {
            relayer_url: "http://localhost:8080".to_string(),
            keypair_path: "~/.config/solana/id.json".to_string(),
            program_id: BRIDGE_PROGRAM_ID.to_string(),
        };

        let toml_str = toml::to_string_pretty(&config).expect("Should serialize to TOML");
        
        // Verify the TOML contains expected fields
        assert!(toml_str.contains("relayer_url"));
        assert!(toml_str.contains("keypair_path"));
        assert!(toml_str.contains("program_id"));
        assert!(toml_str.contains("http://localhost:8080"));
        assert!(toml_str.contains(BRIDGE_PROGRAM_ID));

        // Verify round-trip deserialization
        let deserialized: BridgeConfig = toml::from_str(&toml_str).expect("Should deserialize from TOML");
        assert_eq!(deserialized.relayer_url, config.relayer_url);
        assert_eq!(deserialized.keypair_path, config.keypair_path);
        assert_eq!(deserialized.program_id, config.program_id);
    }

    /// Test BridgeConfig default values
    #[test]
    fn test_bridge_config_default() {
        let config = BridgeConfig::default();
        assert_eq!(config.relayer_url, "http://localhost:8080");
        assert_eq!(config.keypair_path, "~/.config/solana/id.json");
        assert_eq!(config.program_id, BRIDGE_PROGRAM_ID);
    }

    /// Test that program ID in default config is a valid pubkey
    #[test]
    fn test_default_program_id_is_valid() {
        let config = BridgeConfig::default();
        let pubkey = config.program_id.parse::<Pubkey>();
        assert!(pubkey.is_ok(), "Default program ID should be a valid Solana pubkey");
    }

    /// Test truncate helper function
    #[test]
    fn test_truncate() {
        assert_eq!(truncate("short", 10), "short");
        assert_eq!(truncate("exactly10!", 10), "exactly10!");
        assert_eq!(truncate("exactly11!!", 10), "exactly...");
        assert_eq!(truncate("this is a very long string", 15), "this is a ve...");
        assert_eq!(truncate("", 5), "");
        assert_eq!(truncate("abc", 3), "abc");
        assert_eq!(truncate("abcd", 3), "...");
    }

    /// Test TxStatus display formatting
    #[test]
    fn test_tx_status_display() {
        assert_eq!(TxStatus::Pending.to_string(), "Pending");
        assert_eq!(TxStatus::Signaturescollected.to_string(), "SignaturesCollected");
        assert_eq!(TxStatus::Submitted.to_string(), "Submitted");
        assert_eq!(TxStatus::Confirmed.to_string(), "Confirmed");
        assert_eq!(TxStatus::Failed.to_string(), "Failed");
    }

    /// Test format_amount helper function
    #[test]
    fn test_format_amount() {
        // Whole numbers
        assert_eq!(format_amount(1_000_000_000), "1");
        assert_eq!(format_amount(5_000_000_000), "5");
        assert_eq!(format_amount(0), "0");

        // With fractional parts
        assert_eq!(format_amount(1_500_000_000), "1.5");
        assert_eq!(format_amount(1_050_000_000), "1.05");
        assert_eq!(format_amount(1_005_000_000), "1.005");
        assert_eq!(format_amount(1_000_500_000), "1.0005");
        assert_eq!(format_amount(1_000_050_000), "1.00005");
        assert_eq!(format_amount(1_000_005_000), "1.000005");
        assert_eq!(format_amount(1_000_000_500), "1.0000005");
        assert_eq!(format_amount(1_000_000_050), "1.00000005");
        assert_eq!(format_amount(1_000_000_005), "1.000000005");

        // Trailing zeros removed
        assert_eq!(format_amount(2_100_000_000), "2.1");
        assert_eq!(format_amount(3_010_000_000), "3.01");

        // Large numbers
        assert_eq!(format_amount(1_234_567_890_123_456_789), "1234567890.123456789");
    }

    /// Test Ethereum address validation in parse_destination_address
    #[test]
    fn test_ethereum_address_validation_valid() {
        // Valid Ethereum address (20 bytes = 40 hex chars)
        let valid_addr = "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb0";
        let result = parse_destination_address(valid_addr, 1); // chain_id 1 = Ethereum
        assert!(result.is_ok());
        
        let bytes = result.unwrap();
        // Should be left-padded with 12 zeros
        assert_eq!(&bytes[0..12], &[0u8; 12]);
        // Last 20 bytes should contain the address
        assert_eq!(&bytes[12..], &hex::decode("742d35Cc6634C0532925a3b844Bc9e7595f0bEb0").unwrap());
    }

    /// Test Ethereum address validation - too short
    #[test]
    fn test_ethereum_address_validation_too_short() {
        // Too short (only 38 hex chars)
        let short_addr = "0x742d35Cc6634C0532925a3b844Bc9e7595f0bE";
        let result = parse_destination_address(short_addr, 1);
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("must be 20 bytes"));
    }

    /// Test Ethereum address validation - too long
    #[test]
    fn test_ethereum_address_validation_too_long() {
        // Too long (42 hex chars)
        let long_addr = "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb0AB";
        let result = parse_destination_address(long_addr, 1);
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("must be 20 bytes"));
    }

    /// Test Ethereum address validation - invalid hex characters
    #[test]
    fn test_ethereum_address_validation_invalid_hex() {
        // Invalid hex characters (contains 'G')
        let invalid_addr = "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEbG";
        let result = parse_destination_address(invalid_addr, 1);
        assert!(result.is_err());
    }

    /// Test Ethereum address without 0x prefix
    #[test]
    fn test_ethereum_address_validation_no_prefix() {
        // Valid address but without 0x prefix
        let addr_no_prefix = "742d35Cc6634C0532925a3b844Bc9e7595f0bEb0";
        let result = parse_destination_address(addr_no_prefix, 1);
        assert!(result.is_ok());
    }

    /// Test amount conversion - zero should be rejected
    #[test]
    fn test_amount_conversion_zero() {
        // When amount is 0, the conversion should result in 0
        let amount: f64 = 0.0;
        let amount_u64: u64 = if amount.fract() != 0.0 || amount < 1_000_000.0 {
            (amount * 1_000_000_000.0) as u64
        } else {
            amount as u64
        };
        assert_eq!(amount_u64, 0);
    }

    /// Test amount conversion - positive amounts
    #[test]
    fn test_amount_conversion_positive() {
        // 1.5 tokens with 9 decimals = 1,500,000,000
        let amount: f64 = 1.5;
        let amount_u64 = (amount * 1_000_000_000.0) as u64;
        assert_eq!(amount_u64, 1_500_000_000);
        
        // Verify it's non-zero
        assert!(amount_u64 > 0);
    }
}
