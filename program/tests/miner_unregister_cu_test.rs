#![cfg(test)]

use litesvm::LiteSVM;
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signature::Keypair,
    signer::Signer,
    system_program, sysvar,
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
fn test_pinocchio_miner_unregister_single() {
    println!("\nPINOCCHIO MINER UNREGISTER - SINGLE RUN");

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
    let mut register_data = vec![0x20]; // Register discriminator
    register_data.extend_from_slice(&name_bytes);

    let register_ix = Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(payer_pk, true),
            AccountMeta::new(miner_address, false),
            AccountMeta::new_readonly(system_program::ID, false),
            AccountMeta::new_readonly(sysvar::rent::ID, false),
            AccountMeta::new_readonly(sysvar::slot_hashes::ID, false),
        ],
        data: register_data,
    };

    let blockhash = svm.latest_blockhash();
    let tx =
        Transaction::new_signed_with_payer(&[register_ix], Some(&payer_pk), &[&payer], blockhash);
    let result = svm.send_transaction(tx);

    assert!(result.is_ok(), "Register failed: {:?}", result.err());
    println!("Miner registered: {}", miner_address);

    // Verify miner exists
    let miner_account = svm.get_account(&miner_address).unwrap();
    let miner = Miner::unpack(&miner_account.data).unwrap();
    assert_eq!(miner.authority.as_ref(), payer_pk.as_ref());
    assert_eq!(miner.unclaimed_rewards, 0);
    println!("Authority: {:?}", &miner.authority[..8]);
    println!("Unclaimed rewards: {}", miner.unclaimed_rewards);

    // Get payer balance before unregister
    let payer_balance_before = svm.get_account(&payer_pk).unwrap().lamports;
    println!(
        "\nPayer balance before: {} lamports",
        payer_balance_before
    );

    // Step 2: Unregister the miner
    let unregister_data = vec![0x21]; // Unregister discriminator

    let unregister_ix = Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(payer_pk, true),
            AccountMeta::new(miner_address, false),
            AccountMeta::new_readonly(system_program::ID, false),
        ],
        data: unregister_data,
    };

    let blockhash = svm.latest_blockhash();
    let tx =
        Transaction::new_signed_with_payer(&[unregister_ix], Some(&payer_pk), &[&payer], blockhash);

    let result = svm.send_transaction(tx);

    if let Ok(metadata) = result {
        println!(
            "\nCOMPUTE UNITS CONSUMED: {}",
            metadata.compute_units_consumed
        );

        // Verify miner account is closed
        let miner_account_result = svm.get_account(&miner_address);
        if let Some(account) = miner_account_result {
            println!("\nMiner Account After Unregister:");
            println!("Data length: {}", account.data.len());
            println!("Lamports: {}", account.lamports);

            // Account should be fully closed (LiteSVM removes it completely)
            assert_eq!(
                account.data.len(),
                0,
                "Account data should be empty after close"
            );
            assert_eq!(
                account.lamports, 0,
                "Account lamports should be 0 after close"
            );
        } else {
            println!("\nMiner Account After Unregister: Fully closed (removed)");
        }

        // Verify payer received rent back
        let payer_balance_after = svm.get_account(&payer_pk).unwrap().lamports;
        println!("Payer balance after: {} lamports", payer_balance_after);
        println!(
            " Rent returned: {} lamports",
            payer_balance_after.saturating_sub(payer_balance_before)
        );

        // Payer should have more lamports (rent returned minus tx fee)
        assert!(
            payer_balance_after > payer_balance_before,
            "Payer should receive rent back"
        );

        println!(
            "\nTEST PASSED - CUs: {}",
            metadata.compute_units_consumed
        );
    } else {
        panic!("Unregister failed: {:?}", result.err());
    }
}

#[test]
fn test_pinocchio_miner_unregister_multiple_runs() {
    println!("\nPINOCCHIO MINER UNREGISTER - MULTIPLE RUNS");

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
        // Register a miner
        let miner_name = format!("miner-{}", i);
        let name_bytes = to_name(&miner_name);

        let (miner_address, _miner_bump) =
            Pubkey::find_program_address(&[MINER, payer_pk.as_ref(), &name_bytes], &program_id);

        let mut register_data = vec![0x20];
        register_data.extend_from_slice(&name_bytes);

        let register_ix = Instruction {
            program_id,
            accounts: vec![
                AccountMeta::new(payer_pk, true),
                AccountMeta::new(miner_address, false),
                AccountMeta::new_readonly(system_program::ID, false),
                AccountMeta::new_readonly(sysvar::rent::ID, false),
                AccountMeta::new_readonly(sysvar::slot_hashes::ID, false),
            ],
            data: register_data,
        };

        let blockhash = svm.latest_blockhash();
        let tx = Transaction::new_signed_with_payer(
            &[register_ix],
            Some(&payer_pk),
            &[&payer],
            blockhash,
        );
        let result = svm.send_transaction(tx);
        assert!(result.is_ok(), "Register run {} failed", i);

        // Unregister the miner
        let unregister_data = vec![0x21];

        let unregister_ix = Instruction {
            program_id,
            accounts: vec![
                AccountMeta::new(payer_pk, true),
                AccountMeta::new(miner_address, false),
                AccountMeta::new_readonly(system_program::ID, false),
            ],
            data: unregister_data,
        };

        let blockhash = svm.latest_blockhash();
        let tx = Transaction::new_signed_with_payer(
            &[unregister_ix],
            Some(&payer_pk),
            &[&payer],
            blockhash,
        );
        let result = svm.send_transaction(tx);

        assert!(
            result.is_ok(),
            "Unregister run {} failed: {:?}",
            i,
            result.err()
        );

        if let Ok(metadata) = result {
            cus.push(metadata.compute_units_consumed);
            println!("Run {}: {} CUs", i, metadata.compute_units_consumed);
        }
    }

    let total: u64 = cus.iter().sum();
    let avg = total / num_runs as u64;
    let min = *cus.iter().min().unwrap();
    let max = *cus.iter().max().unwrap();

    println!("\nPINOCCHIO MINER UNREGISTER RESULTS:");
    println!("Runs: {}", num_runs);
    println!("Min CUs: {}", min);
    println!("Max CUs: {}", max);
    println!("Avg CUs: {}", avg);
    println!("Total CUs: {}", total);

    println!("\nPINOCCHIO MINER UNREGISTER - MULTIPLE RUNS PASSED");
}
