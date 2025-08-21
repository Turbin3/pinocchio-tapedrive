use crate::state::miner::Miner;
use pinocchio::{
    account_info::AccountInfo,
    instruction::{Seed, Signer},
    program_error::ProgramError,
    pubkey::find_program_address,
    ProgramResult,
};
use pinocchio_token::instructions::*;

pub fn process_claim(accounts: &[AccountInfo], data: &[u8]) -> ProgramResult {
    pub const TREASURY: &[u8] = b"treasury";

    let [signer_acc, beneficiary_acc, proof_acc, treasury_acc, treasury_ata_acc, _token_program_acc] =
        accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    // We need to add a lot more checks for various accounts....
    // todo!();

    if !signer_acc.is_signer() {
        return Err(ProgramError::IncorrectAuthority);
    }

    // Fetching the amount from raw data bytes
    let mut amount = u64::from_le_bytes(data[1..9].try_into().unwrap());

    //Deserializing the miner account
    let miner = Miner::from_account_info_unchecked(proof_acc);

    //This was originally implemented this way in tapedrive. If amount is 0 which is default case then withdraw everything from unclaimed rewards

    if amount == 0 {
        amount = miner.unclaimed_rewards;
    }

    miner.unclaimed_rewards = miner
        .unclaimed_rewards
        .checked_sub(amount)
        .ok_or(ProgramError::ArithmeticOverflow)?;

    // Derive treasury PDA
    let (treasury_pda, _bump) = find_program_address(&[TREASURY], &crate::ID);
    if treasury_pda != *treasury_acc.key() {
        return Err(ProgramError::InvalidSeeds);
    }

    // Let's derive the signer seeds for p-token CPI
    let seeds = [Seed::from(TREASURY)];
    let signers = [Signer::from(&seeds)];

    // Do token transfer signed by treasury PDA
    Transfer {
        from: treasury_ata_acc,
        to: beneficiary_acc,
        authority: treasury_acc,
        amount,
    }
    .invoke_signed(&signers)?;

    Ok(())
}
