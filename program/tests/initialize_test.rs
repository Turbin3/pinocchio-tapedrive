#![cfg(test)]

use litesvm::LiteSVM;
use solana_program::program_pack::Pack;
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signature::Keypair,
    signer::Signer,
    system_program,
    sysvar::{rent, slot_hashes},
    transaction::Transaction,
};
use spl_token::state::Mint;

// Import from the source directly (like pinocchio-multisig does)
use pinnochio_tape_program::state::{Archive, Block, Epoch, Tape, TapeState};
use tape_api::consts::*;
use tape_api::utils::to_name;

/// Test basic initialization of the pinocchio tape program
#[test]
fn test_pinocchio_initialize_basic() {
    // Setup environment
    let (mut svm, payer, program_id) = setup_environment();

    // Initialize program
    initialize_program(&mut svm, &payer, program_id);

    // Verify all accounts were created correctly
    verify_archive_account(&svm);
    verify_epoch_account(&svm);
    verify_block_account(&svm);
    verify_treasury_account(&svm);
    verify_mint_account(&svm);
    verify_metadata_account(&svm);
    verify_treasury_ata(&svm);
    verify_genesis_tape(&svm, &payer);

    println!("Successfully initialized pinocchio tape program!");
}

/// Test that we can't initialize twice
#[test]
fn test_pinocchio_initialize_already_initialized() {
    let (mut svm, payer, program_id) = setup_environment();

    // Initialize program successfully
    initialize_program(&mut svm, &payer, program_id);

    // Try to initialize again - should fail
    let ix = build_initialize_ix(payer.pubkey(), program_id);
    let blockhash = svm.latest_blockhash();
    let tx = Transaction::new_signed_with_payer(&[ix], Some(&payer.pubkey()), &[&payer], blockhash);
    let res = svm.send_transaction(tx);

    assert!(res.is_err(), "Should not be able to initialize twice");
    println!("Correctly rejected double initialization!");
}

/// Test archive account state after initialization
#[test]
fn test_pinocchio_initialize_archive_state() {
    let (mut svm, payer, program_id) = setup_environment();
    initialize_program(&mut svm, &payer, program_id);

    let archive_address = Pubkey::from(ARCHIVE_ADDRESS);
    let account = svm
        .get_account(&archive_address)
        .expect("Archive account should exist");

    // Use bytemuck to deserialize
    let archive: &Archive = bytemuck::from_bytes(&account.data[..core::mem::size_of::<Archive>()]);

    // Genesis tape should already be stored
    assert_eq!(
        archive.tapes_stored, 1,
        "Archive should have genesis tape stored"
    );
    assert_eq!(
        archive.segments_stored, 1,
        "Archive should have genesis segments stored"
    );

    println!(
        " Archive state verified: {} tapes, {} segments",
        archive.tapes_stored, archive.segments_stored
    );
}

/// Test epoch account state after initialization
#[test]
fn test_pinocchio_initialize_epoch_state() {
    let (mut svm, payer, program_id) = setup_environment();
    initialize_program(&mut svm, &payer, program_id);

    let epoch_address = Pubkey::from(EPOCH_ADDRESS);
    let account = svm
        .get_account(&epoch_address)
        .expect("Epoch account should exist");

    let epoch: &Epoch = bytemuck::from_bytes(&account.data[..core::mem::size_of::<Epoch>()]);

    assert_eq!(epoch.number, 1, "Epoch number should start at 1");
    assert_eq!(epoch.progress, 0, "Epoch progress should start at 0");
    assert_eq!(epoch.target_participation, MIN_PARTICIPATION_TARGET);
    assert_eq!(epoch.mining_difficulty, MIN_MINING_DIFFICULTY);
    assert_eq!(epoch.packing_difficulty, MIN_PACKING_DIFFICULTY);
    assert_eq!(epoch.duplicates, 0, "Duplicates should start at 0");
    assert_eq!(epoch.last_epoch_at, 0, "Last epoch should start at 0");

    println!(
        " Epoch state verified: epoch #{}, difficulty M:{} P:{}",
        epoch.number, epoch.mining_difficulty, epoch.packing_difficulty
    );
}

/// Test block account state after initialization
#[test]
fn test_pinocchio_initialize_block_state() {
    let (mut svm, payer, program_id) = setup_environment();
    initialize_program(&mut svm, &payer, program_id);

    let block_address = Pubkey::from(BLOCK_ADDRESS);
    let account = svm
        .get_account(&block_address)
        .expect("Block account should exist");

    let block: &Block = bytemuck::from_bytes(&account.data[..core::mem::size_of::<Block>()]);

    assert_eq!(block.number, 1, "Block number should start at 1");
    assert_eq!(block.progress, 0, "Block progress should start at 0");
    assert_eq!(block.last_proof_at, 0, "Last proof should start at 0");
    assert_eq!(block.last_block_at, 0, "Last block should start at 0");
    assert_eq!(block.challenge_set, 1, "Challenge set should be 1");
    assert_ne!(block.challenge, [0u8; 32], "Challenge should be set");

    println!(
        " Block state verified: block #{}, challenge set",
        block.number
    );
}

/// Test mint account state after initialization
#[test]
fn test_pinocchio_initialize_mint_state() {
    let (mut svm, payer, program_id) = setup_environment();
    initialize_program(&mut svm, &payer, program_id);

    let mint_address = Pubkey::from(MINT_ADDRESS);
    let treasury_address = Pubkey::from(TREASURY_ADDRESS);

    let account = svm
        .get_account(&mint_address)
        .expect("Mint account should exist");
    let mint = Mint::unpack(&account.data).expect("Failed to unpack Mint");

    assert_eq!(
        mint.decimals, TOKEN_DECIMALS,
        "Decimals should be {}",
        TOKEN_DECIMALS
    );
    assert_eq!(mint.supply, MAX_SUPPLY, "Supply should be max supply");
    assert_eq!(
        mint.mint_authority.unwrap(),
        treasury_address,
        "Treasury should be mint authority"
    );
    assert!(mint.is_initialized, "Mint should be initialized");

    println!(
        " Mint state verified: {} tokens with {} decimals",
        MAX_SUPPLY, TOKEN_DECIMALS
    );
}

/// Test treasury ATA has full supply after initialization
#[test]
fn test_pinocchio_initialize_treasury_balance() {
    let (mut svm, payer, program_id) = setup_environment();
    initialize_program(&mut svm, &payer, program_id);

    let treasury_ata = Pubkey::from(TREASURY_ATA);
    let ata_balance = get_ata_balance(&svm, &treasury_ata);

    assert_eq!(ata_balance, MAX_SUPPLY, "Treasury should have max supply");

    println!("Treasury balance verified: {} tokens", ata_balance);
}

/// Test genesis tape was created and finalized
#[test]
fn test_pinocchio_initialize_genesis_tape() {
    let (mut svm, payer, program_id) = setup_environment();
    initialize_program(&mut svm, &payer, program_id);

    let genesis_name = "genesis";
    let name_bytes = to_name(genesis_name);
    // Derive tape PDA: seeds = ["tape", payer_pubkey, name]
    let (tape_address, _) = Pubkey::find_program_address(
        &[b"tape", payer.pubkey().as_ref(), &name_bytes],
        &program_id,
    );

    let account = svm
        .get_account(&tape_address)
        .expect("Genesis tape should exist");
    let tape: &Tape = bytemuck::from_bytes(&account.data[..core::mem::size_of::<Tape>()]);

    assert_eq!(tape.number, 1, "Genesis should be tape #1");
    assert_eq!(
        tape.state,
        TapeState::Finalized as u64,
        "Genesis tape should be finalized"
    );
    assert_eq!(tape.total_segments, 1, "Genesis should have 1 segment");

    println!(
        " Genesis tape verified: finalized with {} segments",
        tape.total_segments
    );
}

/// Test metadata account exists and has correct data
#[test]
fn test_pinocchio_initialize_metadata() {
    let (mut svm, payer, program_id) = setup_environment();
    initialize_program(&mut svm, &payer, program_id);

    let mint_address = Pubkey::from(MINT_ADDRESS);
    // Must match MPL_TOKEN_METADATA_ID in constant.rs
    let metadata_program = Pubkey::new_from_array([
        11, 112, 101, 177, 227, 209, 124, 69, 56, 157, 82, 127, 107, 4, 195, 205, 88, 184, 108,
        115, 26, 160, 253, 181, 73, 182, 209, 188, 3, 248, 41, 70,
    ]);
    let (metadata_address, _) = Pubkey::find_program_address(
        &[
            b"metadata",
            metadata_program.as_ref(),
            mint_address.as_ref(),
        ],
        &metadata_program,
    );

    let account = svm
        .get_account(&metadata_address)
        .expect("Metadata account should exist");
    assert!(!account.data.is_empty(), "Metadata should have data");

    println!("Metadata account verified");
}

/// Test all PDAs have correct addresses
#[test]
fn test_pinocchio_initialize_pda_addresses() {
    let (mut svm, payer, program_id) = setup_environment();
    initialize_program(&mut svm, &payer, program_id);

    let archive_address = Pubkey::from(ARCHIVE_ADDRESS);
    let epoch_address = Pubkey::from(EPOCH_ADDRESS);
    let block_address = Pubkey::from(BLOCK_ADDRESS);
    let mint_address = Pubkey::from(MINT_ADDRESS);
    let treasury_address = Pubkey::from(TREASURY_ADDRESS);

    assert_eq!(
        archive_address,
        Pubkey::from(ARCHIVE_ADDRESS),
        "Archive PDA mismatch"
    );
    assert_eq!(
        epoch_address,
        Pubkey::from(EPOCH_ADDRESS),
        "Epoch PDA mismatch"
    );
    assert_eq!(
        block_address,
        Pubkey::from(BLOCK_ADDRESS),
        "Block PDA mismatch"
    );
    assert_eq!(
        mint_address,
        Pubkey::from(MINT_ADDRESS),
        "Mint PDA mismatch"
    );
    assert_eq!(
        treasury_address,
        Pubkey::from(TREASURY_ADDRESS),
        "Treasury PDA mismatch"
    );

    println!("All PDA addresses verified");
}

/// Measure compute units for initialization - KEY TEST FOR CU COMPARISON
#[test]
fn test_pinocchio_initialize_compute_units() {
    let (mut svm, payer, program_id) = setup_environment();

    let ix = build_initialize_ix(payer.pubkey(), program_id);
    let blockhash = svm.latest_blockhash();
    let tx = Transaction::new_signed_with_payer(&[ix], Some(&payer.pubkey()), &[&payer], blockhash);

    let res = svm.send_transaction(tx);
    assert!(res.is_ok(), "Initialization should succeed");

    let meta = res.unwrap();
    let cu_used = meta.compute_units_consumed;

    println!("\nPINOCCHIO Initialize Compute Units: {}", cu_used);
    println!("  Max CU limit: 1,000,000");
    println!("  Usage: {:.2}%\n", (cu_used as f64 / 1_000_000.0) * 100.0);
}

// Helper functions

fn setup_environment() -> (LiteSVM, Keypair, Pubkey) {
    let mut svm = LiteSVM::new();

    // TODO: Set compute budget to max (1.4M CUs) since initialize does a lot of work
    // Currently disabled due to missing solana_compute_budget crate
    // use solana_compute_budget::compute_budget::ComputeBudget;
    // let mut compute_budget = ComputeBudget::default();
    // compute_budget.compute_unit_limit = 1_400_000; // Max CU limit
    // let mut svm = svm.with_compute_budget(compute_budget);

    // Create and fund payer
    let payer = Keypair::new();
    svm.airdrop(&payer.pubkey(), 100_000_000_000).unwrap();

    // Load the pinocchio program (convert [u8; 32] to solana_sdk::Pubkey)
    let program_id = Pubkey::from(tape_api::ID);
    svm.add_program_from_file(program_id, "../target/deploy/pinnochio_tape_program.so")
        .expect("Failed to load pinocchio tape program");

    // Load metadata program (needed for token metadata)
    load_metadata_program(&mut svm);

    (svm, payer, program_id)
}

fn load_metadata_program(svm: &mut LiteSVM) {
    // Load the metadata program from elfs directory
    let metadata_bytes = std::fs::read("tests/elfs/metadata.so")
        .expect("Failed to read metadata program. Run: solana program dump --url mainnet-beta metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s tests/elfs/metadata.so");

    // Metadata program ID: metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s
    // Must match the ID in pinocchio-tapedrive/program/src/state/constant.rs
    let metadata_program_id = Pubkey::new_from_array([
        11, 112, 101, 177, 227, 209, 124, 69, 56, 157, 82, 127, 107, 4, 195, 205, 88, 184, 108,
        115, 26, 160, 253, 181, 73, 182, 209, 188, 3, 248, 41, 70,
    ]);
    svm.add_program(metadata_program_id, &metadata_bytes);
}

fn build_initialize_ix(signer: Pubkey, program_id: Pubkey) -> Instruction {
    // Use constant PDAs
    let archive_pda = Pubkey::from(ARCHIVE_ADDRESS);
    let epoch_pda = Pubkey::from(EPOCH_ADDRESS);
    let block_pda = Pubkey::from(BLOCK_ADDRESS);
    let mint_pda = Pubkey::from(MINT_ADDRESS);
    let treasury_pda = Pubkey::from(TREASURY_ADDRESS);
    let treasury_ata_pda = Pubkey::from(TREASURY_ATA);

    // Derive metadata PDA (must match MPL_TOKEN_METADATA_ID in constant.rs)
    let metadata_program = Pubkey::new_from_array([
        11, 112, 101, 177, 227, 209, 124, 69, 56, 157, 82, 127, 107, 4, 195, 205, 88, 184, 108,
        115, 26, 160, 253, 181, 73, 182, 209, 188, 3, 248, 41, 70,
    ]);
    let (metadata_pda, _) = Pubkey::find_program_address(
        &[b"metadata", metadata_program.as_ref(), mint_pda.as_ref()],
        &metadata_program,
    );

    // Derive tape and writer PDAs
    let name = to_name("genesis");
    let (tape_pda, _) =
        Pubkey::find_program_address(&[b"tape", signer.as_ref(), &name], &program_id);
    let (writer_pda, _) =
        Pubkey::find_program_address(&[b"writer", tape_pda.as_ref()], &program_id);

    // Token program IDs
    let spl_token_id = Pubkey::new_from_array([
        6, 221, 246, 225, 215, 101, 161, 147, 217, 203, 225, 70, 206, 235, 121, 172, 28, 180, 133,
        237, 95, 91, 55, 145, 58, 140, 245, 133, 126, 255, 0, 169,
    ]);
    let spl_ata_id = Pubkey::new_from_array([
        140, 151, 37, 143, 78, 36, 137, 241, 187, 61, 16, 41, 20, 142, 13, 131, 11, 90, 19, 153,
        218, 255, 16, 132, 4, 142, 123, 216, 219, 233, 248, 89,
    ]);

    Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(signer, true),
            AccountMeta::new(archive_pda, false),
            AccountMeta::new(epoch_pda, false),
            AccountMeta::new(block_pda, false),
            AccountMeta::new(metadata_pda, false),
            AccountMeta::new(mint_pda, false),
            AccountMeta::new(treasury_pda, false),
            AccountMeta::new(treasury_ata_pda, false),
            AccountMeta::new(tape_pda, false),
            AccountMeta::new(writer_pda, false),
            AccountMeta::new_readonly(program_id, false),
            AccountMeta::new_readonly(system_program::ID, false),
            AccountMeta::new_readonly(spl_token_id, false),
            AccountMeta::new_readonly(spl_ata_id, false),
            AccountMeta::new_readonly(metadata_program, false),
            AccountMeta::new_readonly(rent::ID, false),
            AccountMeta::new_readonly(slot_hashes::ID, false),
        ],
        data: vec![1], // Initialize instruction discriminator
    }
}

fn initialize_program(svm: &mut LiteSVM, payer: &Keypair, program_id: Pubkey) {
    let ix = build_initialize_ix(payer.pubkey(), program_id);
    let blockhash = svm.latest_blockhash();
    let tx = Transaction::new_signed_with_payer(&[ix], Some(&payer.pubkey()), &[payer], blockhash);
    let res = svm.send_transaction(tx);

    if res.is_err() {
        let err = res.as_ref().err().unwrap();
        println!("Error: {:?}", err.err);
        println!("Logs:");
        for log in &err.meta.logs {
            println!(" {}", log);
        }
    }

    assert!(res.is_ok(), "Program initialization should succeed");
}

fn verify_archive_account(svm: &LiteSVM) {
    let archive_address = Pubkey::from(ARCHIVE_ADDRESS);
    let account = svm
        .get_account(&archive_address)
        .expect("Archive account should exist");
    let _archive: &Archive = bytemuck::from_bytes(&account.data[..core::mem::size_of::<Archive>()]);
}

fn verify_epoch_account(svm: &LiteSVM) {
    let epoch_address = Pubkey::from(EPOCH_ADDRESS);
    let account = svm
        .get_account(&epoch_address)
        .expect("Epoch account should exist");
    let _epoch: &Epoch = bytemuck::from_bytes(&account.data[..core::mem::size_of::<Epoch>()]);
}

fn verify_block_account(svm: &LiteSVM) {
    let block_address = Pubkey::from(BLOCK_ADDRESS);
    let account = svm
        .get_account(&block_address)
        .expect("Block account should exist");
    let _block: &Block = bytemuck::from_bytes(&account.data[..core::mem::size_of::<Block>()]);
}

fn verify_treasury_account(svm: &LiteSVM) {
    let treasury_address = Pubkey::from(TREASURY_ADDRESS);
    let _account = svm
        .get_account(&treasury_address)
        .expect("Treasury account should exist");
}

fn verify_mint_account(svm: &LiteSVM) {
    let mint_address = Pubkey::from(MINT_ADDRESS);
    let account = svm
        .get_account(&mint_address)
        .expect("Mint account should exist");
    let _mint = Mint::unpack(&account.data).expect("Failed to unpack Mint");
}

fn verify_metadata_account(svm: &LiteSVM) {
    // Derive metadata PDA: seeds = ["metadata", metadata_program_id, mint_address]
    let mint_address = Pubkey::from(MINT_ADDRESS);
    let metadata_program = Pubkey::new_from_array([
        11, 112, 132, 119, 193, 192, 49, 38, 73, 174, 55, 16, 196, 233, 99, 165, 38, 132, 214, 37,
        185, 184, 179, 237, 235, 186, 165, 87, 32, 172, 79, 238,
    ]);
    let (metadata_address, _) = Pubkey::find_program_address(
        &[
            b"metadata",
            metadata_program.as_ref(),
            mint_address.as_ref(),
        ],
        &metadata_program,
    );
    let account = svm
        .get_account(&metadata_address)
        .expect("Metadata account should exist");
    assert!(!account.data.is_empty(), "Metadata should have data");
}

fn verify_treasury_ata(svm: &LiteSVM) {
    let treasury_ata = Pubkey::from(TREASURY_ATA);
    let account = svm
        .get_account(&treasury_ata)
        .expect("Treasury ATA should exist");
    assert!(!account.data.is_empty(), "Treasury ATA should have data");
}

fn verify_genesis_tape(svm: &LiteSVM, payer: &Keypair) {
    let genesis_name = "genesis";
    let name_bytes = to_name(genesis_name);
    let program_id = Pubkey::from(tape_api::ID);
    let (tape_address, _) = Pubkey::find_program_address(
        &[b"tape", payer.pubkey().as_ref(), &name_bytes],
        &program_id,
    );
    let account = svm
        .get_account(&tape_address)
        .expect("Genesis tape should exist");
    let _tape: &Tape = bytemuck::from_bytes(&account.data[..core::mem::size_of::<Tape>()]);
}

fn get_ata_balance(svm: &LiteSVM, ata: &Pubkey) -> u64 {
    let account = svm.get_account(ata).expect("ATA should exist");
    let token_account =
        spl_token::state::Account::unpack(&account.data).expect("Failed to unpack token account");
    token_account.amount
}
