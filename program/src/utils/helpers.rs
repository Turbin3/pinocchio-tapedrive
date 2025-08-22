use bytemuck::Pod;
use pinocchio::sysvars::rent::Rent;
use pinocchio::{
    account_info::AccountInfo,
    instruction::{Seed, Signer},
    pubkey::{find_program_address, Pubkey},
    ProgramResult,
};
use pinocchio_system::instructions::CreateAccount;

pub trait Discriminator {
    fn discriminator() -> u8;
}

struct helpers {}

impl helpers {
    pub fn create_program_account<'a, T: Discriminator + Pod>(
        target_account: &'a AccountInfo,
        system_program: &'a AccountInfo,
        payer: &'a AccountInfo,
        owner: &Pubkey,
        seeds: &[&[u8]],
    ) -> ProgramResult {
        let bump = find_program_address(seeds, owner).1;
        let space = 8 + core::mem::size_of::<T>();
        // let total_seeds = [Seed, seeds.len() + 1];

        for seed in seeds {}

        let seeds = [Seed::from(b"seed")];
        let signers: &[Signer] = &[Signer::from(&seeds)];

        CreateAccount {
            from: payer,
            to: target_account,
            lamports: 100,
            space: space as u64,
            owner: owner,
        }
        .invoke_signed(signers);

        // let mut data = target_account.try_borrow_mut_data()?;
        // data[0] = T::discriminator();

        // let signer = [Signer::from(&seeds)];

        Ok(())
    }
}
