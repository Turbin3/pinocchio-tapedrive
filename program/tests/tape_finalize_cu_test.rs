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
    consts::{ARCHIVE_ADDRESS, HEADER_SIZE, NAME_LEN, TAPE, WRITER},
    state::{Archive, Tape, TapeState, Writer},
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
fn create_tape(
    svm: &mut LiteSVM,
    payer: &Keypair,
    program_id: Pubkey,
    tape_name: &str,
) -> (Pubkey, Pubkey) {
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

    (tape_address, writer_address)
}

/// Helper to manually set tape to Writing state
fn set_tape_writing_state(svm: &mut LiteSVM, tape_address: &Pubkey) {
    let mut tape_account = svm.get_account(tape_address).unwrap();
    let tape_mut = Tape::unpack_mut(&mut tape_account.data).unwrap();
    tape_mut.state = TapeState::Writing as u64;
    tape_mut.total_segments = 1; // Add at least one segment
    svm.set_account(*tape_address, tape_account.into()).unwrap();
}

#[test]
fn test_pinocchio_tape_finalize_cu_measurement() {
    println!("\nPINOCCHIO TAPE FINALIZE - CU MEASUREMENT TEST");

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
    let (tape_address, writer_address) = create_tape(&mut svm, &payer, program_id, "finalize-test");
    println!("Tape created: {}", tape_address);
    println!("Writer created: {}", writer_address);

    // Step 2: Set tape to Writing state
    set_tape_writing_state(&mut svm, &tape_address);
    println!("Tape set to Writing state");

    // Verify tape is in Writing state
    let tape_account = svm.get_account(&tape_address).unwrap();
    let tape = Tape::unpack(&tape_account.data).unwrap();
    assert_eq!(
        tape.state,
        TapeState::Writing as u64,
        "Tape should be in Writing state"
    );
    println!("Tape state: Writing");
    println!("Total segments: {}", tape.total_segments);

    // Step 3: Add rent for finalization
    const BLOCKS_PER_YEAR: u64 = 525_600; // 60 * 60 * 24 * 365 / 60
    let rent_needed = tape.rent_per_block() * BLOCKS_PER_YEAR;
    let mut tape_account = svm.get_account(&tape_address).unwrap();
    tape_account.lamports += rent_needed;

    // Update balance in tape data
    {
        let tape_mut = Tape::unpack_mut(&mut tape_account.data).unwrap();
        tape_mut.balance = rent_needed;
    }

    svm.set_account(tape_address, tape_account.into()).unwrap();
    println!("Added {} lamports rent", rent_needed);

    // Step 4: Create archive account if needed
    let archive_address = Pubkey::from(ARCHIVE_ADDRESS);
    if svm.get_account(&archive_address).is_none() {
        // Create archive manually for test
        let mut archive_account = solana_sdk::account::Account {
            lamports: 10_000_000,
            data: vec![0; core::mem::size_of::<Archive>()],
            owner: program_id,
            executable: false,
            rent_epoch: 0,
        };
        svm.set_account(archive_address, archive_account.into())
            .unwrap();
        println!("Created archive account");
    }

    // Step 5: Finalize tape
    let mut finalize_data = vec![0x13]; // Finalize discriminator

    let ix = Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(payer_pk, true),
            AccountMeta::new(tape_address, false),
            AccountMeta::new(writer_address, false),
            AccountMeta::new(archive_address, false),
            AccountMeta::new_readonly(system_program::ID, false),
            AccountMeta::new_readonly(sysvar::rent::ID, false),
        ],
        data: finalize_data,
    };

    let blockhash = svm.latest_blockhash();
    let tx = Transaction::new_signed_with_payer(&[ix], Some(&payer_pk), &[&payer], blockhash);
    let result = svm.send_transaction(tx);

    assert!(result.is_ok(), "Finalize failed: {:?}", result.err());

    if let Ok(metadata) = result {
        println!(
            "\nCOMPUTE UNITS CONSUMED: {}",
            metadata.compute_units_consumed
        );

        // Verify tape is finalized
        let tape_account = svm.get_account(&tape_address).unwrap();
        let tape = Tape::unpack(&tape_account.data).unwrap();

        println!("\nTape Finalized:");
        println!("Number: {}", tape.number);
        println!(
            " State: {} (Finalized={})",
            tape.state,
            TapeState::Finalized as u64
        );
        println!("Total segments: {}", tape.total_segments);

        assert_eq!(tape.state, TapeState::Finalized as u64);
        assert_eq!(tape.number, 1, "Should be tape number 1");

        // Verify writer is closed
        let writer_account = svm.get_account(&writer_address);
        assert!(
            writer_account.is_none() || writer_account.as_ref().unwrap().data.len() <= 1,
            "Writer should be closed"
        );
        println!("Writer account closed");

        // Verify archive
        let archive_account = svm.get_account(&archive_address).unwrap();
        let archive = Archive::unpack(&archive_account.data).unwrap();

        println!("\nArchive Updated:");
        println!("Tapes stored: {}", archive.tapes_stored);
        println!("Segments stored: {}", archive.segments_stored);

        assert_eq!(archive.tapes_stored, 1);

        println!(
            "\nTEST PASSED - CUs: {}",
            metadata.compute_units_consumed
        );
    }
}

#[test]
fn test_pinocchio_tape_finalize_multiple_runs() {
    println!("\nPINOCCHIO TAPE FINALIZE - MULTIPLE RUNS");

    let mut svm = LiteSVM::new();
    let program_id: Pubkey = "7wApqqrfJo2dAGAKVgheccaVEgeDoqVKogtJSTbFRWn2"
        .parse()
        .unwrap();

    svm.add_program_from_file(program_id, "../target/deploy/pinnochio_tape_program.so")
        .unwrap();

    let payer = Keypair::new();
    svm.airdrop(&payer.pubkey(), 10_000_000_000).unwrap();
    let payer_pk = payer.pubkey();

    // Setup archive
    let archive_address = Pubkey::from(ARCHIVE_ADDRESS);
    let archive_account = solana_sdk::account::Account {
        lamports: 10_000_000,
        data: vec![0; core::mem::size_of::<Archive>()],
        owner: program_id,
        executable: false,
        rent_epoch: 0,
    };
    svm.set_account(archive_address, archive_account.into())
        .unwrap();

    let mut cus = Vec::new();
    let num_runs = 3;

    for i in 0..num_runs {
        let tape_name = format!("finalize-{}", i);

        // Create tape
        let (tape_address, writer_address) = create_tape(&mut svm, &payer, program_id, &tape_name);
        set_tape_writing_state(&mut svm, &tape_address);

        // Add rent
        const BLOCKS_PER_YEAR: u64 = 525_600;
        let tape_account = svm.get_account(&tape_address).unwrap();
        let tape = Tape::unpack(&tape_account.data).unwrap();
        let rent_needed = tape.rent_per_block() * BLOCKS_PER_YEAR;

        let mut tape_account = svm.get_account(&tape_address).unwrap();
        tape_account.lamports += rent_needed;
        let tape_mut = Tape::unpack_mut(&mut tape_account.data).unwrap();
        tape_mut.balance = rent_needed;
        svm.set_account(tape_address, tape_account.into()).unwrap();

        // Finalize
        let ix = Instruction {
            program_id,
            accounts: vec![
                AccountMeta::new(payer_pk, true),
                AccountMeta::new(tape_address, false),
                AccountMeta::new(writer_address, false),
                AccountMeta::new(archive_address, false),
                AccountMeta::new_readonly(system_program::ID, false),
                AccountMeta::new_readonly(sysvar::rent::ID, false),
            ],
            data: vec![0x13], // Finalize discriminator
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

    println!("\nPINOCCHIO FINALIZE RESULTS:");
    println!("Runs: {}", num_runs);
    println!("Min CUs: {}", min);
    println!("Max CUs: {}", max);
    println!("Avg CUs: {}", avg);
    println!();
}
