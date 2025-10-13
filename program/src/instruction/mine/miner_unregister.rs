use pinocchio::{account_info::AccountInfo, program_error::ProgramError, ProgramResult};
use tape_api::prelude::*;

pub fn process_unregister(accounts: &[AccountInfo], _data: &[u8]) -> ProgramResult {
    // Destructure accounts array
    let [signer_info, miner_info, system_program_info] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    // Validate signer
    if !signer_info.is_signer() {
        return Err(ProgramError::MissingRequiredSignature);
    }

    // Load and validate miner account
    let miner_data = miner_info.try_borrow_data()?;
    let miner = Miner::unpack(&miner_data)?;

    // Check miner authority matches signer
    if miner.authority.ne(signer_info.key()) {
        return Err(ProgramError::MissingRequiredSignature);
    }

    // Check unclaimed rewards are zero
    if miner.unclaimed_rewards != 0 {
        return Err(ProgramError::InvalidAccountData);
    }

    // Drop miner data borrow before closing
    drop(miner_data);

    // Validate system program
    if system_program_info.key().ne(&pinocchio_system::ID) {
        return Err(ProgramError::IncorrectProgramId);
    }

    // Validate miner account is writable
    if !miner_info.is_writable() {
        return Err(ProgramError::InvalidAccountData);
    }

    // Validate miner account owner is this program
    if miner_info.owner().ne(&tape_api::id()) {
        return Err(ProgramError::IllegalOwner);
    }

    // Close the miner account and return rent to signer
    close_miner_account(miner_info, signer_info)?;

    Ok(())
}

/// Close miner account and return rent to destination
#[inline(always)]
fn close_miner_account(account: &AccountInfo, destination: &AccountInfo) -> ProgramResult {
    // Set first byte to 0xff to prevent reinitialization attacks
    {
        let mut data = account.try_borrow_mut_data()?;
        if !data.is_empty() {
            data[0] = 0xff;
        }
    }

    // Transfer all lamports to destination
    *destination.try_borrow_mut_lamports()? += *account.try_borrow_lamports()?;

    // Resize and close account
    account.realloc(1, true)?;
    account.close()
}
