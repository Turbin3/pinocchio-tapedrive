use pinocchio::{account_info::AccountInfo, program_error::ProgramError, ProgramResult};
use tape_api::{
    consts::ARCHIVE_ADDRESS,
    pda::{tape_pda, writer_pda},
    state::{Archive, Tape, TapeState, Writer},
};

use crate::instruction::Finalize;
use crate::utils::ByteConversion;

pub fn process_tape_finalize(accounts: &[AccountInfo], data: &[u8]) -> ProgramResult {
    let _args = Finalize::try_from_bytes(data)?;

    let [signer_info, tape_info, writer_info, archive_info, _remaining @ ..] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    // Validate signer
    if !signer_info.is_signer() {
        return Err(ProgramError::MissingRequiredSignature);
    }

    // Load and validate tape account
    let mut tape_data = tape_info.try_borrow_mut_data()?;
    let tape = Tape::unpack_mut(&mut tape_data)?;

    // Validate tape authority matches signer
    if tape.authority.ne(signer_info.key()) {
        return Err(ProgramError::MissingRequiredSignature);
    }

    // Load and validate writer account
    let writer_data = writer_info.try_borrow_data()?;
    let writer = Writer::unpack(&writer_data)?;

    // Validate writer tape matches tape account
    if writer.tape.ne(tape_info.key()) {
        return Err(ProgramError::InvalidAccountData);
    }

    // Drop writer borrow before we close it
    drop(writer_data);

    // Derive and validate PDAs
    let (tape_address, _tape_bump) = tape_pda(tape.authority, &tape.name);
    let (writer_address, _writer_bump) = writer_pda(tape_address);

    if tape_info.key().ne(&tape_address) {
        return Err(ProgramError::InvalidAccountData);
    }

    if writer_info.key().ne(&writer_address) {
        return Err(ProgramError::InvalidAccountData);
    }

    // Validate archive account
    if archive_info.key().ne(&ARCHIVE_ADDRESS) {
        return Err(ProgramError::InvalidAccountData);
    }

    // Load archive
    let mut archive_data = archive_info.try_borrow_mut_data()?;
    let archive = Archive::unpack_mut(&mut archive_data)?;

    // Can't finalize if the tape is not in Writing state
    if tape.state != (TapeState::Writing as u64) {
        return Err(ProgramError::InvalidAccountData); // UnexpectedState
    }

    // Can't finalize the tape if it doesn't have enough rent
    if !tape.can_finalize() {
        return Err(ProgramError::InvalidAccountData); // InsufficientRent
    }

    // Update archive counters
    archive.tapes_stored = archive.tapes_stored.saturating_add(1);
    archive.segments_stored = archive.segments_stored.saturating_add(tape.total_segments);

    // Update tape
    tape.number = archive.tapes_stored;
    tape.state = TapeState::Finalized as u64;
    // merkle_root is already set from writer's state during write operations

    // Drop borrows before closing writer
    drop(tape_data);
    drop(archive_data);

    // Close the writer account and return rent to signer
    close_writer_account(writer_info, signer_info)?;

    // Note: Native logs FinalizeEvent here, but we'll skip logging for now

    Ok(())
}

/// Close writer account and return rent to destination
#[inline(always)]
fn close_writer_account(account: &AccountInfo, destination: &AccountInfo) -> ProgramResult {
    // Set first byte to 0xff to prevent reinitialization
    {
        let mut data = account.try_borrow_mut_data()?;
        if !data.is_empty() {
            data[0] = 0xff;
        }
    }

    // Transfer all lamports to destination
    *destination.try_borrow_mut_lamports()? += *account.try_borrow_lamports()?;

    // Resize and close account
    account.realloc(1, true)?;
    account.close()
}
