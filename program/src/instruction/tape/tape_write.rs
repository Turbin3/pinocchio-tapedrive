use pinocchio::{
    account_info::AccountInfo,
    program_error::ProgramError,
    sysvars::{clock::Clock, Sysvar},
    ProgramResult,
};
use tape_api::{
    consts::{MAX_SEGMENTS_PER_TAPE, SEGMENT_SIZE},
    error::TapeError,
    pda::{tape_pda, writer_pda},
    state::{Tape, TapeState, Writer},
    utils::{check_condition, padded_array},
};
use tape_utils::leaf::Leaf;

// Helper function to compute leaf - same logic as tape_api::utils::compute_leaf
#[inline(always)]
fn compute_leaf(segment_id: u64, segment: &[u8; SEGMENT_SIZE]) -> Leaf {
    let segment_id_bytes = segment_id.to_le_bytes();
    Leaf::new(&[segment_id_bytes.as_ref(), segment])
}

pub fn process_tape_write(accounts: &[AccountInfo], _data: &[u8]) -> ProgramResult {
    let [signer_info, tape_info, writer_info] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    if !signer_info.is_signer() {
        return Err(ProgramError::MissingRequiredSignature);
    };

    let mut tape_info_raw_data = tape_info.try_borrow_mut_data()?;
    let tape = Tape::unpack_mut(&mut tape_info_raw_data)?;

    if signer_info.key().ne(&tape.authority) {
        return Err(ProgramError::MissingRequiredSignature);
    };

    let mut writer_info_raw_data = writer_info.try_borrow_mut_data()?;
    let writer = Writer::unpack_mut(&mut writer_info_raw_data)?;

    if writer.tape.ne(tape_info.key()) {
        return Err(ProgramError::InvalidAccountData);
    };

    let (tape_address, _) = tape_pda(*signer_info.key(), &tape.name);
    let (writer_address, _) = writer_pda(tape_address);

    if tape_info.key().ne(&tape_address) {
        return Err(ProgramError::InvalidAccountData);
    };
    if writer_info.key().ne(&writer_address) {
        return Err(ProgramError::InvalidAccountData);
    };

    check_condition(
        tape.state.eq(&(TapeState::Created as u64)) || tape.state.eq(&(TapeState::Writing as u64)),
        TapeError::UnexpectedState,
    )?;

    // Convert the data to canonical segments and write to Merkle tree
    let write_data = _data;

    // Calculate number of segments
    let segment_count = if write_data.is_empty() {
        0
    } else {
        ((write_data.len() + SEGMENT_SIZE - 1) / SEGMENT_SIZE) as u64
    };

    check_condition(
        tape.total_segments + segment_count <= MAX_SEGMENTS_PER_TAPE as u64,
        TapeError::TapeTooLong,
    )?;

    // Process each segment
    let mut offset = 0;
    for i in 0..segment_count {
        let end = core::cmp::min(offset + SEGMENT_SIZE, write_data.len());
        let segment_slice = &write_data[offset..end];
        let canonical_segment = padded_array::<SEGMENT_SIZE>(segment_slice);

        // Compute leaf and add to merkle tree
        let segment_number = tape.total_segments + i;
        let leaf = compute_leaf(segment_number, &canonical_segment);

        writer
            .state
            .try_add_leaf(leaf)
            .map_err(|_| TapeError::WriteFailed)?;

        offset = end;
    }

    let _prev_slot = tape.tail_slot;
    let current_slot = Clock::get()?.slot;

    tape.total_segments += segment_count;
    tape.merkle_root = writer.state.get_root().to_bytes();
    tape.state = TapeState::Writing as u64;
    tape.tail_slot = current_slot;

    // No event logging in Pinocchio for now

    Ok(())
}
