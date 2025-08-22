use crate::state::*;
use pinocchio::{
    account_info::AccountInfo,
    program_error::ProgramError,
    pubkey::{find_program_address, Pubkey},
    ProgramResult,
};

trait AccountInfoExt {
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

enum GetPda {
    Metadata(Pubkey),
    Mint,
    Treasury,
}

impl GetPda {
    pub fn address(&self) -> (Pubkey, u8) {
        match self {
            GetPda::Mint => {
                find_program_address(&[b"mint", &[152, 68, 212, 200, 25, 113, 221, 71]], &TAPE_ID)
            }
            GetPda::Treasury => find_program_address(&[b"treasury"], &TAPE_ID),
            GetPda::Metadata(mint) => find_program_address(
                &[b"metadata", MPL_TOKEN_METADATA_ID.as_ref(), mint.as_ref()],
                &MPL_TOKEN_METADATA_ID,
            ),
        }
    }
}

pub fn process_initialize(accounts: &[AccountInfo], data: &[u8]) -> ProgramResult {
    if !data.is_empty() {
        return Err(ProgramError::InvalidInstructionData);
    }

    let [signer_info, archive_info, epoch_info, block_info, metadata_info, mint_info, treasury_info, treasury_ata_info, tape_info, writer_info, tape_program_info, system_program_info, token_program_info, associated_token_program_info, metadata_program_info, rent_sysvar_info, slot_hashes_info] =
        accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    archive_info.check_account(ARCHIVE)?;
    epoch_info.check_account(EPOCH)?;
    block_info.check_account(BLOCK)?;

    let (mint_address, mint_bump) = GetPda::Mint.address();
    let (treasury_address, treasury_bump) = GetPda::Treasury.address();
    let (metadata_address, _metadata_bump) = GetPda::Metadata(mint_address).address();

    assert_eq!(mint_bump, MINT_BUMP);
    assert_eq!(treasury_bump, TREASURY_BUMP);

    mint_info.check_account_with_address(&mint_address)?;
    metadata_info.check_account_with_address(&metadata_address)?;
    treasury_info.check_account_with_address(&treasury_address)?;

    if !treasury_ata_info.data_is_empty() {
        return Err(ProgramError::AccountAlreadyInitialized);
    }
    if !treasury_ata_info.is_writable() {
        return Err(ProgramError::Immutable);
    }

    tape_program_info.is_program_check()?;
    system_program_info.is_program_check()?;
    token_program_info.is_program_check()?;
    associated_token_program_info.is_program_check()?;
    metadata_program_info.is_program_check()?;
    rent_sysvar_info.is_program_check()?;
    slot_hashes_info.is_program_check()?;

    Ok(())
}
