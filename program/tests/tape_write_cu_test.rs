#![cfg(test)]

use litesvm::LiteSVM;
use solana_sdk::{
    pubkey::Pubkey, signature::Keypair, signer::Signer, system_program, transaction::Transaction,
};
use tape_api::{
    consts::{ARCHIVE_ADDRESS, NAME_LEN, TAPE, WRITER},
    state::{Tape, TapeState, Writer},
};

/// Helper to convert string to fixed-size name array
fn to_name(s: &str) -> [u8; NAME_LEN] {
    let mut name = [0u8; NAME_LEN];
    let bytes = s.as_bytes();
    let len = bytes.len().min(NAME_LEN);
    name[..len].copy_from_slice(&bytes[..len]);
    name
}

fn initialize_program(svm: &mut LiteSVM, payer: &Keypair, program_id: Pubkey) {
    let payer_pk = payer.pubkey();

    // Build initialize instruction
    let data = vec![0x00]; // Initialize discriminator

    let archive_address = Pubkey::from(ARCHIVE_ADDRESS);

    let accounts = vec![
        solana_sdk::instruction::AccountMeta::new(payer_pk, true),
        solana_sdk::instruction::AccountMeta::new(archive_address, false),
        solana_sdk::instruction::AccountMeta::new_readonly(system_program::ID, false),
    ];

    let ix = solana_sdk::instruction::Instruction {
        program_id,
        accounts,
        data,
    };

    let blockhash = svm.latest_blockhash();
    let tx = Transaction::new_signed_with_payer(&[ix], Some(&payer_pk), &[payer], blockhash);
    svm.send_transaction(tx)
        .expect("Initialize instruction failed");
}

fn create_tape(
    svm: &mut LiteSVM,
    payer: &Keypair,
    program_id: Pubkey,
    tape_name: &str,
) -> (Pubkey, Pubkey) {
    let payer_pk = payer.pubkey();
    let name_bytes = to_name(tape_name);

    // Derive PDAs using Solana SDK
    let (tape_address, _tape_bump) =
        Pubkey::find_program_address(&[TAPE, payer_pk.as_ref(), &name_bytes], &program_id);

    let (writer_address, _writer_bump) =
        Pubkey::find_program_address(&[WRITER, tape_address.as_ref()], &program_id);

    // Build create instruction manually
    let mut data = vec![0x10]; // Create discriminator
    data.extend_from_slice(&name_bytes);

    let accounts = vec![
        solana_sdk::instruction::AccountMeta::new(payer_pk, true),
        solana_sdk::instruction::AccountMeta::new(tape_address, false),
        solana_sdk::instruction::AccountMeta::new(writer_address, false),
        solana_sdk::instruction::AccountMeta::new_readonly(system_program::ID, false),
    ];

    let ix = solana_sdk::instruction::Instruction {
        program_id,
        accounts,
        data,
    };

    let blockhash = svm.latest_blockhash();
    let tx = Transaction::new_signed_with_payer(&[ix], Some(&payer_pk), &[payer], blockhash);
    let _result = svm.send_transaction(tx).unwrap();

    (tape_address, writer_address)
}

#[test]
fn test_pinocchio_tape_write_cu_measurement() {
    println!("\nPINOCCHIO TAPE WRITE - CU MEASUREMENT TEST");

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

    // Step 1: Create tape
    let (tape_address, writer_address) = create_tape(&mut svm, &payer, program_id, "write-test");
    println!("Tape created: {}", tape_address);

    // Verify initial state
    let tape_account = svm.get_account(&tape_address).unwrap();
    let tape = Tape::unpack(&tape_account.data).unwrap();
    assert_eq!(
        tape.state,
        TapeState::Created as u64,
        "Tape should be in Created state"
    );
    assert_eq!(tape.total_segments, 0, "Tape should have 0 segments");

    // Step 2: Write data
    let write_data = b"Hello, Pinocchio World! This is a test segment.";

    // Build write instruction
    let mut data = vec![0x11]; // Write discriminator
    data.extend_from_slice(write_data);

    let accounts = vec![
        solana_sdk::instruction::AccountMeta::new(payer_pk, true),
        solana_sdk::instruction::AccountMeta::new(tape_address, false),
        solana_sdk::instruction::AccountMeta::new(writer_address, false),
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

        // Verify tape state
        let tape_account = svm.get_account(&tape_address).unwrap();
        let tape = Tape::unpack(&tape_account.data).unwrap();

        println!("\nTape Written:");
        println!(
            " State: {} (Writing={})",
            tape.state,
            TapeState::Writing as u64
        );
        println!("Total segments: {}", tape.total_segments);

        assert_eq!(
            tape.state,
            TapeState::Writing as u64,
            "Tape should be in Writing state"
        );
        assert_eq!(tape.total_segments, 1, "Tape should have 1 segment");

        // Verify writer merkle root
        let writer_account = svm.get_account(&writer_address).unwrap();
        let writer = Writer::unpack(&writer_account.data).unwrap();

        assert_eq!(
            tape.merkle_root,
            writer.state.get_root().to_bytes(),
            "Merkle roots should match"
        );
        println!("Merkle root verified");

        println!(
            "\nTEST PASSED - CUs: {}",
            metadata.compute_units_consumed
        );
    } else {
        panic!("Write failed: {:?}", result.err());
    }
}

#[test]
fn test_pinocchio_tape_write_multiple_runs() {
    println!("\nPINOCCHIO TAPE WRITE - MULTIPLE RUNS");

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
        let tape_name = format!("write-{}", i);

        // Create tape
        let (tape_address, writer_address) = create_tape(&mut svm, &payer, program_id, &tape_name);

        // Write data
        let write_data = format!("Segment {}", i);
        let mut data = vec![0x11]; // Write discriminator
        data.extend_from_slice(write_data.as_bytes());

        let accounts = vec![
            solana_sdk::instruction::AccountMeta::new(payer_pk, true),
            solana_sdk::instruction::AccountMeta::new(tape_address, false),
            solana_sdk::instruction::AccountMeta::new(writer_address, false),
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

    println!("\nPINOCCHIO WRITE RESULTS:");
    println!("Runs: {}", num_runs);
    println!("Min CUs: {}", min);
    println!("Max CUs: {}", max);
    println!("Avg CUs: {}", avg);
    println!("Total CUs: {}", total);

    println!("\nPINOCCHIO TAPE WRITE - MULTIPLE RUNS PASSED");
}
