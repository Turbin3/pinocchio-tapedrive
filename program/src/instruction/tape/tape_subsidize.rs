use pinocchio::{account_info::AccountInfo, program_error::ProgramError, ProgramResult};
use pinocchio_token::instructions::Transfer;
use tape_api::prelude::*;

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, shank::ShankType)]
pub struct SubsidizeIxData {
    pub amount: u64,
}

impl DataLen for SubsidizeIxData {
    const LEN: usize = core::mem::size_of::<SubsidizeIxData>();
}

pub fn process_tape_subsidize_rent(accounts: &[AccountInfo], data: &[u8]) -> ProgramResult {
    let [signer_info, ata_info, tape_info, treasury_ata_info, _token_program_info, remaining @ ..] =
        accounts
    else {
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

    if !treasury_ata_info.is_writable() {
        return Err(ProgramError::Immutable);
    }

    let args = unsafe { load_ix_data::<SubsidizeIxData>(&data)? };

    let amount = args.amount;

    Transfer {
        from: &ata_info,
        to: &treasury_ata_info,
        authority: &signer_info,
        amount,
    }
    .invoke()?;

    tape.balance = tape.balance.saturating_add(amount);

    Ok(())
}
