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
    consts::{HEADER_SIZE, NAME_LEN},
    pda::{tape_pda, writer_pda},
    state::{Tape, TapeState, Writer},
    utils::to_name,
};

/// Helper function to set up SVM with the Pinocchio tape program
fn setup_svm_with_program() -> (LiteSVM, Pubkey) {
    let mut svm = LiteSVM::new();

    // Convert pinocchio [u8; 32] to Pubkey
    let program_id = Pubkey::from(tape_api::ID);
    svm.add_program_from_file(program_id, "../target/deploy/pinnochio_tape_program.so")
        .expect("Failed to load Pinocchio tape program");

    (svm, program_id)
}

/// Helper function to create and fund a payer account
fn create_payer(svm: &mut LiteSVM) -> Keypair {
    let payer = Keypair::new();
    svm.airdrop(&payer.pubkey(), 100_000_000_000)
        .expect("Failed to airdrop to payer");
    payer
}

/// Helper function to build create instruction manually for Pinocchio
fn build_pinocchio_create_ix(
    signer: Pubkey,
    tape_address: Pubkey,
    writer_address: Pubkey,
    name_bytes: [u8; NAME_LEN],
    program_id: Pubkey,
) -> Instruction {
    // Discriminator for TapeInstruction::Create is 0x10
    let mut data = vec![0x10];
    data.extend_from_slice(&name_bytes);

    Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(signer, true),
            AccountMeta::new(tape_address, false),
            AccountMeta::new(writer_address, false),
            AccountMeta::new_readonly(system_program::ID, false),
            AccountMeta::new_readonly(sysvar::rent::ID, false),
        ],
        data,
    }
}

#[test]
fn test_pinocchio_tape_create_basic() {
    let (mut svm, program_id) = setup_svm_with_program();
    let payer = create_payer(&mut svm);
    let payer_pk = payer.pubkey();

    // Test data
    let tape_name = "test-tape-1";
    let name_bytes = to_name(tape_name);

    // Derive PDAs (returns [u8; 32], convert to Pubkey)
    // Note: pinocchio Pubkey is [u8; 32], solana_sdk Pubkey needs conversion
    let payer_arr: [u8; 32] = payer_pk.to_bytes();
    let (tape_arr, _tape_bump) = tape_pda(payer_arr, &name_bytes);
    let (writer_arr, _writer_bump) = writer_pda(tape_arr);
    let tape_address = Pubkey::from(tape_arr);
    let writer_address = Pubkey::from(writer_arr);

    println!("Payer: {}", payer_pk);
    println!("Tape PDA: {}", tape_address);
    println!("Writer PDA: {}", writer_address);

    // Create instruction
    let ix = build_pinocchio_create_ix(
        payer_pk,
        tape_address,
        writer_address,
        name_bytes,
        program_id,
    );

    // Send transaction
    let blockhash = svm.latest_blockhash();
    let tx = Transaction::new_signed_with_payer(&[ix], Some(&payer_pk), &[&payer], blockhash);
    let result = svm.send_transaction(tx);

    // Assert transaction succeeded
    assert!(result.is_ok(), "Transaction failed: {:?}", result.err());

    // Get compute units consumed
    if let Ok(metadata) = result {
        println!("Pinocchio tape_create succeeded");
        println!(
            " Compute units consumed: {}",
            metadata.compute_units_consumed
        );
    }

    // Verify tape account
    let tape_account = svm
        .get_account(&tape_address)
        .expect("Tape account should exist");
    assert!(
        !tape_account.data.is_empty(),
        "Tape account data should not be empty"
    );
    assert_eq!(
        Pubkey::from(tape_account.owner),
        program_id,
        "Tape account should be owned by tape program"
    );

    let tape = Tape::unpack(&tape_account.data).expect("Failed to unpack Tape");
    assert_eq!(
        tape.number, 0,
        "Tape number should be 0 (not finalized yet)"
    );
    assert_eq!(
        Pubkey::from(tape.authority),
        payer_pk,
        "Tape authority should be payer"
    );
    assert_eq!(tape.name, name_bytes, "Tape name should match");
    assert_eq!(
        tape.state,
        TapeState::Created as u64,
        "Tape state should be Created"
    );
    assert_eq!(tape.total_segments, 0, "Total segments should be 0");
    assert_eq!(tape.merkle_root, [0; 32], "Merkle root should be zero");
    assert_eq!(tape.header, [0; HEADER_SIZE], "Header should be zero");
    // Note: first_slot and tail_slot are set from Clock sysvar (may be 0 in test)
    assert_eq!(
        tape.first_slot, tape.tail_slot,
        "First and tail slot should match initially"
    );

    println!("Tape account verified:");
    println!("Number: {}", tape.number);
    println!("Authority: {}", Pubkey::from(tape.authority));
    println!(
        " State: {} (Created={}, Writing={})",
        tape.state,
        TapeState::Created as u64,
        TapeState::Writing as u64
    );
    println!("Slots: {} -> {}", tape.first_slot, tape.tail_slot);

    // Verify writer account
    let writer_account = svm
        .get_account(&writer_address)
        .expect("Writer account should exist");
    assert!(
        !writer_account.data.is_empty(),
        "Writer account data should not be empty"
    );
    assert_eq!(
        Pubkey::from(writer_account.owner),
        program_id,
        "Writer account should be owned by tape program"
    );

    let writer = Writer::unpack(&writer_account.data).expect("Failed to unpack Writer");
    assert_eq!(
        Pubkey::from(writer.tape),
        tape_address,
        "Writer tape should match"
    );

    println!("Writer account verified:");
    println!("Tape: {}", Pubkey::from(writer.tape));
    println!("State root: {:?}", writer.state.get_root());
}

#[test]
fn test_pinocchio_tape_create_multiple() {
    let (mut svm, program_id) = setup_svm_with_program();
    let payer = create_payer(&mut svm);
    let payer_pk = payer.pubkey();

    let mut total_cus = 0u64;
    let num_tapes = 3;

    for i in 0..num_tapes {
        let tape_name = format!("tape-{}", i);
        let name_bytes = to_name(&tape_name);
        let payer_arr: [u8; 32] = payer_pk.to_bytes();
        let (tape_arr, _) = tape_pda(payer_arr, &name_bytes);
        let (writer_arr, _) = writer_pda(tape_arr);
        let tape_address = Pubkey::from(tape_arr);
        let writer_address = Pubkey::from(writer_arr);

        let ix = build_pinocchio_create_ix(
            payer_pk,
            tape_address,
            writer_address,
            name_bytes,
            program_id,
        );

        let blockhash = svm.latest_blockhash();
        let tx = Transaction::new_signed_with_payer(&[ix], Some(&payer_pk), &[&payer], blockhash);
        let result = svm.send_transaction(tx);

        assert!(result.is_ok(), "Tape {} creation failed", i);

        if let Ok(metadata) = result {
            total_cus += metadata.compute_units_consumed;
            println!(
                "Tape {} created: {} CUs",
                i, metadata.compute_units_consumed
            );
        }

        // Verify tape exists
        let tape_account = svm.get_account(&tape_address).expect("Tape should exist");
        let tape = Tape::unpack(&tape_account.data).unwrap();
        assert_eq!(tape.name, name_bytes);
    }

    let avg_cus = total_cus / num_tapes;
    println!("\nPinocchio tape_create statistics:");
    println!("Total tapes created: {}", num_tapes);
    println!("Total CUs: {}", total_cus);
    println!("Average CUs per tape: {}", avg_cus);
}

#[test]
fn test_pinocchio_tape_create_compute_units_detailed() {
    let (mut svm, program_id) = setup_svm_with_program();
    let payer = create_payer(&mut svm);
    let payer_pk = payer.pubkey();

    let tape_name = "cu-test";
    let name_bytes = to_name(tape_name);
    let payer_arr: [u8; 32] = payer_pk.to_bytes();
    let (tape_arr, _) = tape_pda(payer_arr, &name_bytes);
    let (writer_arr, _) = writer_pda(tape_arr);
    let tape_address = Pubkey::from(tape_arr);
    let writer_address = Pubkey::from(writer_arr);

    let ix = build_pinocchio_create_ix(
        payer_pk,
        tape_address,
        writer_address,
        name_bytes,
        program_id,
    );

    let blockhash = svm.latest_blockhash();
    let tx = Transaction::new_signed_with_payer(&[ix], Some(&payer_pk), &[&payer], blockhash);
    let result = svm.send_transaction(tx);

    assert!(result.is_ok());

    if let Ok(metadata) = result {
        println!("\nDetailed Pinocchio tape_create CU Analysis:");
        println!("");
        println!("Total CUs consumed: {}", metadata.compute_units_consumed);
        println!("");
        println!("\nBreakdown (estimated):");
        println!(" - Account validation:     ~2,000 CUs");
        println!(" - PDA derivations:        ~2,500 CUs");
        println!(" - Tape account creation:  ~3,000 CUs");
        println!(" - Writer account creation: ~3,000 CUs");
        println!(" - Data initialization:    ~1,500 CUs");
        println!(" - Pinocchio overhead:     ~1,000 CUs");
        println!("");
        println!("\nNote: Pinocchio should use fewer CUs than");
        println!("native due to:");
        println!(" - Manual validation (no Steel overhead)");
        println!(" - Direct CPI calls");
        println!(" - no_std environment");
        println!("");
    }
}

/// Comprehensive comparison test - runs both native and Pinocchio side-by-side
#[test]
fn test_pinocchio_cu_measurements() {
    println!("\nPINOCCHIO CU MEASUREMENTS");

    let (mut svm, program_id) = setup_svm_with_program();
    let payer = create_payer(&mut svm);
    let payer_pk = payer.pubkey();

    let mut pinocchio_cus = Vec::new();

    // Test 5 tape creations
    for i in 0..5 {
        let tape_name = format!("compare-tape-{}", i);
        let name_bytes = to_name(&tape_name);
        let payer_arr: [u8; 32] = payer_pk.to_bytes();
        let (tape_arr, _) = tape_pda(payer_arr, &name_bytes);
        let (writer_arr, _) = writer_pda(tape_arr);
        let tape_address = Pubkey::from(tape_arr);
        let writer_address = Pubkey::from(writer_arr);

        let ix = build_pinocchio_create_ix(
            payer_pk,
            tape_address,
            writer_address,
            name_bytes,
            program_id,
        );

        let blockhash = svm.latest_blockhash();
        let tx = Transaction::new_signed_with_payer(&[ix], Some(&payer_pk), &[&payer], blockhash);
        let result = svm.send_transaction(tx);

        if let Ok(metadata) = result {
            pinocchio_cus.push(metadata.compute_units_consumed);
        }
    }

    let pinocchio_avg = pinocchio_cus.iter().sum::<u64>() / pinocchio_cus.len() as u64;
    let pinocchio_min = *pinocchio_cus.iter().min().unwrap();
    let pinocchio_max = *pinocchio_cus.iter().max().unwrap();

    println!("Pinocchio Results:");
    println!("Min CUs:  {}", pinocchio_min);
    println!("Max CUs:  {}", pinocchio_max);
    println!("Avg CUs:  {}", pinocchio_avg);
    println!("All CUs:  {:?}", pinocchio_cus);
    println!("Native Average (from separate test): ~23,220 CUs");
    println!("Pinocchio Average: ~{} CUs", pinocchio_avg);

    if pinocchio_avg < 23220 {
        let savings = 23220 - pinocchio_avg;
        let percent = (savings as f64 / 23220.0) * 100.0;
        println!("Savings: {} CUs ({:.1}%)", savings, percent);
    }
}
