#![cfg(test)]

use litesvm::LiteSVM;
use solana_sdk::{
    instruction::AccountMeta, pubkey::Pubkey, signature::Keypair, signer::Signer, system_program,
    sysvar, transaction::Transaction,
};
use tape_api::{
    consts::{MINER, NAME_LEN, SPOOL, TAPE, WRITER},
    state::{Spool, Tape, TapeState},
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

    let (miner_address, _miner_bump) =
        Pubkey::find_program_address(&[MINER, payer_pk.as_ref(), &name_bytes], &program_id);

    let mut data = vec![0x20];
    data.extend_from_slice(&name_bytes);

    let accounts = vec![
        AccountMeta::new(payer_pk, true),
        AccountMeta::new(miner_address, false),
        AccountMeta::new_readonly(sysvar::rent::ID, false),
        AccountMeta::new_readonly(sysvar::slot_hashes::ID, false),
        AccountMeta::new_readonly(system_program::ID, false),
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

fn create_tape(
    svm: &mut LiteSVM,
    payer: &Keypair,
    program_id: Pubkey,
    tape_name: &str,
) -> (Pubkey, Pubkey) {
    let payer_pk = payer.pubkey();
    let name_bytes = to_name(tape_name);

    let (tape_address, _tape_bump) =
        Pubkey::find_program_address(&[TAPE, payer_pk.as_ref(), &name_bytes], &program_id);

    let (writer_address, _writer_bump) =
        Pubkey::find_program_address(&[WRITER, tape_address.as_ref()], &program_id);

    let mut data = vec![0x10];
    data.extend_from_slice(&name_bytes);

    let accounts = vec![
        AccountMeta::new(payer_pk, true),
        AccountMeta::new(tape_address, false),
        AccountMeta::new(writer_address, false),
        AccountMeta::new_readonly(system_program::ID, false),
        AccountMeta::new_readonly(sysvar::rent::ID, false),
        AccountMeta::new_readonly(sysvar::clock::ID, false),
    ];

    let ix = solana_sdk::instruction::Instruction {
        program_id,
        accounts,
        data,
    };

    let blockhash = svm.latest_blockhash();
    let tx = Transaction::new_signed_with_payer(&[ix], Some(&payer_pk), &[payer], blockhash);
    svm.send_transaction(tx).unwrap();

    (tape_address, writer_address)
}

fn write_tape(
    svm: &mut LiteSVM,
    payer: &Keypair,
    program_id: Pubkey,
    tape_address: Pubkey,
    writer_address: Pubkey,
    data: &[u8],
) {
    let payer_pk = payer.pubkey();

    let mut ix_data = vec![0x11];
    ix_data.extend_from_slice(data);

    let accounts = vec![
        AccountMeta::new(payer_pk, true),
        AccountMeta::new(tape_address, false),
        AccountMeta::new(writer_address, false),
    ];

    let ix = solana_sdk::instruction::Instruction {
        program_id,
        accounts,
        data: ix_data,
    };

    let blockhash = svm.latest_blockhash();
    let tx = Transaction::new_signed_with_payer(&[ix], Some(&payer_pk), &[payer], blockhash);
    svm.send_transaction(tx).unwrap();
}

fn finalize_tape(
    svm: &mut LiteSVM,
    payer: &Keypair,
    program_id: Pubkey,
    tape_address: Pubkey,
    writer_address: Pubkey,
) {
    let payer_pk = payer.pubkey();
    let archive_address = Pubkey::from(tape_api::consts::ARCHIVE_ADDRESS);

    let data = vec![0x13];

    let accounts = vec![
        AccountMeta::new(payer_pk, true),
        AccountMeta::new(tape_address, false),
        AccountMeta::new(writer_address, false),
        AccountMeta::new(archive_address, false),
        AccountMeta::new_readonly(system_program::ID, false),
        AccountMeta::new_readonly(sysvar::rent::ID, false),
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

fn create_spool(
    svm: &mut LiteSVM,
    payer: &Keypair,
    program_id: Pubkey,
    miner_address: Pubkey,
    spool_number: u64,
) -> Pubkey {
    let payer_pk = payer.pubkey();
    let spool_number_bytes = spool_number.to_le_bytes();
    let (spool_address, _spool_bump) = Pubkey::find_program_address(
        &[SPOOL, miner_address.as_ref(), &spool_number_bytes],
        &program_id,
    );

    let mut data = vec![0x40];
    data.extend_from_slice(&spool_number_bytes);

    let accounts = vec![
        AccountMeta::new(payer_pk, true),
        AccountMeta::new(miner_address, false),
        AccountMeta::new(spool_address, false),
        AccountMeta::new_readonly(system_program::ID, false),
        AccountMeta::new_readonly(sysvar::rent::ID, false),
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

fn add_rent_to_tape(svm: &mut LiteSVM, tape_address: &Pubkey, amount: u64) {
    let mut tape_account = svm.get_account(tape_address).unwrap();
    tape_account.lamports += amount;
    svm.set_account(*tape_address, tape_account.into()).unwrap();
}

#[test]
fn test_pinocchio_spool_pack_cu_measurement() {
    println!("\nPINOCCHIO SPOOL PACK - CU MEASUREMENT TEST");

    let mut svm = LiteSVM::new();

    let program_id: Pubkey = "7wApqqrfJo2dAGAKVgheccaVEgeDoqVKogtJSTbFRWn2"
        .parse()
        .expect("Invalid program ID");

    svm.add_program_from_file(program_id, "../target/deploy/pinnochio_tape_program.so")
        .expect("Failed to load Pinocchio tape program");

    let payer = Keypair::new();
    svm.airdrop(&payer.pubkey(), 10_000_000_000)
        .expect("Failed to airdrop to payer");

    let payer_pk = payer.pubkey();

    println!("Payer: {}", payer_pk);
    println!("Program ID: {}", program_id);

    // Step 1: Register miner
    let miner_address = register_miner(&mut svm, &payer, program_id, "pack-miner");
    println!("Miner registered: {}", miner_address);

    // Step 2: Create tape
    let (tape_address, writer_address) = create_tape(&mut svm, &payer, program_id, "pack-tape");
    println!("Tape created: {}", tape_address);

    // Step 3: Write data to tape
    write_tape(
        &mut svm,
        &payer,
        program_id,
        tape_address,
        writer_address,
        b"test data",
    );
    println!("Data written to tape");

    // Step 4: Add rent to tape
    {
        let tape_account = svm.get_account(&tape_address).unwrap();
        let tape = Tape::unpack(&tape_account.data).unwrap();
        const BLOCKS_PER_YEAR: u64 = 525_600;
        let rent_needed = tape.rent_per_block() * BLOCKS_PER_YEAR;
        add_rent_to_tape(&mut svm, &tape_address, rent_needed);

        let mut tape_account = svm.get_account(&tape_address).unwrap();
        let tape_mut = Tape::unpack_mut(&mut tape_account.data).unwrap();
        tape_mut.balance = rent_needed;
        svm.set_account(tape_address, tape_account.into()).unwrap();
        println!("Added {} lamports rent to tape", rent_needed);
    }

    // Step 5: Finalize tape
    finalize_tape(&mut svm, &payer, program_id, tape_address, writer_address);
    println!("Tape finalized");

    // Verify tape is finalized
    let tape_account = svm.get_account(&tape_address).unwrap();
    let tape = Tape::unpack(&tape_account.data).unwrap();
    assert_eq!(
        tape.state,
        TapeState::Finalized as u64,
        "Tape should be finalized"
    );
    assert!(tape.number > 0, "Tape number should be > 0");

    // Step 6: Create spool
    let spool_address = create_spool(&mut svm, &payer, program_id, miner_address, 0);
    println!("Spool created: {}", spool_address);

    // Step 7: Pack value into spool
    let test_value = [42u8; 32];
    let mut data = vec![0x42];
    data.extend_from_slice(&test_value);

    let accounts = vec![
        AccountMeta::new(payer_pk, true),
        AccountMeta::new(spool_address, false),
        AccountMeta::new_readonly(tape_address, false),
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

        // Verify spool state
        let spool_account = svm.get_account(&spool_address).unwrap();
        let spool = Spool::unpack(&spool_account.data).unwrap();

        println!("\nSpool Packed:");
        println!("Total tapes: {}", spool.total_tapes);
        println!("Merkle root: {:?}", &spool.contains[..8]);

        assert_eq!(spool.total_tapes, 1);

        println!(
            "\nTEST PASSED - CUs: {}",
            metadata.compute_units_consumed
        );
    } else {
        panic!("Pack failed: {:?}", result.err());
    }
}

#[test]
fn test_pinocchio_spool_pack_multiple_runs() {
    println!("\nPINOCCHIO SPOOL PACK - MULTIPLE RUNS");

    let mut svm = LiteSVM::new();

    let program_id: Pubkey = "7wApqqrfJo2dAGAKVgheccaVEgeDoqVKogtJSTbFRWn2"
        .parse()
        .expect("Invalid program ID");

    svm.add_program_from_file(program_id, "../target/deploy/pinnochio_tape_program.so")
        .expect("Failed to load Pinocchio tape program");

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

        // Create tape
        let tape_name = format!("tape-{}", i);
        let (tape_address, writer_address) = create_tape(&mut svm, &payer, program_id, &tape_name);

        // Write data
        let data = format!("test data {}", i);
        write_tape(
            &mut svm,
            &payer,
            program_id,
            tape_address,
            writer_address,
            data.as_bytes(),
        );

        // Add rent
        {
            let tape_account = svm.get_account(&tape_address).unwrap();
            let tape = Tape::unpack(&tape_account.data).unwrap();
            const BLOCKS_PER_YEAR: u64 = 525_600;
            let rent_needed = tape.rent_per_block() * BLOCKS_PER_YEAR;
            add_rent_to_tape(&mut svm, &tape_address, rent_needed);

            let mut tape_account = svm.get_account(&tape_address).unwrap();
            let tape_mut = Tape::unpack_mut(&mut tape_account.data).unwrap();
            tape_mut.balance = rent_needed;
            svm.set_account(tape_address, tape_account.into()).unwrap();
        }

        // Finalize tape
        finalize_tape(&mut svm, &payer, program_id, tape_address, writer_address);

        // Create spool
        let spool_address = create_spool(&mut svm, &payer, program_id, miner_address, 0);

        // Pack value
        let test_value = [i as u8; 32];
        let mut data = vec![0x42];
        data.extend_from_slice(&test_value);

        let accounts = vec![
            AccountMeta::new(payer_pk, true),
            AccountMeta::new(spool_address, false),
            AccountMeta::new_readonly(tape_address, false),
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

    println!("\nPINOCCHIO SPOOL PACK RESULTS:");
    println!("Runs: {}", num_runs);
    println!("Min CUs: {}", min);
    println!("Max CUs: {}", max);
    println!("Avg CUs: {}", avg);
    println!("Total CUs: {}", total);

    println!("\nPINOCCHIO SPOOL PACK - MULTIPLE RUNS PASSED");
}
