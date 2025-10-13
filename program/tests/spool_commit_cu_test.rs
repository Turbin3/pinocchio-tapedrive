#![cfg(test)]

use litesvm::LiteSVM;
use solana_sdk::{
    pubkey::Pubkey, signature::Keypair, signer::Signer, system_program, sysvar,
    transaction::Transaction,
};
use tape_api::{
    consts::{MINER, NAME_LEN, SEGMENT_PROOF_LEN, SPOOL, TAPE_TREE_HEIGHT},
    state::{Miner, Spool},
    types::ProofPath,
};
use tape_utils::{leaf::Leaf, tree::MerkleTree};

type TapeTree = MerkleTree<TAPE_TREE_HEIGHT>;

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

fn pack_value(
    svm: &mut LiteSVM,
    payer: &Keypair,
    program_id: Pubkey,
    spool_address: Pubkey,
    tape_address: Pubkey,
    value: [u8; 32],
) {
    let payer_pk = payer.pubkey();

    // Build pack instruction
    let mut data = vec![0x42]; // Pack discriminator
    data.extend_from_slice(&value);

    let accounts = vec![
        solana_sdk::instruction::AccountMeta::new(payer_pk, true),
        solana_sdk::instruction::AccountMeta::new(spool_address, false),
        solana_sdk::instruction::AccountMeta::new_readonly(tape_address, false),
    ];

    let ix = solana_sdk::instruction::Instruction {
        program_id,
        accounts,
        data,
    };

    let blockhash = svm.latest_blockhash();
    let tx = Transaction::new_signed_with_payer(&[ix], Some(&payer_pk), &[payer], blockhash);
    svm.send_transaction(tx).unwrap();
}

#[test]
fn test_pinocchio_spool_commit_cu_measurement() {
    println!("\nPINOCCHIO SPOOL COMMIT - CU MEASUREMENT TEST");

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
    let miner_address = register_miner(&mut svm, &payer, program_id, "commit-miner");
    println!("Miner registered: {}", miner_address);

    // Step 2: Create spool
    let spool_number: u64 = 0;
    let spool_address = create_spool(&mut svm, &payer, program_id, miner_address, spool_number);
    println!("Spool created: {}", spool_address);

    // Step 3: Pack a value
    let test_value = [42u8; 32];
    pack_value(
        &mut svm,
        &payer,
        program_id,
        spool_address,
        spool_address,
        test_value,
    );
    println!("Value packed");

    // Get spool state
    let spool_account = svm.get_account(&spool_address).unwrap();
    let spool = Spool::unpack(&spool_account.data).unwrap();

    // Step 4: Build merkle proof
    let leaf = Leaf::from(test_value);
    let mut tree = TapeTree::new(&[spool_address.as_ref()]);
    tree.try_add_leaf(leaf).unwrap();

    // Verify proof matches on-chain state
    assert_eq!(tree.get_root().to_bytes(), spool.contains);
    println!("Merkle proof verified locally");

    let proof_hashes = tree.get_proof_no_std(&[leaf], 0);
    let proof_array: [[u8; 32]; SEGMENT_PROOF_LEN] = proof_hashes.map(|h| h.to_bytes());

    // Step 5: Commit
    let mut data = vec![0x44]; // Commit discriminator (0x40 + 4)
    data.extend_from_slice(&test_value);
    for proof_hash in &proof_array {
        data.extend_from_slice(proof_hash);
    }

    let accounts = vec![
        solana_sdk::instruction::AccountMeta::new(payer_pk, true),
        solana_sdk::instruction::AccountMeta::new(miner_address, false),
        solana_sdk::instruction::AccountMeta::new_readonly(spool_address, false),
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

        // Verify miner commitment
        let miner_account = svm.get_account(&miner_address).unwrap();
        let miner = Miner::unpack(&miner_account.data).unwrap();

        println!("\nCommitment Set:");
        println!("Miner commitment: {:?}", &miner.commitment[..8]);

        assert_eq!(miner.commitment, test_value);

        println!(
            "\nTEST PASSED - CUs: {}",
            metadata.compute_units_consumed
        );
    } else {
        panic!("Commit failed: {:?}", result.err());
    }
}

#[test]
fn test_pinocchio_spool_commit_multiple_runs() {
    println!("\nPINOCCHIO SPOOL COMMIT - MULTIPLE RUNS");

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
        // Register miner
        let miner_name = format!("miner-{}", i);
        let miner_address = register_miner(&mut svm, &payer, program_id, &miner_name);

        // Create spool
        let spool_address = create_spool(&mut svm, &payer, program_id, miner_address, 0);

        // Pack value
        let test_value = [i as u8; 32];
        pack_value(
            &mut svm,
            &payer,
            program_id,
            spool_address,
            spool_address,
            test_value,
        );

        // Build proof
        let leaf = Leaf::from(test_value);
        let mut tree = TapeTree::new(&[spool_address.as_ref()]);
        tree.try_add_leaf(leaf).unwrap();

        let proof_hashes = tree.get_proof_no_std(&[leaf], 0);
        let proof_array: [[u8; 32]; SEGMENT_PROOF_LEN] = proof_hashes.map(|h| h.to_bytes());

        // Commit
        let mut data = vec![0x44]; // Commit discriminator
        data.extend_from_slice(&test_value);
        for proof_hash in &proof_array {
            data.extend_from_slice(proof_hash);
        }

        let accounts = vec![
            solana_sdk::instruction::AccountMeta::new(payer_pk, true),
            solana_sdk::instruction::AccountMeta::new(miner_address, false),
            solana_sdk::instruction::AccountMeta::new_readonly(spool_address, false),
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

    println!("\nPINOCCHIO SPOOL COMMIT RESULTS:");
    println!("Runs: {}", num_runs);
    println!("Min CUs: {}", min);
    println!("Max CUs: {}", max);
    println!("Avg CUs: {}", avg);
    println!("Total CUs: {}", total);

    println!("\nPINOCCHIO SPOOL COMMIT - MULTIPLE RUNS PASSED");
}
