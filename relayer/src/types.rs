use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

/// Chain identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "TEXT")]
pub enum Chain {
    Solana,
    Ethereum,
    Sui,
}

impl std::fmt::Display for Chain {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Chain::Solana => write!(f, "Solana"),
            Chain::Ethereum => write!(f, "Ethereum"),
            Chain::Sui => write!(f, "Sui"),
        }
    }
}

/// Bridge event types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BridgeEvent {
    /// Tokens locked on source chain (ready to mint on destination)
    TokensLocked {
        from_chain: Chain,
        to_chain: Chain,
        sender: String,
        recipient: String,
        amount: u64,
        nonce: u64,
        tx_hash: String,
    },
    /// Tokens burned on destination chain (ready to unlock on source)
    TokensBurned {
        from_chain: Chain,
        to_chain: Chain,
        sender: String,
        recipient: String,
        amount: u64,
        nonce: u64,
        tx_hash: String,
    },
}

impl BridgeEvent {
    pub fn nonce(&self) -> u64 {
        match self {
            BridgeEvent::TokensLocked { nonce, .. } => *nonce,
            BridgeEvent::TokensBurned { nonce, .. } => *nonce,
        }
    }

    pub fn from_chain(&self) -> Chain {
        match self {
            BridgeEvent::TokensLocked { from_chain, .. } => *from_chain,
            BridgeEvent::TokensBurned { from_chain, .. } => *from_chain,
        }
    }

    pub fn to_chain(&self) -> Chain {
        match self {
            BridgeEvent::TokensLocked { to_chain, .. } => *to_chain,
            BridgeEvent::TokensBurned { to_chain, .. } => *to_chain,
        }
    }

    pub fn tx_hash(&self) -> &str {
        match self {
            BridgeEvent::TokensLocked { tx_hash, .. } => tx_hash,
            BridgeEvent::TokensBurned { tx_hash, .. } => tx_hash,
        }
    }
}

/// Transaction status in the relayer
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "TEXT")]
pub enum TransactionStatus {
    Pending,
    SignaturesCollected,
    Submitted,
    Confirmed,
    Failed,
}

impl std::fmt::Display for TransactionStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TransactionStatus::Pending => write!(f, "Pending"),
            TransactionStatus::SignaturesCollected => write!(f, "SignaturesCollected"),
            TransactionStatus::Submitted => write!(f, "Submitted"),
            TransactionStatus::Confirmed => write!(f, "Confirmed"),
            TransactionStatus::Failed => write!(f, "Failed"),
        }
    }
}

/// Relayer transaction record
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct RelayerTransaction {
    pub id: i64,
    pub nonce: i64,
    pub from_chain: Chain,
    pub to_chain: Chain,
    pub from_tx_hash: String,
    pub to_tx_hash: Option<String>,
    pub sender: String,
    pub recipient: String,
    pub amount: i64,
    pub status: TransactionStatus,
    pub signatures: Option<String>, // JSON array of signatures
    pub error_message: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Validator signature
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidatorSignature {
    pub validator_address: String,
    pub signature: String,
    pub signed_at: DateTime<Utc>,
}
