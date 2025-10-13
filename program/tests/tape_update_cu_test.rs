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
    consts::{NAME_LEN, SEGMENT_SIZE, TAPE, WRITER},
    state::{Tape, TapeState, Writer},
    types::{ProofPath, SegmentTree},
};
use tape_utils::leaf::Leaf;

fn to_name(s: &str) -> [u8; NAME_LEN] {
    let mut name = [0u8; NAME_LEN];
    let bytes = s.as_bytes();
    let len = bytes.len().min(NAME_LEN);
    name[..len].copy_from_slice(&bytes[..len]);
    name
}

fn padded_array<const N: usize>(input: &[u8]) -> [u8; N] {
    let mut out = [0u8; N];
    let len = input.len().min(N);
    out[..len].copy_from_slice(&input[..len]);
    out
}

fn compute_leaf(segment_id: u64, segment: &[u8; SEGMENT_SIZE]) -> Leaf {
    Leaf::new(&[segment_id.to_le_bytes().as_ref(), segment])
}

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

fn write_to_tape(
    svm: &mut LiteSVM,
    payer: &Keypair,
    program_id: Pubkey,
    tape_address: Pubkey,
    writer_address: Pubkey,
    data: &[u8],
) {
    let payer_pk = payer.pubkey();

    let mut write_data = vec![0x11]; // Write discriminator
    write_data.extend_from_slice(data);

    let ix = Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(payer_pk, true),
            AccountMeta::new(tape_address, false),
            AccountMeta::new(writer_address, false),
        ],
        data: write_data,
    };

    let blockhash = svm.latest_blockhash();
    let tx = Transaction::new_signed_with_payer(&[ix], Some(&payer_pk), &[payer], blockhash);
    svm.send_transaction(tx).unwrap();
}

#[test]
fn test_pinocchio_tape_update_cu_measurement() {
    println!("\nPINOCCHIO TAPE UPDATE - CU MEASUREMENT TEST");

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
    let (tape_address, writer_address) = create_tape(&mut svm, &payer, program_id, "update-test");
    println!("Tape created: {}", tape_address);

    // Step 2: Manually setup tape state (since write instruction not implemented yet)
    let initial_data = b"Hello, original segment!";
    {
        let mut tape_account = svm.get_account(&tape_address).unwrap();
        let tape_mut = Tape::unpack_mut(&mut tape_account.data).unwrap();
        tape_mut.state = TapeState::Writing as u64;
        tape_mut.total_segments = 1;

        // Initialize writer with one segment
        let mut writer_account = svm.get_account(&writer_address).unwrap();
        let writer_mut = Writer::unpack_mut(&mut writer_account.data).unwrap();
        let segment_number: u64 = 0;
        let old_data = padded_array::<SEGMENT_SIZE>(initial_data);
        let old_leaf = compute_leaf(segment_number, &old_data);
        writer_mut.state.try_add_leaf(old_leaf).unwrap();
        tape_mut.merkle_root = writer_mut.state.get_root().to_bytes();

        svm.set_account(tape_address, tape_account.into()).unwrap();
        svm.set_account(writer_address, writer_account.into())
            .unwrap();
    }
    println!("Tape and writer state manually initialized");

    // Step 3: Prepare update
    let segment_number: u64 = 0;
    let old_data = padded_array::<SEGMENT_SIZE>(initial_data);
    let new_data_raw = b"Hello, UPDATED segment!";
    let new_data = padded_array::<SEGMENT_SIZE>(new_data_raw);

    // Build merkle proof
    let old_leaf = compute_leaf(segment_number, &old_data);
    let mut writer_tree = SegmentTree::new(&[tape_address.as_ref()]);
    writer_tree.try_add_leaf(old_leaf).unwrap();

    let proof_hashes = writer_tree.get_proof_no_std(&[old_leaf], segment_number as usize);
    let proof_nodes: Vec<[u8; 32]> = proof_hashes.iter().map(|h| h.to_bytes()).collect();

    let proof_path = ProofPath::from_slice(&proof_nodes).unwrap();

    // Step 4: Build update instruction
    let mut data = vec![0x12]; // Update discriminator
    data.extend_from_slice(&segment_number.to_le_bytes());
    data.extend_from_slice(&old_data);
    data.extend_from_slice(&new_data);
    data.extend_from_slice(bytemuck::bytes_of(&proof_path));

    let ix = Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(payer_pk, true),
            AccountMeta::new(tape_address, false),
            AccountMeta::new(writer_address, false),
        ],
        data,
    };

    let blockhash = svm.latest_blockhash();
    let tx = Transaction::new_signed_with_payer(&[ix], Some(&payer_pk), &[&payer], blockhash);
    let result = svm.send_transaction(tx);

    assert!(result.is_ok(), "Update failed: {:?}", result.err());

    if let Ok(metadata) = result {
        println!(
            "\nCOMPUTE UNITS CONSUMED: {}",
            metadata.compute_units_consumed
        );

        // Verify tape state
        let tape_account = svm.get_account(&tape_address).unwrap();
        let tape = Tape::unpack(&tape_account.data).unwrap();

        println!("\nTape Updated:");
        println!(
            " State: {} (Writing={})",
            tape.state,
            TapeState::Writing as u64
        );
        println!("Total segments: {}", tape.total_segments);

        assert_eq!(tape.state, TapeState::Writing as u64);
        assert_eq!(tape.total_segments, 1);

        // Verify merkle root
        let writer_account = svm.get_account(&writer_address).unwrap();
        let writer = Writer::unpack(&writer_account.data).unwrap();

        let new_leaf = compute_leaf(segment_number, &new_data);
        writer_tree
            .try_replace_leaf_no_std(&proof_nodes, old_leaf, new_leaf)
            .unwrap();

        assert_eq!(writer.state.get_root(), writer_tree.get_root());
        println!("Merkle root verified");

        println!(
            "\nTEST PASSED - CUs: {}",
            metadata.compute_units_consumed
        );
    }
}

#[test]
fn test_pinocchio_tape_update_multiple_runs() {
    println!("\nPINOCCHIO TAPE UPDATE - MULTIPLE RUNS");

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
        let tape_name = format!("update-{}", i);

        // Create tape
        let (tape_address, writer_address) = create_tape(&mut svm, &payer, program_id, &tape_name);

        // Manually setup tape state
        let initial_data = format!("Segment {}", i);
        {
            let mut tape_account = svm.get_account(&tape_address).unwrap();
            let tape_mut = Tape::unpack_mut(&mut tape_account.data).unwrap();
            tape_mut.state = TapeState::Writing as u64;
            tape_mut.total_segments = 1;

            let mut writer_account = svm.get_account(&writer_address).unwrap();
            let writer_mut = Writer::unpack_mut(&mut writer_account.data).unwrap();
            let segment_number: u64 = 0;
            let old_data = padded_array::<SEGMENT_SIZE>(initial_data.as_bytes());
            let old_leaf = compute_leaf(segment_number, &old_data);
            writer_mut.state.try_add_leaf(old_leaf).unwrap();
            tape_mut.merkle_root = writer_mut.state.get_root().to_bytes();

            svm.set_account(tape_address, tape_account.into()).unwrap();
            svm.set_account(writer_address, writer_account.into())
                .unwrap();
        }

        // Prepare update
        let segment_number: u64 = 0;
        let old_data = padded_array::<SEGMENT_SIZE>(initial_data.as_bytes());
        let new_data_raw = format!("Updated {}", i);
        let new_data = padded_array::<SEGMENT_SIZE>(new_data_raw.as_bytes());

        let old_leaf = compute_leaf(segment_number, &old_data);
        let mut writer_tree = SegmentTree::new(&[tape_address.as_ref()]);
        writer_tree.try_add_leaf(old_leaf).unwrap();

        let proof_hashes = writer_tree.get_proof_no_std(&[old_leaf], segment_number as usize);
        let proof_nodes: Vec<[u8; 32]> = proof_hashes.iter().map(|h| h.to_bytes()).collect();

        let proof_path = ProofPath::from_slice(&proof_nodes).unwrap();

        // Update
        let mut data = vec![0x12]; // Update discriminator
        data.extend_from_slice(&segment_number.to_le_bytes());
        data.extend_from_slice(&old_data);
        data.extend_from_slice(&new_data);
        data.extend_from_slice(bytemuck::bytes_of(&proof_path));

        let ix = Instruction {
            program_id,
            accounts: vec![
                AccountMeta::new(payer_pk, true),
                AccountMeta::new(tape_address, false),
                AccountMeta::new(writer_address, false),
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

    println!("\nPINOCCHIO UPDATE RESULTS:");
    println!("Runs: {}", num_runs);
    println!("Min CUs: {}", min);
    println!("Max CUs: {}", max);
    println!("Avg CUs: {}", avg);
    println!();
}
