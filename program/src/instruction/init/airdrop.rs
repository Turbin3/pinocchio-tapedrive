use pinocchio::{
    account_info::AccountInfo, 
    instruction::{Seed, Signer},
    program_error::ProgramError, 
    ProgramResult,
};
use crate::state::utils::{DataLen, load_ix_data};
use crate::state::pda::{get_mint_pda, get_treasury_pda};
use pinocchio_token::instructions::MintTo;
use pinocchio_token::state::TokenAccount;

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AirdropIx {
    pub amount: [u8; 8],
    pub bump: u8,
}

impl DataLen for AirdropIx {
    const LEN: usize = core::mem::size_of::<AirdropIx>();
}

pub fn process_airdrop(accounts: &[AccountInfo], data: &[u8]) -> ProgramResult {
    let [
        signer_info,
        beneficiary_info,
        mint_info,
        treasury_info,
        _token_program_info,
    ] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    if data.len() < core::mem::size_of::<AirdropIx>() {
        return Err(ProgramError::InvalidInstructionData);
    }

    if !signer_info.is_signer() {
        return Err(ProgramError::MissingRequiredSignature);
    }

    let (mint_address, _mint_bump) = get_mint_pda();
    let (treasury_address, treasury_bump) = get_treasury_pda();

    if mint_info.key() != &mint_address {
        return Err(ProgramError::InvalidAccountData)
    }

    if treasury_info.key() != &treasury_address {
        return Err(ProgramError::InvalidAccountData)
    }

    let ix_data = unsafe { load_ix_data::<AirdropIx>(data)? };
    let amount = ix_data.amount;

    let treasury_token_account = unsafe {
        TokenAccount::from_account_info_unchecked(treasury_info).map_err(|_| ProgramError::InvalidAccountData)?
    };

    if treasury_token_account.amount() < u64::from_le_bytes(amount) {
        return Err(ProgramError::InsufficientFunds);
    }

    let beneficiary_token_account = unsafe {
        TokenAccount::from_account_info_unchecked(beneficiary_info).map_err(|_| ProgramError::InvalidAccountData)?
    };

    if beneficiary_token_account.mint() != mint_info.key() {
        return Err(ProgramError::InvalidAccountData);
    }

    let binding = [treasury_bump];

    let treasury_seed = Seed::from("treasury".as_bytes());
    let bump_seed = Seed::from(&binding);

    let signer_seeds = &[treasury_seed, bump_seed];
    let signers = [Signer::from(&signer_seeds[..])];

    MintTo{
        mint: &mint_info,
        account: &beneficiary_info,
        mint_authority: &treasury_info,
        amount: u64::from_le_bytes(ix_data.amount)
    }.invoke_signed(&signers)?;


    Ok(())
}
