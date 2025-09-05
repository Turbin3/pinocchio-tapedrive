use crate::consts::*;
use pinocchio::{account_info::AccountInfo, program_error::ProgramError, pubkey::Pubkey};

pub trait AccountInfoLoader {
    fn is_archive(&self) -> Result<&Self, ProgramError>;
    fn is_epoch(&self) -> Result<&Self, ProgramError>;
    fn is_block(&self) -> Result<&Self, ProgramError>;
    fn is_treasury(&self) -> Result<&Self, ProgramError>;
    fn is_treasury_ata(&self) -> Result<&Self, ProgramError>;
}

impl AccountInfoLoader for AccountInfo {
    fn is_archive(&self) -> Result<&Self, ProgramError> {
        if self.key() != &ARCHIVE_ADDRESS {
            return Err(ProgramError::InvalidAccountData);
        }
        if self.owner() != &crate::ID {
            return Err(ProgramError::IllegalOwner);
        }
        Ok(self)
    }

    fn is_epoch(&self) -> Result<&Self, ProgramError> {
        if self.key() != &EPOCH_ADDRESS {
            return Err(ProgramError::InvalidAccountData);
        }
        if self.owner() != &crate::ID {
            return Err(ProgramError::IllegalOwner);
        }
        Ok(self)
    }

    fn is_block(&self) -> Result<&Self, ProgramError> {
        if self.key() != &BLOCK_ADDRESS {
            return Err(ProgramError::InvalidAccountData);
        }
        if self.owner() != &crate::ID {
            return Err(ProgramError::IllegalOwner);
        }
        Ok(self)
    }

    fn is_treasury(&self) -> Result<&Self, ProgramError> {
        if self.key() != &TREASURY_ADDRESS {
            return Err(ProgramError::InvalidAccountData);
        }
        if self.owner() != &crate::ID {
            return Err(ProgramError::IllegalOwner);
        }
        Ok(self)
    }

    fn is_treasury_ata(&self) -> Result<&Self, ProgramError> {
        if self.key() != &TREASURY_ATA {
            return Err(ProgramError::InvalidAccountData);
        }
        Ok(self)
    }
}
