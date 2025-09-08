use crate::state::constant::{TREASURY_ADDRESS, TREASURY_BUMP};
use crate::state::pda::mint_pda;
use bytemuck::try_from_bytes;
use bytemuck::{Pod, Zeroable};
use pinocchio::{
    account_info::AccountInfo,
    instruction::{Seed, Signer},
    program_error::ProgramError,
    ProgramResult,
};
use pinocchio_token::instructions::MintTo;

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
pub struct AirdropIx {
    pub amount: [u8; 8],
}

pub fn process_airdrop(accounts: &[AccountInfo], data: &[u8]) -> ProgramResult {
    let [signer_info, beneficiary_info, mint_info, treasury_info, _token_program_info] = accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    if data.len() < core::mem::size_of::<AirdropIx>() {
        return Err(ProgramError::InvalidInstructionData);
    }

    if !signer_info.is_signer() {
        return Err(ProgramError::MissingRequiredSignature);
    }

    let (mint_address, _mint_bump) = mint_pda();

    if !mint_info.is_writable() {
        return Err(ProgramError::Immutable);
    }

    if mint_info.key() != &mint_address {
        return Err(ProgramError::InvalidAccountData);
    }

    if treasury_info.key() != &TREASURY_ADDRESS {
        return Err(ProgramError::InvalidAccountData);
    }

    if !beneficiary_info.is_writable() {
        return Err(ProgramError::Immutable);
    }

    let ix_data =
        try_from_bytes::<AirdropIx>(data).map_err(|_| ProgramError::InvalidInstructionData)?;
    let amount = ix_data.amount;

    let binding = [TREASURY_BUMP];

    let treasury_seed = Seed::from("treasury".as_bytes());
    let bump_seed = Seed::from(&binding);

    let signer_seeds = &[treasury_seed, bump_seed];
    let signers: &[Signer] = &[Signer::from(&signer_seeds[..])];

    MintTo {
        mint: mint_info,
        account: beneficiary_info,
        mint_authority: treasury_info,
        amount: u64::from_le_bytes(amount),
    }
    .invoke_signed(signers)
    .map_err(|_| ProgramError::InvalidAccountData)?;

    Ok(())
}
