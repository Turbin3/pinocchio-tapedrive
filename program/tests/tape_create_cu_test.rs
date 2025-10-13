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
    consts::{HEADER_SIZE, NAME_LEN, TAPE, WRITER},
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

#[test]
fn test_pinocchio_tape_create_cu_measurement() {
    println!("\nPINOCCHIO TAPE CREATE - CU MEASUREMENT TEST");

    // Setup SVM
    let mut svm = LiteSVM::new();

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

    // Test data
    let tape_name = "test-tape-1";
    let name_bytes = to_name(tape_name);

    // Derive PDAs using Solana SDK (not pinocchio api)
    let (tape_address, _tape_bump) =
        Pubkey::find_program_address(&[TAPE, payer_pk.as_ref(), &name_bytes], &program_id);

    let (writer_address, _writer_bump) =
        Pubkey::find_program_address(&[WRITER, tape_address.as_ref()], &program_id);

    println!("Payer: {}", payer_pk);
    println!("Program ID: {}", program_id);
    println!("Tape PDA: {}", tape_address);
    println!("Writer PDA: {}", writer_address);
    println!();

    // Build instruction manually
    let mut data = vec![0x10]; // TapeInstruction::Create discriminator
    data.extend_from_slice(&name_bytes);

    let ix = Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(payer_pk, true),
            AccountMeta::new(tape_address, false),
            AccountMeta::new(writer_address, false),
            AccountMeta::new_readonly(system_program::ID, false),
            AccountMeta::new_readonly(sysvar::rent::ID, false),
        ],
        data,
    };

    // Send transaction
    let blockhash = svm.latest_blockhash();
    let tx = Transaction::new_signed_with_payer(&[ix], Some(&payer_pk), &[&payer], blockhash);
    let result = svm.send_transaction(tx);

    // Check result and get CUs
    assert!(result.is_ok(), "Transaction failed: {:?}", result.err());

    if let Ok(metadata) = result {
        println!("Pinocchio tape_create succeeded!");
        println!();
        println!(
            " COMPUTE UNITS CONSUMED: {}",
            metadata.compute_units_consumed
        );
        println!();

        // Verify tape account
        let tape_account = svm
            .get_account(&tape_address)
            .expect("Tape account should exist");

        let tape = Tape::unpack(&tape_account.data).expect("Failed to unpack Tape");

        println!("Tape Account Verification:");
        println!("Number: {}", tape.number);
        println!("Authority: {}", Pubkey::from(tape.authority));
        println!(
            " Name: {:?}",
            std::str::from_utf8(&tape.name).unwrap_or("<invalid>")
        );
        println!(
            " State: {} (Created={})",
            tape.state,
            TapeState::Created as u64
        );
        println!("Total segments: {}", tape.total_segments);
        println!("First slot: {}", tape.first_slot);
        println!("Tail slot: {}", tape.tail_slot);

        // Assertions
        assert_eq!(tape.number, 0, "Tape number should be 0");
        assert_eq!(Pubkey::from(tape.authority), payer_pk, "Authority mismatch");
        assert_eq!(tape.name, name_bytes, "Name mismatch");
        assert_eq!(
            tape.state,
            TapeState::Created as u64,
            "State should be Created"
        );
        assert_eq!(tape.total_segments, 0, "Total segments should be 0");
        assert_eq!(tape.merkle_root, [0; 32], "Merkle root should be zero");
        assert_eq!(tape.header, [0; HEADER_SIZE], "Header should be zero");

        println!();

        // Verify writer account
        let writer_account = svm
            .get_account(&writer_address)
            .expect("Writer account should exist");

        let writer = Writer::unpack(&writer_account.data).expect("Failed to unpack Writer");

        println!("Writer Account Verification:");
        println!("Tape: {}", Pubkey::from(writer.tape));
        println!("State root: {:?}", writer.state.get_root());

        assert_eq!(
            Pubkey::from(writer.tape),
            tape_address,
            "Writer tape mismatch"
        );

        println!();
        println!("");
        println!("TEST PASSED - CUs: {}", metadata.compute_units_consumed);
        println!("");
    }
}

#[test]
fn test_pinocchio_tape_create_multiple_for_average() {
    println!("\nPINOCCHIO TAPE CREATE - MULTIPLE RUNS FOR AVERAGE");

    let mut svm = LiteSVM::new();

    let program_id: Pubkey = "7wApqqrfJo2dAGAKVgheccaVEgeDoqVKogtJSTbFRWn2"
        .parse()
        .expect("Invalid program ID");

    svm.add_program_from_file(program_id, "../target/deploy/pinnochio_tape_program.so")
        .expect("Failed to load program");

    let payer = Keypair::new();
    svm.airdrop(&payer.pubkey(), 10_000_000_000).unwrap();
    let payer_pk = payer.pubkey();

    let mut cus = Vec::new();
    let num_runs = 5;

    for i in 0..num_runs {
        let tape_name = format!("tape-{}", i);
        let name_bytes = to_name(&tape_name);

        let (tape_address, _) =
            Pubkey::find_program_address(&[TAPE, payer_pk.as_ref(), &name_bytes], &program_id);

        let (writer_address, _) =
            Pubkey::find_program_address(&[WRITER, tape_address.as_ref()], &program_id);

        let mut data = vec![0x10];
        data.extend_from_slice(&name_bytes);

        let ix = Instruction {
            program_id,
            accounts: vec![
                AccountMeta::new(payer_pk, true),
                AccountMeta::new(tape_address, false),
                AccountMeta::new(writer_address, false),
                AccountMeta::new_readonly(system_program::ID, false),
                AccountMeta::new_readonly(sysvar::rent::ID, false),
            ],
            data,
        };

        let blockhash = svm.latest_blockhash();
        let tx = Transaction::new_signed_with_payer(&[ix], Some(&payer_pk), &[&payer], blockhash);
        let result = svm.send_transaction(tx);

        assert!(result.is_ok(), "Run {} failed", i);

        if let Ok(metadata) = result {
            cus.push(metadata.compute_units_consumed);
            println!("Run {}: {} CUs", i, metadata.compute_units_consumed);
        }
    }

    let total: u64 = cus.iter().sum();
    let avg = total / num_runs;
    let min = *cus.iter().min().unwrap();
    let max = *cus.iter().max().unwrap();

    println!();
    println!("PINOCCHIO RESULTS:");
    println!("Runs: {}", num_runs);
    println!("Min CUs: {}", min);
    println!("Max CUs: {}", max);
    println!("Avg CUs: {}", avg);
    println!("Total CUs: {}", total);
    println!();
    println!("COMPARISON WITH NATIVE:");
    println!("Native Avg: ~23,220 CUs");
    println!("Pinocchio Avg: {} CUs", avg);

    if avg < 23220 {
        let savings = 23220 - avg;
        let percent = (savings as f64 / 23220.0) * 100.0;
        println!("Savings: {} CUs ({:.1}%)", savings, percent);
    } else {
        let overhead = avg - 23220;
        let percent = (overhead as f64 / 23220.0) * 100.0;
        println!("Overhead: {} CUs ({:.1}%)", overhead, percent);
    }

    println!();
}
