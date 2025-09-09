use crate::api::state::{Miner, Spool};
use brine_tree::{verify, Leaf};
use pinocchio::{account_info::AccountInfo, program_error::ProgramError, ProgramResult};
use tape_api::{error::TapeError, utils::check_condition, SEGMENT_PROOF_LEN};
pub fn process_spool_commit(accounts: &[AccountInfo], data: &[u8]) -> ProgramResult {
    let args = Commit::try_from_bytes(data)?;
    let [signer_info, miner_info, spool_info, _remaining @ ..] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    if !signer_info.is_signer() {
        return Err(ProgramError::MissingRequiredSignature);
    }

    if !miner_info.is_owned_by(&tape_api::ID) {
        return Err(ProgramError::IncorrectProgramId);
    }

    let miner = Miner::unpack_mut(&mut miner_info.try_borrow_mut_data()?)?;
    if miner.authority != *signer_info.key() {
        return Err(ProgramError::MissingRequiredSignature);
    }

    if !spool_info.is_owned_by(&tape_api::ID) {
        return Err(ProgramError::IncorrectProgramId);
    }

    let spool = Spool::unpack(&spool_info.try_borrow_data()?)?;
    if spool.authority != *signer_info.key() {
        return Err(ProgramError::MissingRequiredSignature);
    }

    let merkle_root = &spool.contains;
    let merkle_proof = args.proof.as_ref();

    if merkle_proof.len() != SEGMENT_PROOF_LEN {
        return Err(ProgramError::InvalidInstructionData);
    }

    let leaf = Leaf::from(args.value);

    check_condition(
        verify(*merkle_root, merkle_proof, leaf),
        TapeError::SpoolCommitFailed,
    )?;

    miner.commitment = args.value;

    Ok(())
}
