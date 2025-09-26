use core::fmt::Write;

use pinocchio::sysvars::clock::Clock;
use pinocchio::sysvars::Sysvar;
use pinocchio::{account_info::AccountInfo, program_error::ProgramError, ProgramResult};
use tape_api::prelude::*;
use tape_api::{error::TapeError, SEGMENT_SIZE};

pub fn process_tape_write(accounts: &[AccountInfo], data: &[u8]) -> ProgramResult {
    let current_slot = Clock::get()?.slot;

    let [signer_info, tape_info, writer_info, _remaining @ ..] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    if !signer_info.is_signer() {
        return Err(ProgramError::MissingRequiredSignature);
    }

    if !tape_info.is_owned_by(&tape_api::ID) {
        return Err(ProgramError::IncorrectProgramId);
    }

    let mut tape_data = tape_info.try_borrow_mut_data()?;
    let tape = Tape::unpack_mut(&mut tape_data)?;

    if tape.authority != *signer_info.key() {
        return Err(ProgramError::MissingRequiredSignature);
    }

    if !writer_info.is_owned_by(&tape_api::ID) {
        return Err(ProgramError::IncorrectProgramId);
    }

    let mut writer_data = writer_info.try_borrow_mut_data()?;
    let writer = Writer::unpack_mut(&mut writer_data)?;

    if writer.tape != *tape_info.key() {
        return Err(ProgramError::InvalidAccountData);
    }

    let (tape_address, _tape_bump) = tape_pda(*signer_info.key(), &tape.name);
    let (writer_address, _writer_bump) = writer_pda(tape_address);

    if tape_info.key() != &tape_address {
        return Err(ProgramError::InvalidAccountData.into());
    }

    if writer_info.key() != &writer_address {
        return Err(ProgramError::InvalidAccountData.into());
    }

    check_condition(
        tape.state == TapeState::Created as u64 || tape.state == TapeState::Writing as u64,
        TapeError::UnexpectedState,
    )?;

    let segments = data.chunks(SEGMENT_SIZE);
    let segment_count = segments.len() as u64;

    check_condition(
        tape.total_segments + segment_count <= MAX_SEGMENTS_PER_TAPE as u64,
        TapeError::TapeTooLong,
    )?;

    for (segment_number, segment) in segments.enumerate() {
        let canonical_segment = padded_array::<SEGMENT_SIZE>(segment);

        // write_segment(
        //     &mut writer.state,
        //     tape.total_segments + segment_number as u64,
        //     &canonical_segment,
        // )?;
    }

    let prev_slot = tape.tail_slot;

    tape.total_segments += segment_count;
    // tape.merkle_root = writer.state.get_root().to_bytes();
    tape.state = TapeState::Writing as u64;
    tape.tail_slot = current_slot;

    WriteEvent {
        prev_slot,
        num_added: segment_count,
        num_total: tape.total_segments,
        address: tape_address,
    }
    .log();

    Ok(())
}
