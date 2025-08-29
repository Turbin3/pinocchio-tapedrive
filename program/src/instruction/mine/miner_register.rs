use pinocchio::{
    account_info::AccountInfo,
    instruction::{Seed, Signer},
    program_error::ProgramError,
    pubkey::{self, Pubkey},
    ProgramResult,
    sysvars::rent::Rent,
};

use pinocchio_system::instructions::CreateAccount;

use crate::state::utils::try_from_account_info_mut;

use crate::api::prelude::*;

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
    let [
        signer_info,
        miner_info,
        system_program_info, 
        rent_info,
        slot_hashes_info,
        _remaining @ ..,
    ] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    if signer_info.is_signer() {
        return Err(ProgramError::MissingRequiredSignature);
    }

    if !miner_info.data_is_empty() {
        return Err(ProgramError::AccountAlreadyInitialized);
    }

    let rent = Rent::from_account_info(rent_info)?;

    let ix_data = unsafe { load_ix_data::<RegisterMinerIxData>(&data)? };

    let seeds = &[b"miner_account", &ix_data.name[..]];
    let (miner_pda, miner_bump) = pubkey::find_program_address(seeds, &crate::ID);
    
    if miner_pda.ne(miner_info.key()) {
        return Err(ProgramError::InvalidAccountOwner);
    }

    let rent = Rent::from_account_info(rent_info)?;

    let bump_binding = [miner_bump];
    let signer_seeds = [
        Seed::from(b"miner_account"),
        Seed::from(&ix_data.name[..]),
        Seed::from(&bump_binding),
    ];
    let signers = [Signer::from(&signer_seeds[..])];

    CreateAccount {
        from: signer_info,
        to: miner_info,
        space: Miner::LEN as u64,
        owner: &crate::ID,
        lamports: rent.minimum_balance(Miner::LEN),
    }
    .invoke_signed(&signers)?;

    let next_challenge = compute_next_challenge(
        &miner_info.key(),
        slot_hashes_info,
    )?;

    Miner::initialize(
        miner_info,
        ix_data.name,
        (*signer_info.key()).into(),
        next_challenge
    )?;

    Ok(())
}