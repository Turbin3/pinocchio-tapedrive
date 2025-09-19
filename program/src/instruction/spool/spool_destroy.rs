use pinocchio::{account_info::AccountInfo, program_error::ProgramError, ProgramResult};
use tape_api::prelude::*;

pub fn process_spool_destroy(accounts: &[AccountInfo], _data: &[u8]) -> ProgramResult {
    let [signer_info, spool_info, _system_program_info, _remaining @ ..] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    if !signer_info.is_signer() {
        return Err(ProgramError::MissingRequiredSignature);
    }

    if !spool_info.is_writable() {
        return Err(ProgramError::Immutable);
    }

    if !spool_info.is_owned_by(&tape_api::ID) {
        return Err(ProgramError::IncorrectProgramId);
    }

    let spool_data = spool_info.try_borrow_data()?;
    let spool = Spool::unpack(&spool_data)?;

    if spool.authority != *signer_info.key() {
        return Err(ProgramError::MissingRequiredSignature);
    }

    *signer_info.try_borrow_mut_lamports()? += *spool_info.try_borrow_lamports()?;
    *spool_info.try_borrow_mut_lamports()? = 0;
    spool_info.close()?;

    Ok(())
}
