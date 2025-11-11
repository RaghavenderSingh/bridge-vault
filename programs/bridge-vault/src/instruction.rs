use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    sysvar,
};

const SYSTEM_PROGRAM_ID: Pubkey = solana_program::pubkey!("11111111111111111111111111111111");

#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub enum BridgeInstruction {
    Initialize {
        admin: Pubkey,
        relayer_authority: Pubkey,
        fee_basis_points: u16,
        validators: Vec<Pubkey>,
        validator_threshold: u8,
    },
    LockTokens {
        amount: u64,
        destination_chain: u8,
        destination_address: [u8; 32],
    },
    UnlockTokens {
        nonce: u64,
        signatures: Vec<[u8; 64]>,
    },
    UpdateConfig {
        new_admin: Option<Pubkey>,
        new_relayer: Option<Pubkey>,
        new_fee: Option<u16>,
    },
    Pause,
    Unpause,
}

impl BridgeInstruction {
    pub fn pack(&self) -> Vec<u8> {
        borsh::to_vec(self).unwrap()
    }

    pub fn unpack(input: &[u8]) -> Result<Self, std::io::Error> {
        if input.is_empty() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Empty instruction data",
            ));
        }
        Self::try_from_slice(input)
    }

    pub fn create_initialize_instruction(
        program_id: &Pubkey,
        admin: &Pubkey,
        bridge_config: &Pubkey,
        vault_pda: &Pubkey,
        relayer_authority: &Pubkey,
        fee_basis_points: u16,
        validators: Vec<Pubkey>,
        validator_threshold: u8,
    ) -> Instruction {
        let accounts = vec![
            AccountMeta::new(*admin, true),
            AccountMeta::new(*bridge_config, false),
            AccountMeta::new_readonly(*vault_pda, false),
            AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false),
            AccountMeta::new_readonly(sysvar::rent::id(), false),
        ];

        Instruction {
            program_id: *program_id,
            accounts,
            data: Self::Initialize {
                admin: *admin,
                relayer_authority: *relayer_authority,
                fee_basis_points,
                validators,
                validator_threshold,
            }
            .pack(),
        }
    }

    pub fn create_lock_tokens_instruction(
        program_id: &Pubkey,
        user: &Pubkey,
        user_token_account: &Pubkey,
        vault_token_account: &Pubkey,
        user_bridge_state: &Pubkey,
        bridge_config: &Pubkey,
        token_mint: &Pubkey,
        amount: u64,
        destination_chain: u8,
        destination_address: [u8; 32],
    ) -> Instruction {
        let accounts = vec![
            AccountMeta::new(*user, true),
            AccountMeta::new(*user_token_account, false),
            AccountMeta::new(*vault_token_account, false),
            AccountMeta::new(*user_bridge_state, false),
            AccountMeta::new(*bridge_config, false),
            AccountMeta::new_readonly(*token_mint, false),
            AccountMeta::new_readonly(spl_token::id(), false),
            AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false),
            AccountMeta::new_readonly(sysvar::rent::id(), false),
            AccountMeta::new_readonly(sysvar::clock::id(), false),
        ];

        Instruction {
            program_id: *program_id,
            accounts,
            data: Self::LockTokens {
                amount,
                destination_chain,
                destination_address,
            }
            .pack(),
        }
    }

    pub fn create_unlock_tokens_instruction(
        program_id: &Pubkey,
        relayer: &Pubkey,
        user: &Pubkey,
        user_token_account: &Pubkey,
        vault_token_account: &Pubkey,
        vault_pda: &Pubkey,
        user_bridge_state: &Pubkey,
        bridge_config: &Pubkey,
        nonce: u64,
        signatures: Vec<[u8; 64]>,
    ) -> Instruction {
        let accounts = vec![
            AccountMeta::new_readonly(*relayer, true),
            AccountMeta::new_readonly(*user, false),
            AccountMeta::new(*user_token_account, false),
            AccountMeta::new(*vault_token_account, false),
            AccountMeta::new_readonly(*vault_pda, false),
            AccountMeta::new(*user_bridge_state, false),
            AccountMeta::new(*bridge_config, false),
            AccountMeta::new_readonly(spl_token::id(), false),
        ];

        Instruction {
            program_id: *program_id,
            accounts,
            data: Self::UnlockTokens { nonce, signatures }.pack(),
        }
    }

    pub fn create_update_config_instruction(
        program_id: &Pubkey,
        admin: &Pubkey,
        bridge_config: &Pubkey,
        new_admin: Option<Pubkey>,
        new_relayer: Option<Pubkey>,
        new_fee: Option<u16>,
    ) -> Instruction {
        let accounts = vec![
            AccountMeta::new_readonly(*admin, true),
            AccountMeta::new(*bridge_config, false),
        ];

        Instruction {
            program_id: *program_id,
            accounts,
            data: Self::UpdateConfig {
                new_admin,
                new_relayer,
                new_fee,
            }
            .pack(),
        }
    }

    pub fn create_pause_instruction(
        program_id: &Pubkey,
        admin: &Pubkey,
        bridge_config: &Pubkey,
    ) -> Instruction {
        let accounts = vec![
            AccountMeta::new_readonly(*admin, true),
            AccountMeta::new(*bridge_config, false),
        ];

        Instruction {
            program_id: *program_id,
            accounts,
            data: Self::Pause.pack(),
        }
    }

    pub fn create_unpause_instruction(
        program_id: &Pubkey,
        admin: &Pubkey,
        bridge_config: &Pubkey,
    ) -> Instruction {
        let accounts = vec![
            AccountMeta::new_readonly(*admin, true),
            AccountMeta::new(*bridge_config, false),
        ];

        Instruction {
            program_id: *program_id,
            accounts,
            data: Self::Unpause.pack(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_instruction_packing() {
        let init = BridgeInstruction::Initialize {
            admin: Pubkey::new_unique(),
            relayer_authority: Pubkey::new_unique(),
            fee_basis_points: 50,
            validators: vec![Pubkey::new_unique(), Pubkey::new_unique()],
            validator_threshold: 2,
        };

        let packed = init.pack();
        let unpacked = BridgeInstruction::unpack(&packed).unwrap();
        match unpacked {
            BridgeInstruction::Initialize {
                fee_basis_points,
                validator_threshold,
                ..
            } => {
                assert_eq!(fee_basis_points, 50);
                assert_eq!(validator_threshold, 2);
            }
            _ => panic!("Wrong instruction type"),
        }
    }

    #[test]
    fn test_lock_tokens_packing() {
        let lock = BridgeInstruction::LockTokens {
            amount: 1_000_000_000,
            destination_chain: 1,
            destination_address: [0u8; 32],
        };

        let packed = lock.pack();
        let unpacked = BridgeInstruction::unpack(&packed).unwrap();

        match unpacked {
            BridgeInstruction::LockTokens {
                amount,
                destination_chain,
                ..
            } => {
                assert_eq!(amount, 1_000_000_000);
                assert_eq!(destination_chain, 1);
            }
            _ => panic!("Wrong instruction type"),
        }
    }
}
