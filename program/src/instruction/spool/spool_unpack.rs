use crate::api::prelude::*;
use brine_tree::Leaf;
use pinocchio::{account_info::AccountInfo, program_error::ProgramError, ProgramResult};
use tape_api::{state::Spool, utils::check_condition, SEGMENT_PROOF_LEN};
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, shank::ShankType)]
pub struct SpoolUnpackIxData {
    pub value: [u8; 32],
    pub proof: [u8; 32],
}

impl DataLen for SpoolUnpackIxData {
    const LEN: usize = core::mem::size_of::<SpoolUnpackIxData>();
}

pub fn process_spool_unpack(accounts: &[AccountInfo], data: &[u8]) -> ProgramResult {
    let unpack_args = unsafe { load_ix_data::<SpoolUnpackIxData>(&data)? };

    let [signer_info, spool_info, _remaining @ ..] = accounts else {
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

    let merkle_proof = unpack_args.proof;
    if merkle_proof.len() != TAPE_PROOF_LEN {
        return Err(ProgramError::InvalidInstructionData);
    }

    let tape_id = unpack_args.proof;

    let leaf = Leaf::new(&[tape_id.as_ref(), &unpack_args.value]);

    // check_condition(
    //     spool.state.contains_leaf(&merkle_proof, leaf),
    //     TapeError::SpoolUnpackFailed,
    // )?;

    spool.contains = unpack_args.value;

    Ok(())
}
