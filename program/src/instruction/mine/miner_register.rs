use pinocchio::{
    account_info::AccountInfo,
    instruction::{Seed, Signer},
    program_error::ProgramError,
    pubkey::{self, Pubkey},
    sysvars::{clock::Clock, rent::Rent, Sysvar},
    ProgramResult,
};

use pinocchio_system::instructions::CreateAccount;

use crate::state::utils::try_from_account_info_mut;

use crate::api::prelude::*;
use crate::api::state::utils::DataLen as ApiDataLen;

use crate::api::utils::compute_next_challenge;

use crate::state::utils::{load_ix_data, DataLen};

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, shank::ShankType)]
pub struct RegisterMinerIxData {
    pub name: [u8; 32],
}

impl DataLen for RegisterMinerIxData {
    const LEN: usize = core::mem::size_of::<RegisterMinerIxData>();
}

pub fn process_register(accounts: &[AccountInfo], data: &[u8]) -> ProgramResult {
    // Account order matches native: signer, miner, system_program, rent, slot_hashes
    let [signer_info, miner_info, _system_program_info, rent_info, slot_hashes_info, _remaining @ ..] =
        accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    if !signer_info.is_signer() {
        return Err(ProgramError::MissingRequiredSignature);
    }

    if !miner_info.data_is_empty() {
        return Err(ProgramError::AccountAlreadyInitialized);
    }

    let rent = Rent::from_account_info(rent_info)?;

    let ix_data = unsafe { load_ix_data::<RegisterMinerIxData>(&data)? };

    let seeds = &[MINER, signer_info.key().as_ref(), &ix_data.name[..]];
    let (miner_pda, miner_bump) = pubkey::find_program_address(seeds, &crate::ID);

    if miner_pda.ne(miner_info.key()) {
        return Err(ProgramError::InvalidAccountOwner);
    }

    let bump_binding = [miner_bump];
    let signer_seeds = [
        Seed::from(MINER),
        Seed::from(signer_info.key().as_ref()),
        Seed::from(&ix_data.name[..]),
        Seed::from(&bump_binding),
    ];
    let signers = [Signer::from(&signer_seeds[..])];

    CreateAccount {
        from: signer_info,
        to: miner_info,
        space: <Miner as ApiDataLen>::LEN as u64,
        owner: &crate::ID,
        lamports: rent.minimum_balance(<Miner as ApiDataLen>::LEN),
    }
    .invoke_signed(&signers)?;

    let next_challenge = compute_next_challenge(&miner_info.key(), &slot_hashes_info)?;

    // Initialize miner using API method
    Miner::initialize(
        miner_info,
        ix_data.name,
        (*signer_info.key()).into(),
        next_challenge,
    )?;

    // Update last_proof_at to current time to match native implementation
    let current_time = Clock::get()?.unix_timestamp;
    let mut miner_data = miner_info.try_borrow_mut_data()?;
    let miner = Miner::unpack_mut(&mut miner_data)?;
    miner.last_proof_at = current_time;

    Ok(())
}
