use crate::state::TAPE_ID;
use pinocchio::program_error::ProgramError;
use pinocchio::pubkey::{find_program_address, Pubkey};
use pinocchio::{account_info::AccountInfo, ProgramResult};

pub trait AccountInfoExt {
    fn check_account(&self, seed: &[u8]) -> ProgramResult;
    fn check_account_with_address(&self, address: &Pubkey) -> ProgramResult;
    fn is_program_check(&self) -> ProgramResult;
}

impl AccountInfoExt for AccountInfo {
    fn check_account(&self, seed: &[u8]) -> ProgramResult {
        if !self.data_is_empty() {
            return Err(ProgramError::AccountAlreadyInitialized);
        }
        if !self.is_writable() {
            return Err(ProgramError::Immutable);
        }
        let (pda, _bump) = find_program_address(&[seed], &TAPE_ID);

        if self.key() != &pda {
            return Err(ProgramError::InvalidAccountData);
        }
        Ok(())
    }

    fn check_account_with_address(&self, address: &Pubkey) -> ProgramResult {
        if !self.data_is_empty() {
            return Err(ProgramError::AccountAlreadyInitialized);
        }
        if !self.is_writable() {
            return Err(ProgramError::Immutable);
        }

        if self.key().ne(address) {
            return Err(ProgramError::InvalidAccountData);
        }
        Ok(())
    }

    fn is_program_check(&self) -> ProgramResult {
        if self.key().ne(&TAPE_ID) {
            return Err(ProgramError::InvalidAccountData);
        }

        if !self.executable() {
            return Err(ProgramError::InvalidAccountData);
        }

        Ok(())
    }
}
