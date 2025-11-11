use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::pubkey::Pubkey;

#[derive(BorshSerialize, BorshDeserialize, Debug, Clone, Copy, PartialEq)]
#[repr(u8)]
pub enum BridgeStatus {
    Pending = 0,
    Completed = 1,
    Cancelled = 2,
}

#[derive(BorshDeserialize, BorshSerialize, Debug)]
pub struct BridgeConfig {
    pub admin: Pubkey,
    pub vault_pda_bump: u8,
    pub relayer_authority: Pubkey,
    pub fee_basis_points: u16,
    pub is_paused: bool,
    pub total_locked: u64,
    pub nonce: u64,
    pub validators: Vec<Pubkey>,
    pub validator_threshold: u8,
}

impl BridgeConfig {
    pub const LEN: usize = 256;
    pub const DISCRIMINATOR: &'static [u8] = b"bridgecfg";
    pub const MAX_VALIDATORS: usize = 5;
}

#[derive(BorshSerialize, BorshDeserialize, Debug)]

pub struct UserBridgeState {
    pub user: Pubkey,
    pub locked_amount: u64,
    pub token_mint: Pubkey,
    pub destination_chain: u8,
    pub destination_address: [u8; 32],
    pub status: BridgeStatus,
    pub nonce: u64,
    pub timestamp: i64,
    pub unlocked: bool,
}

impl UserBridgeState {
    pub const LEN: usize = 131;
    pub const DISCRIMINATOR: &'static [u8] = b"userbridge";
}

pub fn eth_address_to_bytes32(eth_address: &[u8; 20]) -> [u8; 32] {
    let mut bytes32 = [0u8; 32];
    bytes32[12..].copy_from_slice(eth_address);
    bytes32
}

pub fn bytes32_to_eth_address(bytes32: &[u8; 32]) -> [u8; 20] {
    let mut eth_address = [0u8; 20];
    eth_address.copy_from_slice(&bytes32[12..]);
    eth_address
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_bridge_status_values() {
        assert_eq!(BridgeStatus::Pending as u8, 0);
        assert_eq!(BridgeStatus::Completed as u8, 1);
        assert_eq!(BridgeStatus::Cancelled as u8, 2);
    }
    #[test]
    fn test_eth_address_conversion() {
        let eth_addr = [
            0x74, 0x2d, 0x35, 0xCc, 0x66, 0x34, 0xC0, 0x53, 0x29, 0x25, 0xa3, 0xb8, 0x44, 0xBc,
            0x9e, 0x75, 0x95, 0xf0, 0xbE, 0xb0,
        ];
        let bytes32 = eth_address_to_bytes32(&eth_addr);
        assert_eq!(&bytes32[0..12], &[0u8; 12]);
        assert_eq!(&bytes32[12..], &eth_addr[..]);
        let recovered = bytes32_to_eth_address(&bytes32);
        assert_eq!(recovered, eth_addr);
    }

    #[test]
    fn test_serialization() {
        let config = BridgeConfig {
            admin: Pubkey::new_unique(),
            relayer_authority: Pubkey::new_unique(),
            vault_pda_bump: 255,
            fee_basis_points: 50,
            is_paused: false,
            total_locked: 1_000_000_000,
            nonce: 42,
            validators: vec![Pubkey::new_unique(), Pubkey::new_unique(), Pubkey::new_unique()],
            validator_threshold: 2,
        };
        let serialized = borsh::to_vec(&config).unwrap();
        let deserialized = BridgeConfig::try_from_slice(&serialized).unwrap();
        assert_eq!(config.admin, deserialized.admin);
        assert_eq!(config.nonce, deserialized.nonce);
        assert_eq!(config.validators.len(), 3);
        assert_eq!(config.validator_threshold, 2);
    }
}
