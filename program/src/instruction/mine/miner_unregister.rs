use pinocchio::{account_info::AccountInfo, program_error::ProgramError, ProgramResult};
use crate::state::utils::try_from_account_info;
use crate::state::miner::Miner;

pub fn process_unregister(accounts: &[AccountInfo], _data: &[u8]) -> ProgramResult {
    let [
        signer_info,
        miner_info,
        _system_program_info
    ] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys)
    };

    if !signer_info.is_signer(){
        return Err(ProgramError::MissingRequiredSignature);
    }

    if !miner_info.is_writable() {
        return Err(ProgramError::InvalidAccountData);
    }

    // todo : miner_info must be owned by tape_api::ID

    let miner = unsafe {
        try_from_account_info::<Miner>(miner_info)?
    };

    if miner.authority != *signer_info.key(){
        return Err(ProgramError::MissingRequiredSignature);
    }

    if miner.unclaimed_rewards != 0 {
        return Err(ProgramError::InvalidAccountData);
    }

    let amount = miner_info.lamports();
    {
        let mut signer_lamports = signer_info.try_borrow_mut_lamports()?;
        *signer_lamports = signer_lamports.saturating_add(amount);
    }

    {
        let mut miner_lamports = miner_info.try_borrow_mut_lamports()?;
        *miner_lamports = 0;
    }

    miner_info.close()?;

    Ok(())
}
