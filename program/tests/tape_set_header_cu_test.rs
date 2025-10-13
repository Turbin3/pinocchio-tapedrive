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
    state::{Tape, TapeState},
};

/// Helper to convert string to fixed-size name array
fn to_name(s: &str) -> [u8; NAME_LEN] {
    let mut name = [0u8; NAME_LEN];
    let bytes = s.as_bytes();
    let len = bytes.len().min(NAME_LEN);
    name[..len].copy_from_slice(&bytes[..len]);
    name
}

/// Helper to create tape
fn create_tape(svm: &mut LiteSVM, payer: &Keypair, program_id: Pubkey, tape_name: &str) -> Pubkey {
    let payer_pk = payer.pubkey();
    let name_bytes = to_name(tape_name);

    let (tape_address, _) =
        Pubkey::find_program_address(&[TAPE, payer_pk.as_ref(), &name_bytes], &program_id);
    let (writer_address, _) =
        Pubkey::find_program_address(&[WRITER, tape_address.as_ref()], &program_id);

    let mut data = vec![0x10]; // Create discriminator
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
    let tx = Transaction::new_signed_with_payer(&[ix], Some(&payer_pk), &[payer], blockhash);
    svm.send_transaction(tx).unwrap();

    tape_address
}

/// Helper to manually set tape to Writing state
fn set_tape_writing_state(svm: &mut LiteSVM, tape_address: &Pubkey) {
    let mut tape_account = svm.get_account(tape_address).unwrap();
    let tape_mut = Tape::unpack_mut(&mut tape_account.data).unwrap();
    tape_mut.state = TapeState::Writing as u64;
    tape_mut.total_segments = 1;
    svm.set_account(*tape_address, tape_account.into()).unwrap();
}

#[test]
fn test_pinocchio_tape_set_header_cu_measurement() {
    println!("\nPINOCCHIO TAPE SET_HEADER - CU MEASUREMENT TEST\n");

    let mut svm = LiteSVM::new();

    let program_id: Pubkey = "7wApqqrfJo2dAGAKVgheccaVEgeDoqVKogtJSTbFRWn2"
        .parse()
        .expect("Invalid program ID");

    svm.add_program_from_file(program_id, "../target/deploy/pinnochio_tape_program.so")
        .expect("Failed to load program");

    let payer = Keypair::new();
    svm.airdrop(&payer.pubkey(), 10_000_000_000).unwrap();
    let payer_pk = payer.pubkey();

    println!("Payer: {}", payer_pk);
    println!("Program ID: {}", program_id);

    // Step 1: Create tape
    let tape_address = create_tape(&mut svm, &payer, program_id, "header-test");
    println!("Tape created: {}", tape_address);

    // Step 2: Set tape to Writing state
    set_tape_writing_state(&mut svm, &tape_address);
    println!("Tape set to Writing state");

    // Verify initial state
    let tape_account = svm.get_account(&tape_address).unwrap();
    let tape = Tape::unpack(&tape_account.data).unwrap();
    assert_eq!(tape.state, TapeState::Writing as u64);
    assert_eq!(
        tape.header, [0u8; HEADER_SIZE],
        "Header should be zero initially"
    );
    println!("Initial header verified as zero");

    // Step 3: Create custom header
    let mut custom_header = [0u8; HEADER_SIZE];
    custom_header[0] = 0xDE;
    custom_header[1] = 0xAD;
    custom_header[2] = 0xBE;
    custom_header[3] = 0xEF;
    for i in 4..HEADER_SIZE {
        custom_header[i] = (i % 256) as u8;
    }

    // Step 4: Set header
    let mut data = vec![0x14]; // SetHeader discriminator
    data.extend_from_slice(&custom_header);

    let ix = Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(payer_pk, true),
            AccountMeta::new(tape_address, false),
        ],
        data,
    };

    let blockhash = svm.latest_blockhash();
    let tx = Transaction::new_signed_with_payer(&[ix], Some(&payer_pk), &[&payer], blockhash);
    let result = svm.send_transaction(tx);

    assert!(result.is_ok(), "Set header failed: {:?}", result.err());

    if let Ok(metadata) = result {
        println!(
            "\nCOMPUTE UNITS CONSUMED: {}",
            metadata.compute_units_consumed
        );

        // Verify header was set
        let tape_account = svm.get_account(&tape_address).unwrap();
        let tape = Tape::unpack(&tape_account.data).unwrap();

        println!("\nHeader Set:");
        println!(
            "First 4 bytes: {:02X} {:02X} {:02X} {:02X}",
            tape.header[0], tape.header[1], tape.header[2], tape.header[3]
        );
        println!(
            "State: {} (Writing={})",
            tape.state,
            TapeState::Writing as u64
        );

        assert_eq!(tape.header, custom_header, "Header should match");
        assert_eq!(tape.header[0], 0xDE);
        assert_eq!(tape.header[1], 0xAD);
        assert_eq!(tape.header[2], 0xBE);
        assert_eq!(tape.header[3], 0xEF);

        println!("\nTEST PASSED - CUs: {}", metadata.compute_units_consumed);
    }
}

#[test]
fn test_pinocchio_tape_set_header_multiple_runs() {
    println!("\nPINOCCHIO TAPE SET_HEADER - MULTIPLE RUNS\n");

    let mut svm = LiteSVM::new();
    let program_id: Pubkey = "7wApqqrfJo2dAGAKVgheccaVEgeDoqVKogtJSTbFRWn2"
        .parse()
        .unwrap();

    svm.add_program_from_file(program_id, "../target/deploy/pinnochio_tape_program.so")
        .unwrap();

    let payer = Keypair::new();
    svm.airdrop(&payer.pubkey(), 10_000_000_000).unwrap();
    let payer_pk = payer.pubkey();

    let mut cus = Vec::new();
    let num_runs = 3;

    for i in 0..num_runs {
        let tape_name = format!("header-{}", i);

        // Create tape
        let tape_address = create_tape(&mut svm, &payer, program_id, &tape_name);
        set_tape_writing_state(&mut svm, &tape_address);

        // Create custom header
        let mut custom_header = [0u8; HEADER_SIZE];
        custom_header[0] = i as u8;
        for j in 1..HEADER_SIZE {
            custom_header[j] = ((i + j) % 256) as u8;
        }

        // Set header
        let mut data = vec![0x14]; // SetHeader discriminator
        data.extend_from_slice(&custom_header);

        let ix = Instruction {
            program_id,
            accounts: vec![
                AccountMeta::new(payer_pk, true),
                AccountMeta::new(tape_address, false),
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
    let avg = total / num_runs as u64;
    let min = *cus.iter().min().unwrap();
    let max = *cus.iter().max().unwrap();

    println!("\nPINOCCHIO SET_HEADER RESULTS:");
    println!("  Runs: {}", num_runs);
    println!("  Min CUs: {}", min);
    println!("  Max CUs: {}", max);
    println!("  Avg CUs: {}", avg);
    println!();
}
