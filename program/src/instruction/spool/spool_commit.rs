use bytemuck::{try_from_bytes, Pod, Zeroable};
use pinocchio::{account_info::AccountInfo, program_error::ProgramError, ProgramResult};
use tape_api::prelude::*;
use tape_utils::{leaf::Leaf, tree::verify_no_std};

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, shank::ShankType, Pod, Zeroable)]
pub struct SpoolCommitIxData {
    pub value: [u8; 32],
    pub proof: [[u8; 32]; SEGMENT_PROOF_LEN],
}

impl DataLen for SpoolCommitIxData {
    const LEN: usize = core::mem::size_of::<SpoolCommitIxData>();
}

pub fn process_spool_commit(accounts: &[AccountInfo], data: &[u8]) -> ProgramResult {
    if data.len() != SpoolCommitIxData::LEN {
        return Err(ProgramError::InvalidInstructionData);
    }

    let commit_args = try_from_bytes::<SpoolCommitIxData>(data)
        .map_err(|_| ProgramError::InvalidInstructionData)?;
    let [signer_info, miner_info, spool_info, _remaining @ ..] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    if !signer_info.is_signer() {
        return Err(ProgramError::MissingRequiredSignature);
    }

    if !miner_info.is_owned_by(&tape_api::ID) {
        return Err(ProgramError::IncorrectProgramId);
    }

    let mut miner_data = miner_info.try_borrow_mut_data()?;
    let miner = Miner::unpack_mut(&mut miner_data)?;

    if miner.authority != *signer_info.key() {
        return Err(ProgramError::MissingRequiredSignature);
    }

    if !spool_info.is_owned_by(&tape_api::ID) {
        return Err(ProgramError::IncorrectProgramId);
    }

    let spool_data = spool_info.try_borrow_data()?;
    let spool = Spool::unpack(&spool_data)?;

    if spool.authority != *signer_info.key() {
        return Err(ProgramError::MissingRequiredSignature);
    }

    let merkle_root = &spool.contains;
    let merkle_proof = commit_args.proof.as_ref();

    if merkle_proof.len() != SEGMENT_PROOF_LEN {
        return Err(ProgramError::InvalidInstructionData);
    }

    let leaf = Leaf::from(commit_args.value);

    check_condition(
        verify_no_std(*merkle_root, merkle_proof, leaf),
        TapeError::SpoolCommitFailed,
    )?;

    miner.commitment = commit_args.value;

    Ok(())
}
