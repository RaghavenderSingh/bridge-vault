use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::{AccountInfo, next_account_info},
    clock::Clock,
    entrypoint::ProgramResult,
    msg,
    program::{invoke, invoke_signed},
    program_error::ProgramError,
    program_pack::Pack,
    pubkey::Pubkey,
    rent::Rent,
    system_instruction,
    sysvar::Sysvar,
};
use spl_token::state::Account as TokenAccount;
use sha2::{Sha256, Digest};

const SYSTEM_PROGRAM_ID: Pubkey = solana_program::pubkey!("11111111111111111111111111111111");

use crate::{
    error::BridgeError,
    instruction::BridgeInstruction,
    state::{BridgeConfig, BridgeStatus, UserBridgeState},
};


pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    let instruction = BridgeInstruction::unpack(instruction_data)
        .map_err(|_| ProgramError::InvalidInstructionData)?;

    match instruction {
        BridgeInstruction::Initialize {
            admin,
            relayer_authority,
            fee_basis_points,
            validators,
            validator_threshold,
        } => {
            msg!("Instruction: Initialize");
            process_initialize(
                program_id,
                accounts,
                admin,
                relayer_authority,
                fee_basis_points,
                validators,
                validator_threshold,
            )
        }
        BridgeInstruction::LockTokens {
            amount,
            destination_chain,
            destination_address,
        } => {
            msg!("Instruction: LockTokens");
            process_lock_tokens(
                program_id,
                accounts,
                amount,
                destination_chain,
                destination_address,
            )
        }
        BridgeInstruction::UnlockTokens { nonce, signatures } => {
            msg!("Instruction: UnlockTokens");
            process_unlock_tokens(program_id, accounts, nonce, signatures)
        }
        BridgeInstruction::UpdateConfig {
            new_admin,
            new_relayer,
            new_fee,
        } => {
            msg!("Instruction: UpdateConfig");
            process_update_config(program_id, accounts, new_admin, new_relayer, new_fee)
        }
        BridgeInstruction::Pause => {
            msg!("Instruction: Pause");
            process_pause(program_id, accounts)
        }
        BridgeInstruction::Unpause => {
            msg!("Instruction: Unpause");
            process_unpause(program_id, accounts)
        }
    }
}

fn process_initialize(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    admin: Pubkey,
    relayer_authority: Pubkey,
    fee_basis_points: u16,
    validators: Vec<Pubkey>,
    validator_threshold: u8,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let admin_account = next_account_info(account_info_iter)?;
    let bridge_config_account = next_account_info(account_info_iter)?;
    let vault_pda_account = next_account_info(account_info_iter)?;
    let system_program = next_account_info(account_info_iter)?;
    let _rent_sysvar = next_account_info(account_info_iter)?;
    let rent = Rent::get()?;

    if !admin_account.is_signer {
        msg!("Admin must sign the initialize transaction");
        return Err(BridgeError::MissingRequiredSignature.into());
    }

    if admin_account.key != &admin {
        msg!("Admin account key mismatch");
        return Err(BridgeError::Unauthorized.into());
    }

    if bridge_config_account.owner != &SYSTEM_PROGRAM_ID {
        msg!("Bridge config account already initialized");
        return Err(BridgeError::AlreadyInitialized.into());
    }

    if fee_basis_points > 10000 {
        msg!("Fee basis points must be <= 10000 (100%)");
        return Err(BridgeError::InvalidFee.into());
    }

    if validators.is_empty() || validators.len() > crate::state::BridgeConfig::MAX_VALIDATORS {
        msg!("Invalid number of validators (must be 1-5)");
        return Err(ProgramError::InvalidArgument.into());
    }

    if validator_threshold == 0 || validator_threshold as usize > validators.len() {
        msg!("Invalid validator threshold");
        return Err(ProgramError::InvalidArgument.into());
    }

    let (vault_pda, vault_bump) =
        Pubkey::find_program_address(&[b"vault", bridge_config_account.key.as_ref()], program_id);

    if vault_pda_account.key != &vault_pda {
        msg!("Invalid vault PDA provided");
        return Err(BridgeError::InvalidPDA.into());
    }

    let space = BridgeConfig::LEN;
    let rent_lamports = rent.minimum_balance(space);

    msg!(
        "Creating bridge config account with {} bytes, rent: {} lamports",
        space,
        rent_lamports
    );

    invoke(
        &system_instruction::create_account(
            admin_account.key,
            bridge_config_account.key,
            rent_lamports,
            space as u64,
            program_id,
        ),
        &[
            admin_account.clone(),
            bridge_config_account.clone(),
            system_program.clone(),
        ],
    )?;

    msg!("Bridge config account created successfully");

    let bridge_config = BridgeConfig {
        admin,
        relayer_authority,
        vault_pda_bump: vault_bump,
        fee_basis_points,
        is_paused: false,
        total_locked: 0,
        nonce: 0,
        validators,
        validator_threshold,
    };

    bridge_config
        .serialize(&mut &mut bridge_config_account.data.borrow_mut()[..])
        .map_err(|e| {
            msg!("Failed to serialize bridge config: {}", e);
            ProgramError::InvalidAccountData
        })?;

    msg!("Bridge initialized successfully");
    msg!("Admin: {}", admin);
    msg!("Relayer: {}", relayer_authority);
    msg!("Vault PDA: {}", vault_pda);
    msg!("Fee: {} basis points", fee_basis_points);

    Ok(())
}

fn process_lock_tokens(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    amount: u64,
    destination_chain: u8,
    destination_address: [u8; 32],
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let user_account = next_account_info(account_info_iter)?;
    let user_token_account = next_account_info(account_info_iter)?;
    let vault_token_account = next_account_info(account_info_iter)?;
    let user_bridge_state_account = next_account_info(account_info_iter)?;
    let bridge_config_account = next_account_info(account_info_iter)?;
    let token_mint_account = next_account_info(account_info_iter)?;
    let token_program = next_account_info(account_info_iter)?;
    let system_program = next_account_info(account_info_iter)?;
    let _rent_sysvar = next_account_info(account_info_iter)?;
    let _clock_sysvar = next_account_info(account_info_iter)?;

    let rent = Rent::get()?;
    let clock = Clock::get()?;

    if !user_account.is_signer {
        msg!("User must sign the lock transaction");
        return Err(BridgeError::MissingRequiredSignature.into());
    }

    if bridge_config_account.owner != program_id {
        msg!("Bridge config has incorrect owner");
        return Err(BridgeError::IncorrectOwner.into());
    }

    let mut bridge_config = BridgeConfig::try_from_slice(&bridge_config_account.data.borrow())
        .map_err(|_| ProgramError::InvalidAccountData)?;

    if bridge_config.is_paused {
        msg!("Bridge is currently paused");
        return Err(BridgeError::BridgePaused.into());
    }

    if amount == 0 {
        msg!("Lock amount must be greater than 0");
        return Err(BridgeError::InsufficientFunds.into());
    }

    if destination_chain == 0 || destination_chain > 10 {
        msg!("Invalid destination chain: {}", destination_chain);
        return Err(BridgeError::InvalidDestination.into());
    }

    let fee = amount
        .checked_mul(bridge_config.fee_basis_points as u64)
        .ok_or(BridgeError::Overflow)?
        .checked_div(10000)
        .ok_or(BridgeError::Overflow)?;

    let net_amount = amount.checked_sub(fee).ok_or(BridgeError::Overflow)?;

    msg!(
        "Lock amount: {}, Fee: {}, Net amount: {}",
        amount,
        fee,
        net_amount
    );

    let user_token_data = user_token_account.try_borrow_data()?;
    let user_token =
        TokenAccount::unpack(&user_token_data).map_err(|_| ProgramError::InvalidAccountData)?;

    if user_token.amount < amount {
        msg!(
            "Insufficient token balance. Have: {}, Need: {}",
            user_token.amount,
            amount
        );
        return Err(BridgeError::InsufficientFunds.into());
    }

    if user_token.mint != *token_mint_account.key {
        msg!("User token account mint mismatch");
        return Err(ProgramError::InvalidAccountData.into());
    }

    drop(user_token_data);

    let current_nonce = bridge_config.nonce;
    bridge_config.nonce = bridge_config
        .nonce
        .checked_add(1)
        .ok_or(BridgeError::Overflow)?;

    let nonce_bytes = current_nonce.to_le_bytes();
    let (user_bridge_state_pda, _user_bridge_bump) = Pubkey::find_program_address(
        &[b"bridge", user_account.key.as_ref(), &nonce_bytes],
        program_id,
    );

    if user_bridge_state_account.key != &user_bridge_state_pda {
        msg!("Invalid user bridge state PDA");
        return Err(BridgeError::InvalidPDA.into());
    }

    let space = UserBridgeState::LEN;
    let rent_lamports = rent.minimum_balance(space);

    msg!("Creating user bridge state account");

    invoke(
        &system_instruction::create_account(
            user_account.key,
            user_bridge_state_account.key,
            rent_lamports,
            space as u64,
            program_id,
        ),
        &[
            user_account.clone(),
            user_bridge_state_account.clone(),
            system_program.clone(),
        ],
    )?;

    let user_bridge_state = UserBridgeState {
        user: *user_account.key,
        locked_amount: net_amount,
        token_mint: *token_mint_account.key,
        destination_chain,
        destination_address,
        status: BridgeStatus::Pending,
        nonce: current_nonce,
        timestamp: clock.unix_timestamp,
        unlocked: false,
    };

    user_bridge_state
        .serialize(&mut &mut user_bridge_state_account.data.borrow_mut()[..])
        .map_err(|_| ProgramError::InvalidAccountData)?;

    msg!("User bridge state created with nonce: {}", current_nonce);

    msg!("Transferring {} tokens from user to vault", amount);

    let transfer_instruction = spl_token::instruction::transfer(
        token_program.key,
        user_token_account.key,
        vault_token_account.key,
        user_account.key,
        &[],
        amount,
    )?;

    invoke(
        &transfer_instruction,
        &[
            user_token_account.clone(),
            vault_token_account.clone(),
            user_account.clone(),
            token_program.clone(),
        ],
    )?;

    msg!("Token transfer successful");

    bridge_config.total_locked = bridge_config
        .total_locked
        .checked_add(net_amount)
        .ok_or(BridgeError::Overflow)?;

    bridge_config
        .serialize(&mut &mut bridge_config_account.data.borrow_mut()[..])
        .map_err(|_| ProgramError::InvalidAccountData)?;

    msg!("EVENT: TokensLocked");
    msg!("  user: {}", user_account.key);
    msg!("  token_mint: {}", token_mint_account.key);
    msg!("  amount: {}", net_amount);
    msg!("  destination_chain: {}", destination_chain);
    msg!("  destination_address: {:?}", destination_address);
    msg!("  nonce: {}", current_nonce);
    msg!("  timestamp: {}", clock.unix_timestamp);

    Ok(())
}

fn process_unlock_tokens(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    nonce: u64,
    signatures: Vec<[u8; 64]>,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let relayer_account = next_account_info(account_info_iter)?;
    let user_account = next_account_info(account_info_iter)?;
    let user_token_account = next_account_info(account_info_iter)?;
    let vault_token_account = next_account_info(account_info_iter)?;
    let vault_pda_account = next_account_info(account_info_iter)?;
    let user_bridge_state_account = next_account_info(account_info_iter)?;
    let bridge_config_account = next_account_info(account_info_iter)?;
    let token_program = next_account_info(account_info_iter)?;

    if !relayer_account.is_signer {
        msg!("Relayer must sign the unlock transaction");
        return Err(BridgeError::MissingRequiredSignature.into());
    }

    if bridge_config_account.owner != program_id {
        msg!("Bridge config has incorrect owner");
        return Err(BridgeError::IncorrectOwner.into());
    }

    let mut bridge_config = BridgeConfig::try_from_slice(&bridge_config_account.data.borrow())
        .map_err(|_| ProgramError::InvalidAccountData)?;

    if relayer_account.key != &bridge_config.relayer_authority {
        msg!(
            "Relayer is not authorized. Expected: {}, Got: {}",
            bridge_config.relayer_authority,
            relayer_account.key
        );
        return Err(BridgeError::Unauthorized.into());
    }

    if user_bridge_state_account.owner != program_id {
        msg!("User bridge state has incorrect owner");
        return Err(BridgeError::IncorrectOwner.into());
    }

    let mut user_bridge_state =
        UserBridgeState::try_from_slice(&user_bridge_state_account.data.borrow())
            .map_err(|_| ProgramError::InvalidAccountData)?;

    if user_bridge_state.nonce != nonce {
        msg!(
            "Nonce mismatch. Expected: {}, Got: {}",
            user_bridge_state.nonce,
            nonce
        );
        return Err(BridgeError::InvalidNonce.into());
    }

    if user_bridge_state.unlocked {
        msg!("Tokens have already been unlocked");
        return Err(BridgeError::AlreadyUnlocked.into());
    }

    if user_bridge_state.status != BridgeStatus::Pending {
        msg!("Invalid bridge status: {:?}", user_bridge_state.status);
        return Err(BridgeError::InvalidStatus.into());
    }

    if user_account.key != &user_bridge_state.user {
        msg!("User account mismatch");
        return Err(BridgeError::Unauthorized.into());
    }

    let (expected_vault_pda, vault_bump) =
        Pubkey::find_program_address(&[b"vault", bridge_config_account.key.as_ref()], program_id);

    if vault_pda_account.key != &expected_vault_pda {
        msg!("Invalid vault PDA");
        return Err(BridgeError::InvalidPDA.into());
    }

    if vault_bump != bridge_config.vault_pda_bump {
        msg!("Vault PDA bump mismatch");
        return Err(BridgeError::InvalidPDA.into());
    }

    if signatures.len() < bridge_config.validator_threshold as usize {
        msg!(
            "Insufficient signatures. Required: {}, Got: {}",
            bridge_config.validator_threshold,
            signatures.len()
        );
        return Err(BridgeError::ThresholdNotMet.into());
    }

    let message_data = create_unlock_message(
        nonce,
        user_account.key,
        user_bridge_state.locked_amount,
    );

    let mut valid_signature_count = 0;
    for (sig_idx, signature) in signatures.iter().enumerate() {
        for validator_pubkey in &bridge_config.validators {
            if verify_ed25519_signature(&message_data, signature, validator_pubkey.as_ref()) {
                msg!("Valid signature {} from validator {}", sig_idx, validator_pubkey);
                valid_signature_count += 1;
                break;
            }
        }
    }

    if valid_signature_count < bridge_config.validator_threshold as usize {
        msg!(
            "Signature verification failed. Valid: {}, Required: {}",
            valid_signature_count,
            bridge_config.validator_threshold
        );
        return Err(BridgeError::ThresholdNotMet.into());
    }

    msg!(
        "Signature verification passed: {}/{} valid signatures",
        valid_signature_count,
        signatures.len()
    );

    msg!(
        "Unlocking {} tokens to user",
        user_bridge_state.locked_amount
    );

    let transfer_instruction = spl_token::instruction::transfer(
        token_program.key,
        vault_token_account.key,
        user_token_account.key,
        vault_pda_account.key,
        &[],
        user_bridge_state.locked_amount,
    )?;

    let vault_seeds = &[
        b"vault",
        bridge_config_account.key.as_ref(),
        &[bridge_config.vault_pda_bump],
    ];

    invoke_signed(
        &transfer_instruction,
        &[
            vault_token_account.clone(),
            user_token_account.clone(),
            vault_pda_account.clone(),
            token_program.clone(),
        ],
        &[vault_seeds],
    )?;

    msg!("Token transfer successful");

    user_bridge_state.unlocked = true;
    user_bridge_state.status = BridgeStatus::Completed;

    user_bridge_state
        .serialize(&mut &mut user_bridge_state_account.data.borrow_mut()[..])
        .map_err(|_| ProgramError::InvalidAccountData)?;

    bridge_config.total_locked = bridge_config
        .total_locked
        .checked_sub(user_bridge_state.locked_amount)
        .ok_or(BridgeError::Overflow)?;

    bridge_config
        .serialize(&mut &mut bridge_config_account.data.borrow_mut()[..])
        .map_err(|_| ProgramError::InvalidAccountData)?;

    msg!("EVENT: TokensUnlocked");
    msg!("  user: {}", user_account.key);
    msg!("  amount: {}", user_bridge_state.locked_amount);
    msg!("  nonce: {}", nonce);

    Ok(())
}

fn process_update_config(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    new_admin: Option<Pubkey>,
    new_relayer: Option<Pubkey>,
    new_fee: Option<u16>,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let admin_account = next_account_info(account_info_iter)?;
    let bridge_config_account = next_account_info(account_info_iter)?;

    if !admin_account.is_signer {
        msg!("Admin must sign the update config transaction");
        return Err(BridgeError::MissingRequiredSignature.into());
    }

    if bridge_config_account.owner != program_id {
        msg!("Bridge config has incorrect owner");
        return Err(BridgeError::IncorrectOwner.into());
    }

    let mut bridge_config = BridgeConfig::try_from_slice(&bridge_config_account.data.borrow())
        .map_err(|_| ProgramError::InvalidAccountData)?;

    if admin_account.key != &bridge_config.admin {
        msg!(
            "Only current admin can update config. Expected: {}, Got: {}",
            bridge_config.admin,
            admin_account.key
        );
        return Err(BridgeError::Unauthorized.into());
    }

    if let Some(new_admin_key) = new_admin {
        msg!(
            "Updating admin from {} to {}",
            bridge_config.admin,
            new_admin_key
        );
        bridge_config.admin = new_admin_key;
    }

    if let Some(new_relayer_key) = new_relayer {
        msg!(
            "Updating relayer from {} to {}",
            bridge_config.relayer_authority,
            new_relayer_key
        );
        bridge_config.relayer_authority = new_relayer_key;
    }

    if let Some(new_fee_value) = new_fee {
        if new_fee_value > 10000 {
            msg!("Fee basis points must be <= 10000 (100%)");
            return Err(BridgeError::InvalidFee.into());
        }
        msg!(
            "Updating fee from {} to {} basis points",
            bridge_config.fee_basis_points,
            new_fee_value
        );
        bridge_config.fee_basis_points = new_fee_value;
    }

    bridge_config
        .serialize(&mut &mut bridge_config_account.data.borrow_mut()[..])
        .map_err(|_| ProgramError::InvalidAccountData)?;

    msg!("Bridge config updated successfully");

    Ok(())
}

fn process_pause(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let admin_account = next_account_info(account_info_iter)?;
    let bridge_config_account = next_account_info(account_info_iter)?;

    if !admin_account.is_signer {
        msg!("Admin must sign the pause transaction");
        return Err(BridgeError::MissingRequiredSignature.into());
    }

    if bridge_config_account.owner != program_id {
        msg!("Bridge config has incorrect owner");
        return Err(BridgeError::IncorrectOwner.into());
    }

    let mut bridge_config = BridgeConfig::try_from_slice(&bridge_config_account.data.borrow())
        .map_err(|_| ProgramError::InvalidAccountData)?;

    if admin_account.key != &bridge_config.admin {
        msg!("Only admin can pause the bridge");
        return Err(BridgeError::Unauthorized.into());
    }

    if bridge_config.is_paused {
        msg!("Bridge is already paused");
        return Ok(());
    }

    bridge_config.is_paused = true;

    bridge_config
        .serialize(&mut &mut bridge_config_account.data.borrow_mut()[..])
        .map_err(|_| ProgramError::InvalidAccountData)?;

    msg!("Bridge has been paused");
    msg!("Lock operations are now disabled");
    msg!("Unlock operations continue to work");

    Ok(())
}

fn process_unpause(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let admin_account = next_account_info(account_info_iter)?;
    let bridge_config_account = next_account_info(account_info_iter)?;

    if !admin_account.is_signer {
        msg!("Admin must sign the unpause transaction");
        return Err(BridgeError::MissingRequiredSignature.into());
    }

    if bridge_config_account.owner != program_id {
        msg!("Bridge config has incorrect owner");
        return Err(BridgeError::IncorrectOwner.into());
    }

    let mut bridge_config = BridgeConfig::try_from_slice(&bridge_config_account.data.borrow())
        .map_err(|_| ProgramError::InvalidAccountData)?;

    if admin_account.key != &bridge_config.admin {
        msg!("Only admin can unpause the bridge");
        return Err(BridgeError::Unauthorized.into());
    }

    if !bridge_config.is_paused {
        msg!("Bridge is already unpaused");
        return Ok(());
    }

    bridge_config.is_paused = false;

    bridge_config
        .serialize(&mut &mut bridge_config_account.data.borrow_mut()[..])
        .map_err(|_| ProgramError::InvalidAccountData)?;

    msg!("Bridge has been unpaused");
    msg!("Lock operations are now enabled");

    Ok(())
}

fn create_unlock_message(nonce: u64, user: &Pubkey, amount: u64) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(b"unlock:");
    hasher.update(nonce.to_le_bytes());
    hasher.update(user.as_ref());
    hasher.update(amount.to_le_bytes());
    let result = hasher.finalize();
    let mut message = [0u8; 32];
    message.copy_from_slice(&result);
    message
}

fn verify_ed25519_signature(message: &[u8; 32], signature: &[u8; 64], pubkey: &[u8]) -> bool {
    if pubkey.len() != 32 {
        return false;
    }

    let pubkey_bytes = match <[u8; 32]>::try_from(pubkey) {
        Ok(bytes) => bytes,
        Err(_) => return false,
    };

    use ed25519_dalek::{PublicKey, Signature, Verifier};

    let public_key = match PublicKey::from_bytes(&pubkey_bytes) {
        Ok(pk) => pk,
        Err(_) => return false,
    };

    let sig = match Signature::from_bytes(signature) {
        Ok(s) => s,
        Err(_) => return false,
    };

    public_key.verify(message, &sig).is_ok()
}
