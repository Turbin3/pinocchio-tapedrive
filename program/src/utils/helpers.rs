use crate::state::*;
use crate::utils::AccountDiscriminator;
use bytemuck::Pod;
use pinocchio::sysvars::rent::Rent;
use pinocchio::sysvars::Sysvar;
use pinocchio::{
    account_info::AccountInfo,
    instruction::{Seed, Signer},
    pubkey::{find_program_address, Pubkey},
    ProgramResult,
};
use pinocchio_system::instructions::CreateAccount;

pub enum SeedType {
    Archive,
    Epoch,
    Block,
    Treasury,
}

impl SeedType {
    fn get_seeds(&self) -> &'static [&'static [u8]] {
        match self {
            SeedType::Archive => &[ARCHIVE],
            SeedType::Epoch => &[EPOCH],
            SeedType::Block => &[BLOCK],
            SeedType::Treasury => &[TREASURY],
        }
    }
}

pub fn create_program_account<T: AccountDiscriminator + Pod>(
    target_account: &AccountInfo,
    system_program: &AccountInfo,
    payer: &AccountInfo,
    owner: &Pubkey,
    seed_type: SeedType,
) -> ProgramResult {
    let seeds = seed_type.get_seeds();
    let (expected_address, bump) = find_program_address(seeds, owner);

    // Verify the target account has the expected address
    if target_account.key() != &expected_address {
        return Err(pinocchio::program_error::ProgramError::InvalidAccountData);
    }

    let space = 8 + core::mem::size_of::<T>();
    let rent = Rent::get()?;
    let lamports = rent.minimum_balance(space);

    // Create the seed array for signing - we need to include the bump
    let bump_slice = [bump];
    let base_seed = seed_type.get_seeds()[0];
    let seeds_array = [Seed::from(base_seed), Seed::from(bump_slice.as_slice())];

    let signers: &[Signer] = &[Signer::from(&seeds_array)];

    CreateAccount {
        from: payer,
        to: target_account,
        lamports,
        space: space as u64,
        owner: owner,
    }
    .invoke_signed(signers)?;

    // Set the discriminator
    let mut data = target_account.try_borrow_mut_data()?;
    data[0] = T::discriminator();

    Ok(())
}
