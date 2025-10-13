use crate::api::prelude::*;
use bytemuck::{try_from_bytes, Pod, Zeroable};
use pinocchio::{account_info::AccountInfo, program_error::ProgramError, ProgramResult};
use tape_api::{consts::TAPE_PROOF_LEN, error::TapeError, state::Spool, utils::check_condition};
use tape_utils::leaf::Leaf;

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, shank::ShankType, Pod, Zeroable)]
pub struct SpoolUnpackIxData {
    pub index: [u8; 8],
    pub proof: [[u8; 32]; TAPE_PROOF_LEN],
    pub value: [u8; 32],
}

impl DataLen for SpoolUnpackIxData {
    const LEN: usize = core::mem::size_of::<SpoolUnpackIxData>();
}

pub fn process_spool_unpack(accounts: &[AccountInfo], data: &[u8]) -> ProgramResult {
    if data.len() != SpoolUnpackIxData::LEN {
        return Err(ProgramError::InvalidInstructionData);
    }

    let unpack_args = try_from_bytes::<SpoolUnpackIxData>(data)
        .map_err(|_| ProgramError::InvalidInstructionData)?;

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

    let merkle_proof = unpack_args.proof.as_ref();

    if merkle_proof.len() != TAPE_PROOF_LEN {
        return Err(ProgramError::InvalidInstructionData);
    }

    let tape_id = unpack_args.index;
    let leaf = Leaf::new(&[tape_id.as_ref(), &unpack_args.value]);

    check_condition(
        spool.state.contains_leaf_no_std(merkle_proof, leaf),
        TapeError::SpoolUnpackFailed,
    )?;

    spool.contains = unpack_args.value;

    Ok(())
}
