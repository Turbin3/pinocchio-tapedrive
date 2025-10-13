#![cfg(test)]

use litesvm::LiteSVM;
use solana_sdk::{
    pubkey::Pubkey, signature::Keypair, signer::Signer, system_program, sysvar,
    transaction::Transaction,
};
use tape_api::{
    consts::{MINER, NAME_LEN, SPOOL},
    state::{Miner, Spool},
};

/// Helper to convert string to fixed-size name array
fn to_name(s: &str) -> [u8; NAME_LEN] {
    let mut name = [0u8; NAME_LEN];
    let bytes = s.as_bytes();
    let len = bytes.len().min(NAME_LEN);
    name[..len].copy_from_slice(&bytes[..len]);
    name
}

fn register_miner(
    svm: &mut LiteSVM,
    payer: &Keypair,
    program_id: Pubkey,
    miner_name: &str,
) -> Pubkey {
    let payer_pk = payer.pubkey();
    let name_bytes = to_name(miner_name);

    // Derive miner PDA
    let (miner_address, _miner_bump) =
        Pubkey::find_program_address(&[MINER, payer_pk.as_ref(), &name_bytes], &program_id);

    // Build register instruction
    let mut data = vec![0x20]; // Register discriminator
    data.extend_from_slice(&name_bytes);

    let accounts = vec![
        solana_sdk::instruction::AccountMeta::new(payer_pk, true),
        solana_sdk::instruction::AccountMeta::new(miner_address, false),
        solana_sdk::instruction::AccountMeta::new_readonly(sysvar::rent::ID, false),
        solana_sdk::instruction::AccountMeta::new_readonly(sysvar::slot_hashes::ID, false),
        solana_sdk::instruction::AccountMeta::new_readonly(system_program::ID, false),
    ];

    let ix = solana_sdk::instruction::Instruction {
        program_id,
        accounts,
        data,
    };

    let blockhash = svm.latest_blockhash();
    let tx = Transaction::new_signed_with_payer(&[ix], Some(&payer_pk), &[payer], blockhash);
    svm.send_transaction(tx).unwrap();

    miner_address
}

fn create_spool(
    svm: &mut LiteSVM,
    payer: &Keypair,
    program_id: Pubkey,
    miner_address: Pubkey,
    spool_number: u64,
) -> Pubkey {
    let payer_pk = payer.pubkey();

    // Derive spool PDA
    let spool_number_bytes = spool_number.to_le_bytes();
    let (spool_address, _spool_bump) = Pubkey::find_program_address(
        &[SPOOL, miner_address.as_ref(), &spool_number_bytes],
        &program_id,
    );

    // Build create spool instruction
    let mut data = vec![0x40]; // Create spool discriminator
    data.extend_from_slice(&spool_number_bytes);

    let accounts = vec![
        solana_sdk::instruction::AccountMeta::new(payer_pk, true),
        solana_sdk::instruction::AccountMeta::new(miner_address, false),
        solana_sdk::instruction::AccountMeta::new(spool_address, false),
        solana_sdk::instruction::AccountMeta::new_readonly(system_program::ID, false),
        solana_sdk::instruction::AccountMeta::new_readonly(sysvar::rent::ID, false),
    ];

    let ix = solana_sdk::instruction::Instruction {
        program_id,
        accounts,
        data,
    };

    let blockhash = svm.latest_blockhash();
    let tx = Transaction::new_signed_with_payer(&[ix], Some(&payer_pk), &[payer], blockhash);
    svm.send_transaction(tx).unwrap();

    spool_address
}

#[test]
fn test_pinocchio_spool_create_cu_measurement() {
    println!("\nPINOCCHIO SPOOL CREATE - CU MEASUREMENT TEST");

    // Setup SVM
    let mut svm = LiteSVM::new();

    // Load Pinocchio program
    let program_id: Pubkey = "7wApqqrfJo2dAGAKVgheccaVEgeDoqVKogtJSTbFRWn2"
        .parse()
        .expect("Invalid program ID");

    svm.add_program_from_file(program_id, "../target/deploy/pinnochio_tape_program.so")
        .expect("Failed to load Pinocchio tape program");

    // Create and fund payer
    let payer = Keypair::new();
    svm.airdrop(&payer.pubkey(), 10_000_000_000)
        .expect("Failed to airdrop to payer");

    let payer_pk = payer.pubkey();

    println!("Payer: {}", payer_pk);
    println!("Program ID: {}", program_id);

    // Step 1: Register miner
    let miner_address = register_miner(&mut svm, &payer, program_id, "test-miner");
    println!("Miner registered: {}", miner_address);

    // Verify miner
    let miner_account = svm.get_account(&miner_address).unwrap();
    let miner = Miner::unpack(&miner_account.data).unwrap();
    assert_eq!(miner.authority, payer_pk.to_bytes());

    // Step 2: Create spool
    let spool_number: u64 = 0;
    let spool_number_bytes = spool_number.to_le_bytes();
    let (spool_address, _) = Pubkey::find_program_address(
        &[SPOOL, miner_address.as_ref(), &spool_number_bytes],
        &program_id,
    );

    let mut data = vec![0x40]; // Create spool discriminator
    data.extend_from_slice(&spool_number_bytes);

    let accounts = vec![
        solana_sdk::instruction::AccountMeta::new(payer_pk, true),
        solana_sdk::instruction::AccountMeta::new(miner_address, false),
        solana_sdk::instruction::AccountMeta::new(spool_address, false),
        solana_sdk::instruction::AccountMeta::new_readonly(system_program::ID, false),
        solana_sdk::instruction::AccountMeta::new_readonly(sysvar::rent::ID, false),
    ];

    let ix = solana_sdk::instruction::Instruction {
        program_id,
        accounts,
        data,
    };

    let blockhash = svm.latest_blockhash();
    let tx = Transaction::new_signed_with_payer(&[ix], Some(&payer_pk), &[&payer], blockhash);
    let result = svm.send_transaction(tx);

    if let Ok(metadata) = result {
        println!(
            "\nCOMPUTE UNITS CONSUMED: {}",
            metadata.compute_units_consumed
        );

        // Verify spool account
        let spool_account = svm.get_account(&spool_address).unwrap();
        let spool = Spool::unpack(&spool_account.data).unwrap();

        println!("\nSpool Created:");
        println!("Number: {}", spool.number);
        println!("Authority: {:?}", spool.authority);
        println!("Total tapes: {}", spool.total_tapes);

        assert_eq!(spool.number, spool_number);
        assert_eq!(spool.authority, payer_pk.to_bytes());
        assert_eq!(spool.total_tapes, 0);
        assert_eq!(spool.last_proof_block, 0);
        assert_ne!(spool.last_proof_at, 0);

        println!(
            "\nTEST PASSED - CUs: {}",
            metadata.compute_units_consumed
        );
    } else {
        panic!("Spool create failed: {:?}", result.err());
    }
}

#[test]
fn test_pinocchio_spool_create_multiple_runs() {
    println!("\nPINOCCHIO SPOOL CREATE - MULTIPLE RUNS");

    // Setup SVM
    let mut svm = LiteSVM::new();

    // Load Pinocchio program
    let program_id: Pubkey = "7wApqqrfJo2dAGAKVgheccaVEgeDoqVKogtJSTbFRWn2"
        .parse()
        .expect("Invalid program ID");

    svm.add_program_from_file(program_id, "../target/deploy/pinnochio_tape_program.so")
        .expect("Failed to load Pinocchio tape program");

    // Create and fund payer
    let payer = Keypair::new();
    svm.airdrop(&payer.pubkey(), 10_000_000_000)
        .expect("Failed to airdrop to payer");

    let mut cus = Vec::new();
    let num_runs = 3;

    for i in 0..num_runs {
        // Register miner
        let miner_name = format!("miner-{}", i);
        let miner_address = register_miner(&mut svm, &payer, program_id, &miner_name);

        // Create spool
        let spool_number: u64 = 0;
        let spool_number_bytes = spool_number.to_le_bytes();
        let (spool_address, _) = Pubkey::find_program_address(
            &[SPOOL, miner_address.as_ref(), &spool_number_bytes],
            &program_id,
        );

        let mut data = vec![0x40]; // Create spool discriminator
        data.extend_from_slice(&spool_number_bytes);

        let payer_pk = payer.pubkey();
        let accounts = vec![
            solana_sdk::instruction::AccountMeta::new(payer_pk, true),
            solana_sdk::instruction::AccountMeta::new(miner_address, false),
            solana_sdk::instruction::AccountMeta::new(spool_address, false),
            solana_sdk::instruction::AccountMeta::new_readonly(system_program::ID, false),
            solana_sdk::instruction::AccountMeta::new_readonly(sysvar::rent::ID, false),
        ];

        let ix = solana_sdk::instruction::Instruction {
            program_id,
            accounts,
            data,
        };

        let blockhash = svm.latest_blockhash();
        let tx = Transaction::new_signed_with_payer(&[ix], Some(&payer_pk), &[&payer], blockhash);
        let result = svm.send_transaction(tx);

        assert!(result.is_ok(), "Run {} failed: {:?}", i, result.err());

        if let Ok(metadata) = result {
            cus.push(metadata.compute_units_consumed);
            println!("Run {}: {} CUs", i, metadata.compute_units_consumed);
        }
    }

    let total: u64 = cus.iter().sum();
    let avg = total / num_runs as u64;
    let min = *cus.iter().min().unwrap();
    let max = *cus.iter().max().unwrap();

    println!("\nPINOCCHIO SPOOL CREATE RESULTS:");
    println!("Runs: {}", num_runs);
    println!("Min CUs: {}", min);
    println!("Max CUs: {}", max);
    println!("Avg CUs: {}", avg);
    println!("Total CUs: {}", total);

    println!("\nPINOCCHIO SPOOL CREATE - MULTIPLE RUNS PASSED");
}
