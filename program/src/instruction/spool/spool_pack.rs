use crate::api::prelude::*;
use brine_tree::Leaf;
use pinocchio::{account_info::AccountInfo, program_error::ProgramError, ProgramResult};
use tape_api::MAX_TAPES_PER_SPOOL;
use tape_api::{
    error::TapeError,
    state::{tape::Tape, Spool, TapeState},
    utils::check_condition,
};

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, shank::ShankType)]
pub struct Pack {
    pub value: [u8; 32],
}

impl DataLen for Pack {
    const LEN: usize = core::mem::size_of::<Pack>();
}

pub fn process_spool_pack(accounts: &[AccountInfo], data: &[u8]) -> ProgramResult {
    let pack_args = unsafe { load_ix_data::<Pack>(&data)? };

    let [signer_info, spool_info, tape_info, _remaining @ ..] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    if !signer_info.is_signer() {
        return Err(ProgramError::MissingRequiredSignature);
    }

    if !spool_info.is_owned_by(&tape_api::ID) {
        return Err(ProgramError::IncorrectProgramId);
    }

    let spool = Spool::unpack_mut(&mut spool_info.try_borrow_mut_data()?)?;

    if spool.authority != *signer_info.key() {
        return Err(ProgramError::MissingRequiredSignature);
    }

    if !tape_info.is_owned_by(&tape_api::ID) {
        return Err(ProgramError::IncorrectProgramId);
    }

    let tape = Tape::unpack_mut(&mut tape_info.try_borrow_mut_data()?)?;

    if tape.state != u64::from(TapeState::Finalized) {
        return Err(TapeError::UnexpectedState.into());
    }

    if !tape.number > 0 {
        return Err(TapeError::UnexpectedState.into());
    }

    check_condition(
        spool.total_tapes as usize <= MAX_TAPES_PER_SPOOL,
        TapeError::SpoolTooManyTapes,
    )?;

    let tape_id = tape.number.to_le_bytes();
    let tape = Leaf::new(&[tape_id.as_ref(), &pack_args.value]);

    spool.total_tapes += 1;

    Ok(())
}
