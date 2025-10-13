use crate::state::utils::{load_ix_data, DataLen};
use bytemuck::{Pod, Zeroable};
use pinocchio::{
    account_info::AccountInfo,
    instruction::{Seed, Signer},
    program_error::ProgramError,
    sysvars::{
        clock::Clock,
        rent::{Rent, RENT_ID},
        Sysvar,
    },
    ProgramResult,
};
use pinocchio_system::instructions::CreateAccount;
use tape_api::prelude::*;
use tape_api::state::utils::DataLen as ApiDataLen;

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, shank::ShankType, Pod, Zeroable)]
pub struct CreateSpoolIxData {
    pub number: u64,
}

impl DataLen for CreateSpoolIxData {
    const LEN: usize = core::mem::size_of::<CreateSpoolIxData>();
}

pub fn process_spool_create(accounts: &[AccountInfo], data: &[u8]) -> ProgramResult {
    let current_time = Clock::get()?.unix_timestamp;
    let [signer_info, miner_info, spool_info, _system_program_info, rent_info, _remaining @ ..] =
        accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    if !signer_info.is_signer() {
        return Err(ProgramError::MissingRequiredSignature);
    }

    if rent_info.key() != &RENT_ID {
        return Err(ProgramError::InvalidArgument);
    }

    if !spool_info.data_is_empty() {
        return Err(ProgramError::AccountAlreadyInitialized);
    }

    if !spool_info.is_writable() {
        return Err(ProgramError::Immutable);
    }

    if !miner_info.is_owned_by(&tape_api::ID) {
        return Err(ProgramError::IncorrectProgramId);
    }

    let miner_data = miner_info.try_borrow_data()?;
    let miner = Miner::unpack(&miner_data)?;

    if miner.authority != *signer_info.key() {
        return Err(ProgramError::MissingRequiredSignature);
    }

    let ix_data = unsafe { load_ix_data::<CreateSpoolIxData>(&data)? };

    let spool_number = ix_data.number;
    let (spool_pda, _spool_bump) = spool_pda(*miner_info.key(), spool_number);

    if spool_pda.ne(spool_info.key()) {
        return Err(ProgramError::InvalidAccountOwner);
    }

    let rent = Rent::from_account_info(rent_info)?;

    let spool_number_bytes = spool_number.to_le_bytes();
    let bump_binding = [_spool_bump];
    let signer_seeds = [
        Seed::from(SPOOL),
        Seed::from(miner_info.key().as_ref()),
        Seed::from(&spool_number_bytes),
        Seed::from(&bump_binding),
    ];
    let signers = [Signer::from(&signer_seeds[..])];

    CreateAccount {
        from: signer_info,
        to: spool_info,
        space: <Spool as ApiDataLen>::LEN as u64,
        owner: &crate::ID,
        lamports: rent.minimum_balance(<Spool as ApiDataLen>::LEN),
    }
    .invoke_signed(&signers)?;

    let mut spool_data = spool_info.try_borrow_mut_data()?;
    let spool = Spool::unpack_mut(&mut spool_data)?;

    spool.number = spool_number;
    spool.authority = *signer_info.key();
    spool.last_proof_at = current_time;
    spool.last_proof_block = 0;
    // spool.seed =
    spool.state = TapeTree::new(&[spool_info.key().as_ref()]);
    spool.contains = [0; 32];
    spool.total_tapes = 0;

    Ok(())
}
