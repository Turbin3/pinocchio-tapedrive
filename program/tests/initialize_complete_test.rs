#![cfg(test)]

use litesvm::{types::TransactionMetadata, LiteSVM};
use solana_program::program_pack::Pack;
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    message::{v0, VersionedMessage},
    native_token::LAMPORTS_PER_SOL,
    pubkey::Pubkey as SolanaPubkey,
    signature::Keypair,
    signer::Signer,
    system_program,
    sysvar::rent,
    transaction::VersionedTransaction,
};
use spl_token::state::Mint;

use tape_api::consts::*;
use tape_api::utils::to_name;

// Program IDs
fn program_id() -> SolanaPubkey {
    SolanaPubkey::from(tape_api::ID)
}

fn spl_token_id() -> SolanaPubkey {
    SolanaPubkey::from(spl_token::ID.to_bytes())
}

fn spl_ata_id() -> SolanaPubkey {
    // ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL
    SolanaPubkey::new_from_array([
        140, 151, 37, 143, 78, 36, 137, 241, 187, 61, 16, 41, 20, 142, 13, 131, 11, 90, 19, 153,
        218, 255, 16, 132, 4, 142, 123, 216, 219, 233, 248, 89,
    ])
}

fn mpl_metadata_id() -> SolanaPubkey {
    // Must match MPL_TOKEN_METADATA_ID constant in program/src/state/constant.rs
    SolanaPubkey::new_from_array([
        11, 112, 101, 177, 227, 209, 124, 69, 56, 157, 82, 127, 107, 4, 195, 205, 88, 184, 108,
        115, 26, 160, 253, 181, 73, 182, 209, 188, 3, 248, 41, 70,
    ])
}

/// Complete test that runs through the ENTIRE initialize instruction
#[test]
fn test_pinocchio_initialize_complete() {
    println!("\nStarting COMPLETE Pinocchio Initialize Test\n");

    // Setup LiteSVM with all required programs
    let mut svm = setup_litesvm();

    // Create payer
    let payer = Keypair::new();
    let payer_pubkey = payer.pubkey();

    // Airdrop SOL to payer
    svm.airdrop(&payer_pubkey, 100 * LAMPORTS_PER_SOL)
        .expect("Airdrop failed");

    println!("Payer funded: {}", payer_pubkey);

    // Derive all PDAs
    let archive_pda = SolanaPubkey::from(ARCHIVE_ADDRESS);
    let epoch_pda = SolanaPubkey::from(EPOCH_ADDRESS);
    let block_pda = SolanaPubkey::from(BLOCK_ADDRESS);
    let mint_pda = SolanaPubkey::from(MINT_ADDRESS);
    let treasury_pda = SolanaPubkey::from(TREASURY_ADDRESS);
    let treasury_ata_pda = SolanaPubkey::from(TREASURY_ATA);

    let metadata_program = mpl_metadata_id();
    let metadata_pda = {
        let seeds = &[b"metadata", metadata_program.as_ref(), mint_pda.as_ref()];
        let (pda, _) = SolanaPubkey::find_program_address(seeds, &metadata_program);
        pda
    };

    let prog_id = program_id();
    let tape_pda = {
        let name = to_name("genesis");
        let seeds = &[b"tape", payer_pubkey.as_ref(), &name];
        let (pda, _) = SolanaPubkey::find_program_address(seeds, &prog_id);
        pda
    };

    let writer_pda = {
        let seeds = &[b"writer", tape_pda.as_ref()];
        let (pda, _) = SolanaPubkey::find_program_address(seeds, &prog_id);
        pda
    };

    println!("PDAs derived:");
    println!("  Archive:      {}", archive_pda);
    println!("  Epoch:        {}", epoch_pda);
    println!("  Block:        {}", block_pda);
    println!("  Mint:         {}", mint_pda);
    println!("  Treasury:     {}", treasury_pda);
    println!("  Treasury ATA: {}", treasury_ata_pda);
    println!("  Metadata:     {}", metadata_pda);
    println!("  Tape:         {}", tape_pda);
    println!("  Writer:       {}", writer_pda);

    // Build initialize instruction manually (discriminator = 1 for Initialize)
    let instruction = Instruction {
        program_id: prog_id,
        accounts: vec![
            AccountMeta::new(payer_pubkey, true),                 // signer
            AccountMeta::new(archive_pda, false),                 // archive
            AccountMeta::new(epoch_pda, false),                   // epoch
            AccountMeta::new(block_pda, false),                   // block
            AccountMeta::new(metadata_pda, false),                // metadata
            AccountMeta::new(mint_pda, false),                    // mint
            AccountMeta::new(treasury_pda, false),                // treasury
            AccountMeta::new(treasury_ata_pda, false),            // treasury_ata
            AccountMeta::new(tape_pda, false),                    // tape
            AccountMeta::new(writer_pda, false),                  // writer
            AccountMeta::new_readonly(prog_id, false),            // tape_program
            AccountMeta::new_readonly(system_program::ID, false), // system_program
            AccountMeta::new_readonly(spl_token_id(), false),     // token_program
            AccountMeta::new_readonly(spl_ata_id(), false),       // ata_program
            AccountMeta::new_readonly(mpl_metadata_id(), false),  // metadata_program
            AccountMeta::new_readonly(rent::ID, false),           // rent_sysvar
            AccountMeta::new_readonly(solana_sdk::sysvar::slot_hashes::ID, false), // slot_hashes
        ],
        data: vec![1], // Initialize instruction (TapeInstruction::Initialize = 1)
    };

    println!("\n📦 Building and sending transaction...");

    // Build and send transaction
    let msg = v0::Message::try_compile(&payer_pubkey, &[instruction], &[], svm.latest_blockhash())
        .expect("Failed to compile message");

    let tx = VersionedTransaction::try_new(VersionedMessage::V0(msg), &[&payer])
        .expect("Failed to create transaction");

    // Send transaction
    let result: Result<TransactionMetadata, _> = svm.send_transaction(tx);

    match result {
        Ok(tx_metadata) => {
            println!("\nTransaction succeeded!");
            println!("  CUs consumed: {}", tx_metadata.compute_units_consumed);
            println!("  Max CU limit: 1,400,000");
            println!(
                "Usage: {:.2}%",
                (tx_metadata.compute_units_consumed as f64 / 1_400_000.0) * 100.0
            );

            // Verify all accounts were created properly
            println!("\nVerifying account states...");

            // Verify Archive
            let archive = svm
                .get_account(&archive_pda)
                .expect("Archive account should exist");
            assert!(!archive.data.is_empty(), "Archive should have data");
            println!("Archive account created");

            // Verify Epoch
            let epoch = svm
                .get_account(&epoch_pda)
                .expect("Epoch account should exist");
            assert!(!epoch.data.is_empty(), "Epoch should have data");
            println!("Epoch account created");

            // Verify Block
            let block = svm
                .get_account(&block_pda)
                .expect("Block account should exist");
            assert!(!block.data.is_empty(), "Block should have data");
            println!("Block account created");

            // Verify Mint
            let mint_account = svm.get_account(&mint_pda).expect("Mint should exist");
            let mint = Mint::unpack(&mint_account.data).expect("Failed to unpack Mint");
            assert_eq!(mint.decimals, TOKEN_DECIMALS, "Decimals should match");
            assert_eq!(mint.supply, MAX_SUPPLY, "Supply should be max");
            assert!(mint.is_initialized, "Mint should be initialized");
            println!(
                " Mint created: {} tokens with {} decimals",
                MAX_SUPPLY, TOKEN_DECIMALS
            );

            // Verify Treasury
            let treasury = svm
                .get_account(&treasury_pda)
                .expect("Treasury should exist");
            assert!(!treasury.data.is_empty(), "Treasury should have data");
            println!("Treasury account created");

            // Verify Treasury ATA
            let treasury_ata = svm
                .get_account(&treasury_ata_pda)
                .expect("Treasury ATA should exist");
            assert!(
                !treasury_ata.data.is_empty(),
                "Treasury ATA should have data"
            );
            println!("Treasury ATA created");

            // Verify Tape
            let tape_account = svm.get_account(&tape_pda).expect("Tape should exist");
            assert!(!tape_account.data.is_empty(), "Tape should have data");
            println!("Genesis tape created");

            // Verify Writer
            let writer = svm.get_account(&writer_pda).expect("Writer should exist");
            assert!(!writer.data.is_empty(), "Writer should have data");
            println!("Writer account created");

            // Verify Metadata
            let metadata = svm
                .get_account(&metadata_pda)
                .expect("Metadata should exist");
            assert!(!metadata.data.is_empty(), "Metadata should have data");
            println!("Metadata account created");

            println!("\nALL VERIFICATIONS PASSED!");
            println!("\nFINAL RESULT:");
            println!(
                "Pinocchio Initialize CUs: {}",
                tx_metadata.compute_units_consumed
            );
            println!("  Status: COMPLETE ");
        }
        Err(e) => {
            println!("\nTransaction failed: {:?}", e);
            panic!("Initialize instruction failed");
        }
    }
}

// Helper to setup LiteSVM with all required programs
fn setup_litesvm() -> LiteSVM {
    let mut svm = LiteSVM::new();

    println!("🔧 Setting up LiteSVM with all programs...");

    // Debug: Check if program IDs match
    let tape_api_id: SolanaPubkey = tape_api::ID.into();
    println!("  tape_api::ID: {}", tape_api_id);

    // Load main program using tape_api::ID (convert to solana_sdk::Pubkey)
    let program_bytes = std::fs::read(
        std::env::current_dir()
            .unwrap()
            .join("../target/deploy/pinnochio_tape_program.so"),
    )
    .expect("Failed to read program binary");

    svm.add_program(tape_api::ID.into(), &program_bytes);
    println!("Loaded Pinocchio Tape program");

    // Load Metaplex Metadata program
    let metadata_bytes = std::fs::read(
        std::env::current_dir()
            .unwrap()
            .join("tests/elfs/metadata.so"),
    )
    .expect("Failed to read metadata program");

    let metadata_id = mpl_metadata_id();
    svm.add_program(metadata_id, &metadata_bytes);
    println!("Loaded Metadata program");

    // Note: LiteSVM 0.6.1 has built-in support for SPL Token and ATA programs
    println!("Using LiteSVM built-in SPL Token and ATA support");

    println!("LiteSVM setup complete\n");

    svm
}
