use tape_api::prelude::*;

use pinocchio::{account_info::AccountInfo, program_error::ProgramError, ProgramResult};
use tape_api::state::{Archive, Tape, Writer};

pub fn process_tape_finalize(accounts: &[AccountInfo], _data: &[u8]) -> ProgramResult {
    let [signer_info, tape_info, writer_info, archive_info, _system_program_info, _rent_sysvar_info, _remaining @ ..] =
        accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    if !signer_info.is_signer() {
        return Err(ProgramError::MissingRequiredSignature);
    }

    let mut tape_data = tape_info.try_borrow_mut_data()?;
    let tape = Tape::unpack_mut(&mut tape_data)?;

    if tape.authority != *signer_info.key() {
        return Err(ProgramError::MissingRequiredSignature);
    }

    if !tape_info.is_owned_by(&tape_api::ID) {
        return Err(ProgramError::IncorrectProgramId);
    }

    if !writer_info.is_owned_by(&tape_api::ID) {
        return Err(ProgramError::IncorrectProgramId);
    }

    let mut writer_data = writer_info.try_borrow_mut_data()?;
    let writer = Writer::unpack_mut(&mut writer_data)?;

    if writer.tape != *tape_info.key() {
        return Err(ProgramError::InvalidAccountData);
    }

    let mut archive_data = archive_info.try_borrow_mut_data()?;
    let archive = Archive::unpack_mut(&mut archive_data)?;

    let (tape_address, _tape_bump) = tape_pda(*signer_info.key(), &tape.name);

    let (writer_address, _writer_bump) = writer_pda(tape_address);

    if tape_info.key() != &tape_address {
        return Err(ProgramError::InvalidAccountData.into());
    }

    if writer_info.key() != &writer_address {
        return Err(ProgramError::InvalidAccountData.into());
    }

    check_condition(
        tape.state == TapeState::Writing as u64,
        TapeError::UnexpectedState,
    )?;

    check_condition(tape.can_finalize(), TapeError::InsufficientRent)?;

    archive.tapes_stored = archive.tapes_stored.saturating_add(1);
    archive.segments_stored = archive.segments_stored.saturating_add(tape.total_segments);

    tape.number = archive.tapes_stored;
    tape.state = TapeState::Finalized as u64;
    // tape.merkle_root = writer.state.get_root().into();

    *signer_info.try_borrow_mut_lamports()? += *writer_info.try_borrow_lamports()?;
    *writer_info.try_borrow_mut_lamports()? = 0;
    writer_info.close()?;

    FinalizeEvent {
        tape: tape.number,
        address: tape_address,
    };

    Ok(())
}
