// Declare all modules
pub mod error;
pub mod instruction;
pub mod processor;
pub mod state;

pub use error::BridgeError;
pub use instruction::BridgeInstruction;
pub use processor::process_instruction;
pub use state::{BridgeConfig, BridgeStatus, UserBridgeState};

#[cfg(not(feature = "no-entrypoint"))]
use solana_program::entrypoint;

#[cfg(not(feature = "no-entrypoint"))]
entrypoint!(process_instruction);

solana_program::declare_id!("BridgeVau1t11111111111111111111111111111111");
