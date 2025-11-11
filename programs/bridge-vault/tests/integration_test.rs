use bridge_vault::{
    instruction::BridgeInstruction,
    state::{BridgeConfig, BridgeStatus, UserBridgeState},
    BridgeError,
};
use solana_program::{
    instruction::Instruction,
    pubkey::Pubkey,
    system_instruction,
};
use solana_program_test::*;
use solana_sdk::{
    signature::{Keypair, Signer},
    transaction::Transaction,
    transport::TransportError,
};
use spl_token;

#[tokio::test]
async fn test_initialize() {
    let program_id = Pubkey::new_unique();
    let mut program_test = ProgramTest::new(
        "bridge_vault",
        program_id,
        processor!(bridge_vault::process_instruction),
    );

    let (mut banks_client, payer, recent_blockhash) = program_test.start().await;

    let admin = Keypair::new();
    let relayer = Keypair::new();
    let bridge_config = Keypair::new();

    let validators = vec![
        Keypair::new().pubkey(),
        Keypair::new().pubkey(),
        Keypair::new().pubkey(),
    ];

    let (vault_pda, _bump) = Pubkey::find_program_address(
        &[b"vault", bridge_config.pubkey().as_ref()],
        &program_id,
    );

    let ix = BridgeInstruction::create_initialize_instruction(
        &program_id,
        &admin.pubkey(),
        &bridge_config.pubkey(),
        &vault_pda,
        &relayer.pubkey(),
        50,
        validators.clone(),
        2,
    );

    let mut transaction = Transaction::new_with_payer(&[ix], Some(&payer.pubkey()));
    transaction.sign(&[&payer, &admin, &bridge_config], recent_blockhash);

    banks_client.process_transaction(transaction).await.unwrap();

    let account = banks_client
        .get_account(bridge_config.pubkey())
        .await
        .expect("Failed to get bridge config account")
        .expect("Bridge config account not found");

    let config = BridgeConfig::try_from_slice(&account.data).unwrap();
    assert_eq!(config.admin, admin.pubkey());
    assert_eq!(config.relayer_authority, relayer.pubkey());
    assert_eq!(config.fee_basis_points, 50);
    assert_eq!(config.validators.len(), 3);
    assert_eq!(config.validator_threshold, 2);
    assert!(!config.is_paused);
    assert_eq!(config.nonce, 0);
}

#[tokio::test]
async fn test_pause_and_unpause() {
    let program_id = Pubkey::new_unique();
    let mut program_test = ProgramTest::new(
        "bridge_vault",
        program_id,
        processor!(bridge_vault::process_instruction),
    );

    let (mut banks_client, payer, recent_blockhash) = program_test.start().await;

    let admin = Keypair::new();
    let relayer = Keypair::new();
    let bridge_config = Keypair::new();
    let validators = vec![Keypair::new().pubkey()];

    let (vault_pda, _bump) = Pubkey::find_program_address(
        &[b"vault", bridge_config.pubkey().as_ref()],
        &program_id,
    );

    let init_ix = BridgeInstruction::create_initialize_instruction(
        &program_id,
        &admin.pubkey(),
        &bridge_config.pubkey(),
        &vault_pda,
        &relayer.pubkey(),
        50,
        validators,
        1,
    );

    let mut init_tx = Transaction::new_with_payer(&[init_ix], Some(&payer.pubkey()));
    init_tx.sign(&[&payer, &admin, &bridge_config], recent_blockhash);
    banks_client.process_transaction(init_tx).await.unwrap();

    let pause_ix = BridgeInstruction::create_pause_instruction(
        &program_id,
        &admin.pubkey(),
        &bridge_config.pubkey(),
    );

    let mut pause_tx = Transaction::new_with_payer(&[pause_ix], Some(&payer.pubkey()));
    pause_tx.sign(&[&payer, &admin], recent_blockhash);
    banks_client.process_transaction(pause_tx).await.unwrap();

    let account = banks_client
        .get_account(bridge_config.pubkey())
        .await
        .unwrap()
        .unwrap();

    let config = BridgeConfig::try_from_slice(&account.data).unwrap();
    assert!(config.is_paused);

    let unpause_ix = BridgeInstruction::create_unpause_instruction(
        &program_id,
        &admin.pubkey(),
        &bridge_config.pubkey(),
    );

    let mut unpause_tx = Transaction::new_with_payer(&[unpause_ix], Some(&payer.pubkey()));
    unpause_tx.sign(&[&payer, &admin], recent_blockhash);
    banks_client.process_transaction(unpause_tx).await.unwrap();

    let account = banks_client
        .get_account(bridge_config.pubkey())
        .await
        .unwrap()
        .unwrap();

    let config = BridgeConfig::try_from_slice(&account.data).unwrap();
    assert!(!config.is_paused);
}

#[tokio::test]
async fn test_update_config() {
    let program_id = Pubkey::new_unique();
    let mut program_test = ProgramTest::new(
        "bridge_vault",
        program_id,
        processor!(bridge_vault::process_instruction),
    );

    let (mut banks_client, payer, recent_blockhash) = program_test.start().await;

    let admin = Keypair::new();
    let new_admin = Keypair::new();
    let relayer = Keypair::new();
    let new_relayer = Keypair::new();
    let bridge_config = Keypair::new();
    let validators = vec![Keypair::new().pubkey()];

    let (vault_pda, _bump) = Pubkey::find_program_address(
        &[b"vault", bridge_config.pubkey().as_ref()],
        &program_id,
    );

    let init_ix = BridgeInstruction::create_initialize_instruction(
        &program_id,
        &admin.pubkey(),
        &bridge_config.pubkey(),
        &vault_pda,
        &relayer.pubkey(),
        50,
        validators,
        1,
    );

    let mut init_tx = Transaction::new_with_payer(&[init_ix], Some(&payer.pubkey()));
    init_tx.sign(&[&payer, &admin, &bridge_config], recent_blockhash);
    banks_client.process_transaction(init_tx).await.unwrap();

    let update_ix = BridgeInstruction::create_update_config_instruction(
        &program_id,
        &admin.pubkey(),
        &bridge_config.pubkey(),
        Some(new_admin.pubkey()),
        Some(new_relayer.pubkey()),
        Some(100),
    );

    let mut update_tx = Transaction::new_with_payer(&[update_ix], Some(&payer.pubkey()));
    update_tx.sign(&[&payer, &admin], recent_blockhash);
    banks_client.process_transaction(update_tx).await.unwrap();

    let account = banks_client
        .get_account(bridge_config.pubkey())
        .await
        .unwrap()
        .unwrap();

    let config = BridgeConfig::try_from_slice(&account.data).unwrap();
    assert_eq!(config.admin, new_admin.pubkey());
    assert_eq!(config.relayer_authority, new_relayer.pubkey());
    assert_eq!(config.fee_basis_points, 100);
}

#[tokio::test]
async fn test_invalid_fee_initialization() {
    let program_id = Pubkey::new_unique();
    let mut program_test = ProgramTest::new(
        "bridge_vault",
        program_id,
        processor!(bridge_vault::process_instruction),
    );

    let (mut banks_client, payer, recent_blockhash) = program_test.start().await;

    let admin = Keypair::new();
    let relayer = Keypair::new();
    let bridge_config = Keypair::new();
    let validators = vec![Keypair::new().pubkey()];

    let (vault_pda, _bump) = Pubkey::find_program_address(
        &[b"vault", bridge_config.pubkey().as_ref()],
        &program_id,
    );

    let ix = BridgeInstruction::create_initialize_instruction(
        &program_id,
        &admin.pubkey(),
        &bridge_config.pubkey(),
        &vault_pda,
        &relayer.pubkey(),
        10001,
        validators,
        1,
    );

    let mut transaction = Transaction::new_with_payer(&[ix], Some(&payer.pubkey()));
    transaction.sign(&[&payer, &admin, &bridge_config], recent_blockhash);

    let result = banks_client.process_transaction(transaction).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_invalid_validator_threshold() {
    let program_id = Pubkey::new_unique();
    let mut program_test = ProgramTest::new(
        "bridge_vault",
        program_id,
        processor!(bridge_vault::process_instruction),
    );

    let (mut banks_client, payer, recent_blockhash) = program_test.start().await;

    let admin = Keypair::new();
    let relayer = Keypair::new();
    let bridge_config = Keypair::new();
    let validators = vec![Keypair::new().pubkey(), Keypair::new().pubkey()];

    let (vault_pda, _bump) = Pubkey::find_program_address(
        &[b"vault", bridge_config.pubkey().as_ref()],
        &program_id,
    );

    let ix = BridgeInstruction::create_initialize_instruction(
        &program_id,
        &admin.pubkey(),
        &bridge_config.pubkey(),
        &vault_pda,
        &relayer.pubkey(),
        50,
        validators,
        3,
    );

    let mut transaction = Transaction::new_with_payer(&[ix], Some(&payer.pubkey()));
    transaction.sign(&[&payer, &admin, &bridge_config], recent_blockhash);

    let result = banks_client.process_transaction(transaction).await;
    assert!(result.is_err());
}
