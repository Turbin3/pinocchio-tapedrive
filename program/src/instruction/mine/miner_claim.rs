use pinocchio::{
    account_info::AccountInfo,
    instruction::{Seed, Signer},
    program_error::ProgramError,
    ProgramResult,
};
use pinocchio_token::instructions::Transfer;
use tape_api::{
    consts::{MINT_ADDRESS, TREASURY, TREASURY_ADDRESS, TREASURY_ATA, TREASURY_BUMP},
    error::TapeError,
    state::Miner,
};

use crate::instruction::Claim;
use crate::utils::ByteConversion;

pub fn process_claim(accounts: &[AccountInfo], data: &[u8]) -> ProgramResult {
    // Parse instruction data
    let args = Claim::try_from_bytes(data)?;

    // Destructure accounts
    let [signer_info, beneficiary_info, miner_info, treasury_info, treasury_ata_info, token_program_info] =
        accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    // Validate signer
    if !signer_info.is_signer() {
        return Err(ProgramError::MissingRequiredSignature);
    }

    // Validate beneficiary
    if !beneficiary_info.is_writable() {
        return Err(ProgramError::InvalidAccountData);
    }

    // Validate beneficiary is owned by token program
    if beneficiary_info.owner() != &pinocchio_token::ID {
        return Err(ProgramError::IllegalOwner);
    }

    // Load beneficiary token account and verify mint
    let beneficiary_data = beneficiary_info.try_borrow_data()?;
    if beneficiary_data.len() != pinocchio_token::state::TokenAccount::LEN {
        return Err(ProgramError::InvalidAccountData);
    }

    // Check mint matches
    let beneficiary_mint = &beneficiary_data[0..32];
    if beneficiary_mint != MINT_ADDRESS.as_ref() {
        return Err(ProgramError::InvalidAccountData);
    }
    drop(beneficiary_data);

    // Load and validate miner account
    let mut miner_data = miner_info.try_borrow_mut_data()?;
    let miner = Miner::unpack_mut(&mut miner_data)?;

    // Check miner authority matches signer
    if miner.authority.ne(signer_info.key()) {
        return Err(ProgramError::MissingRequiredSignature);
    }

    // Validate treasury
    if treasury_info.key() != &TREASURY_ADDRESS {
        return Err(ProgramError::InvalidAccountData);
    }

    // Validate treasury ATA
    if !treasury_ata_info.is_writable() {
        return Err(ProgramError::InvalidAccountData);
    }

    if treasury_ata_info.key() != &TREASURY_ATA {
        return Err(ProgramError::InvalidAccountData);
    }

    // Validate token program
    if token_program_info.key() != &pinocchio_token::ID {
        return Err(ProgramError::IncorrectProgramId);
    }

    // Parse amount
    let mut amount = u64::from_le_bytes(args.amount);

    // If amount is zero, claim all unclaimed rewards
    if amount == 0 {
        amount = miner.unclaimed_rewards;
    }

    // Update miner balance with checked subtraction
    miner.unclaimed_rewards = miner
        .unclaimed_rewards
        .checked_sub(amount)
        .ok_or(TapeError::ClaimTooLarge)?;

    // Drop miner data before CPI
    drop(miner_data);

    // Transfer tokens from treasury ATA to beneficiary using PDA signer
    let bump_binding = [TREASURY_BUMP];
    let treasury_seeds = [Seed::from(TREASURY), Seed::from(&bump_binding)];
    let signer = [Signer::from(&treasury_seeds)];

    Transfer {
        from: treasury_ata_info,
        to: beneficiary_info,
        authority: treasury_info,
        amount,
    }
    .invoke_signed(&signer)?;

    Ok(())
}
