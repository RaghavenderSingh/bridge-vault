use solana_program::program_error::ProgramError;
use thiserror::Error;

#[derive(Error, Debug, Copy, Clone)]
pub enum BridgeError {
    #[error("Unauthorized: Caller is not authorized to perform this action")]
    Unauthorized,

    #[error("Bridge is currently paused")]
    BridgePaused,

    #[error("Invalid nonce provided")]
    InvalidNonce,

    #[error("Signature threshold not met")]
    ThresholdNotMet,

    #[error("Account already initialized")]
    AlreadyInitialized,

    #[error("Arithmetic overflow")]
    Overflow,

    #[error("Account has incorrect owner")]
    IncorrectOwner,

    #[error("Account is not writable")]
    AccountNotWritable,

    #[error("Missing required signature")]
    MissingRequiredSignature,

    #[error("Invalid fee basis point (must be <= 10000)")]
    InvalidFee,

    #[error("Invalid bridge status for this operation")]
    InvalidStatus,

    #[error("Insufficient funds")]
    InsufficientFunds,

    #[error("Invalid destination chain")]
    InvalidDestination,

    #[error("Invalid PDA")]
    InvalidPDA,

    #[error("Tokens already unlocked")]
    AlreadyUnlocked,
}

impl From<BridgeError> for ProgramError {
    fn from(e: BridgeError) -> Self {
        ProgramError::Custom(e as u32)
    }
}
