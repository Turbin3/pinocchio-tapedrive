use pinocchio::{account_info::AccountInfo, program_error::ProgramError, ProgramResult};
use pinocchio_token::instructions::Transfer;
use tape_api::{consts::TREASURY_ATA, state::Tape};

use crate::instruction::Subsidize;
use crate::utils::ByteConversion;

pub fn process_tape_subsidize_rent(accounts: &[AccountInfo], data: &[u8]) -> ProgramResult {
    let args = Subsidize::try_from_bytes(data)?;

    let [signer_info, ata_info, tape_info, treasury_ata_info, _token_program_info] = accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    // Validate signer
    if !signer_info.is_signer() {
        return Err(ProgramError::MissingRequiredSignature);
    }

    // Load tape account
    let mut tape_data = tape_info.try_borrow_mut_data()?;
    let tape = Tape::unpack_mut(&mut tape_data)?;

    // Validate treasury ATA address
    if treasury_ata_info.key().ne(&TREASURY_ATA) {
        return Err(ProgramError::InvalidAccountData);
    }

    // Validate treasury ATA is writable
    if !treasury_ata_info.is_writable() {
        return Err(ProgramError::InvalidAccountData);
    }

    // Parse amount
    let amount = u64::from_le_bytes(args.amount);

    // Transfer tokens from signer's ATA to treasury ATA
    Transfer {
        from: ata_info,
        to: treasury_ata_info,
        authority: signer_info,
        amount,
    }
    .invoke()?;

    // Update tape balance
    tape.balance = tape.balance.saturating_add(amount);

    Ok(())
}
