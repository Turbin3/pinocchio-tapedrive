#![cfg(test)]

use litesvm::LiteSVM;
use solana_sdk::{
    pubkey::Pubkey, signature::Keypair, signer::Signer, system_program, sysvar,
    transaction::Transaction,
};
use tape_api::{
    consts::{MINER, NAME_LEN},
    state::Miner,
};

/// Helper to convert string to fixed-size name array
fn to_name(s: &str) -> [u8; NAME_LEN] {
    let mut name = [0u8; NAME_LEN];
    let bytes = s.as_bytes();
    let len = bytes.len().min(NAME_LEN);
    name[..len].copy_from_slice(&bytes[..len]);
    name
}

#[test]
fn test_pinocchio_miner_register_cu_measurement() {
    println!("\nPINOCCHIO MINER REGISTER - CU MEASUREMENT TEST");

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

    // Step 1: Register a miner
    let miner_name = "test-miner";
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
        solana_sdk::instruction::AccountMeta::new_readonly(system_program::ID, false),
        solana_sdk::instruction::AccountMeta::new_readonly(sysvar::rent::ID, false),
        solana_sdk::instruction::AccountMeta::new_readonly(sysvar::slot_hashes::ID, false),
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

        // Verify miner account
        let miner_account = svm.get_account(&miner_address).unwrap();
        let miner = Miner::unpack(&miner_account.data).unwrap();

        println!("\nMiner Registered:");
        println!("Authority: {:?}", &miner.authority[..8]);
        println!(
            " Name: {:?}",
            core::str::from_utf8(&miner.name).unwrap_or("invalid utf8")
        );
        println!("Challenge: {:?}", &miner.challenge[..8]);
        println!("Last proof at: {}", miner.last_proof_at);

        assert_eq!(miner.authority.as_ref(), payer_pk.as_ref());
        assert_eq!(miner.name, name_bytes);
        assert_eq!(miner.multiplier, 0);
        assert!(
            miner.last_proof_at > 0,
            "last_proof_at should be set to current time"
        );
        assert_eq!(miner.total_proofs, 0);
        assert_eq!(miner.total_rewards, 0);
        assert_eq!(miner.unclaimed_rewards, 0);

        println!(
            "\nTEST PASSED - CUs: {}",
            metadata.compute_units_consumed
        );
    } else {
        panic!("Register failed: {:?}", result.err());
    }
}

#[test]
fn test_pinocchio_miner_register_multiple_runs() {
    println!("\nPINOCCHIO MINER REGISTER - MULTIPLE RUNS");

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

    let mut cus = Vec::new();
    let num_runs = 3;

    for i in 0..num_runs {
        let miner_name = format!("miner-{}", i);
        let name_bytes = to_name(&miner_name);

        let (miner_address, _miner_bump) =
            Pubkey::find_program_address(&[MINER, payer_pk.as_ref(), &name_bytes], &program_id);

        let mut data = vec![0x20];
        data.extend_from_slice(&name_bytes);

        let accounts = vec![
            solana_sdk::instruction::AccountMeta::new(payer_pk, true),
            solana_sdk::instruction::AccountMeta::new(miner_address, false),
            solana_sdk::instruction::AccountMeta::new_readonly(system_program::ID, false),
            solana_sdk::instruction::AccountMeta::new_readonly(sysvar::rent::ID, false),
            solana_sdk::instruction::AccountMeta::new_readonly(sysvar::slot_hashes::ID, false),
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

    println!("\nPINOCCHIO MINER REGISTER RESULTS:");
    println!("Runs: {}", num_runs);
    println!("Min CUs: {}", min);
    println!("Max CUs: {}", max);
    println!("Avg CUs: {}", avg);
    println!("Total CUs: {}", total);

    println!("\nPINOCCHIO MINER REGISTER - MULTIPLE RUNS PASSED");
}
