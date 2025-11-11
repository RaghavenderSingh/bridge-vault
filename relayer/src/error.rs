use thiserror::Error;

#[derive(Error, Debug)]
pub enum RelayerError {
    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("Database error: {0}")]
    DatabaseError(#[from] sqlx::Error),

    #[error("Solana RPC error: {0}")]
    SolanaRpcError(String),

    #[error("Ethereum RPC error: {0}")]
    EthereumRpcError(String),

    #[error("Invalid signature: {0}")]
    InvalidSignature(String),

    #[error("Insufficient signatures: expected {expected}, got {got}")]
    InsufficientSignatures { expected: usize, got: usize },

    #[error("Transaction already processed: nonce {0}")]
    TransactionAlreadyProcessed(u64),

    #[error("Transaction submission failed: {0}")]
    TransactionSubmissionFailed(String),

    #[error("Invalid chain: {0}")]
    InvalidChain(String),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("Parse error: {0}")]
    ParseError(String),

    #[error("Network error: {0}")]
    NetworkError(String),

    #[error("Timeout error: operation timed out")]
    TimeoutError,

    #[error("Unknown error: {0}")]
    Unknown(String),
}

pub type Result<T> = std::result::Result<T, RelayerError>;
