use crate::api::prelude::*;
use bytemuck::{try_from_bytes, Pod, Zeroable};
use pinocchio::{account_info::AccountInfo, program_error::ProgramError, ProgramResult};
use tape_api::{
    error::TapeError,
    state::{Spool, TapeState},
    utils::check_condition,
    MAX_TAPES_PER_SPOOL,
};
use tape_utils::leaf::Leaf;

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, shank::ShankType, Pod, Zeroable)]
pub struct Pack {
    pub value: [u8; 32],
}

impl DataLen for Pack {
    const LEN: usize = core::mem::size_of::<Pack>();
}

pub fn process_spool_pack(accounts: &[AccountInfo], data: &[u8]) -> ProgramResult {
    let pack_args =
        try_from_bytes::<Pack>(data).map_err(|_| ProgramError::InvalidInstructionData)?;

    let [signer_info, spool_info, tape_info, _remaining @ ..] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    if !signer_info.is_signer() {
        return Err(ProgramError::MissingRequiredSignature);
    }

    if !spool_info.is_owned_by(&tape_api::ID) {
        return Err(ProgramError::IncorrectProgramId);
    }

    let mut spool_data = spool_info.try_borrow_mut_data()?;
    let spool = Spool::unpack_mut(&mut spool_data)?;

    if spool.authority != *signer_info.key() {
        return Err(ProgramError::MissingRequiredSignature);
    }

    if !tape_info.is_owned_by(&tape_api::ID) {
        return Err(ProgramError::IncorrectProgramId);
    }

    let mut tape_data = tape_info.try_borrow_mut_data()?;
    let tape = Tape::unpack_mut(&mut tape_data)?;

    if tape.state != (TapeState::Finalized as u64) {
        return Err(TapeError::UnexpectedState.into());
    }

    if tape.number == 0 {
        return Err(TapeError::UnexpectedState.into());
    }

    check_condition(
        spool.total_tapes as usize <= MAX_TAPES_PER_SPOOL,
        TapeError::SpoolTooManyTapes,
    )?;

    let tape_id = tape.number.to_le_bytes();
    let leaf = Leaf::new(&[tape_id.as_ref(), &pack_args.value]);

    check_condition(
        spool.state.try_add_leaf(leaf).is_ok(),
        TapeError::SpoolPackFailed,
    )?;

    spool.total_tapes += 1;

    Ok(())
}
