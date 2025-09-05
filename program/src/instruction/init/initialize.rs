use crate::state::*;
use crate::utils::account_traits::AccountInfoExt;
use crate::utils::get_pda::GetPda;
use crate::utils::helpers::{create_program_account, SeedType};
use pinocchio::{account_info::AccountInfo, program_error::ProgramError, ProgramResult};

pub fn process_initialize(accounts: &[AccountInfo], _data: &[u8]) -> ProgramResult {
    // if !data.is_empty() {
    //     return Err(ProgramError::InvalidInstructionData);
    // }

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

    create_program_account::<Epoch>(
        archive_info,
        system_program_info,
        signer_info,
        &TAPE_ID,
        SeedType::Archive,
    )?;
    Ok(())
}
