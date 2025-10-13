use crate::utils::AccountDiscriminator;
use bytemuck::Pod;
use pinocchio::program_error::ProgramError;
use pinocchio::sysvars::rent::Rent;
use pinocchio::sysvars::Sysvar;
use pinocchio::{
    account_info::AccountInfo,
    instruction::{Seed, Signer},
    pubkey::{find_program_address, Pubkey},
    ProgramResult,
};
use pinocchio_system::instructions::CreateAccount;

/// Creates a new program account (PDA) with discriminator.
///
/// This is equivalent to Steel's `create_program_account`:
/// - Derives PDA from seeds
/// - Allocates space: 8 bytes (discriminator) + size_of::<T>()
/// - Creates account via CPI to system program
/// - Sets the first byte to T::discriminator()
///
/// # Example
/// ```rust
/// create_program_account::<Epoch>(
///     epoch_info,
///     system_program_info,
///     signer_info,
///     &tape_api::ID,
///     &[EPOCH],
/// )?;
/// ```
#[inline(always)]
pub fn create_program_account<T: AccountDiscriminator + Pod>(
    target_account: &AccountInfo,
    _system_program: &AccountInfo,
    payer: &AccountInfo,
    owner: &Pubkey,
    seeds: &[&[u8]],
) -> ProgramResult {
    // Find the PDA and bump
    let (expected_address, bump) = find_program_address(seeds, owner);

    // Verify the target account has the expected address
    if target_account.key() != &expected_address {
        return Err(pinocchio::program_error::ProgramError::InvalidAccountData);
    }

    // Calculate space: 8 bytes for discriminator + struct size
    let space = 8 + core::mem::size_of::<T>();
    let rent = Rent::get()?;
    let lamports = rent.minimum_balance(space);

    // Build signer seeds: original seeds + bump
    // Bind bump and seeds arrays at this scope so they live long enough
    let bump_slice = [bump];

    // Pattern from PINOCCHIO_PATTERNS.md - create seed bindings outside match
    match seeds.len() {
        1 => {
            let seeds_array = [Seed::from(seeds[0]), Seed::from(bump_slice.as_slice())];
            let signer = [Signer::from(&seeds_array)];

            CreateAccount {
                from: payer,
                to: target_account,
                lamports,
                space: space as u64,
                owner,
            }
            .invoke_signed(&signer)?;
        }
        2 => {
            let seeds_array = [
                Seed::from(seeds[0]),
                Seed::from(seeds[1]),
                Seed::from(bump_slice.as_slice()),
            ];
            let signer = [Signer::from(&seeds_array)];

            CreateAccount {
                from: payer,
                to: target_account,
                lamports,
                space: space as u64,
                owner,
            }
            .invoke_signed(&signer)?;
        }
        3 => {
            let seeds_array = [
                Seed::from(seeds[0]),
                Seed::from(seeds[1]),
                Seed::from(seeds[2]),
                Seed::from(bump_slice.as_slice()),
            ];
            let signer = [Signer::from(&seeds_array)];

            CreateAccount {
                from: payer,
                to: target_account,
                lamports,
                space: space as u64,
                owner,
            }
            .invoke_signed(&signer)?;
        }
        4 => {
            let seeds_array = [
                Seed::from(seeds[0]),
                Seed::from(seeds[1]),
                Seed::from(seeds[2]),
                Seed::from(seeds[3]),
                Seed::from(bump_slice.as_slice()),
            ];
            let signer = [Signer::from(&seeds_array)];

            CreateAccount {
                from: payer,
                to: target_account,
                lamports,
                space: space as u64,
                owner,
            }
            .invoke_signed(&signer)?;
        }
        _ => return Err(pinocchio::program_error::ProgramError::InvalidSeeds),
    };

    // Set the discriminator (first byte)
    let mut data = target_account.try_borrow_mut_data()?;
    data[0] = T::discriminator();

    Ok(())
}

// NOTE: Due to borrow checker limitations, we use a macro instead of a function
// for getting mutable account data. This keeps the RefMut alive in the caller's scope.

/// Safely cast account data to struct using bytemuck (no unsafe!).
///
/// Usage:
/// ```rust
/// let mut data = account.try_borrow_mut_data()?;
/// let account_struct = cast_account_data_mut::<Epoch>(&mut data)?;
/// account_struct.number = 1;
/// ```
#[inline(always)]
pub fn cast_account_data_mut<T: Pod>(data: &mut [u8]) -> Result<&mut T, ProgramError> {
    // Validate length: 8 bytes for discriminator + struct size
    let expected_len = 8 + core::mem::size_of::<T>();
    if data.len() != expected_len {
        return Err(ProgramError::InvalidAccountData);
    }

    // Safe cast using bytemuck (no unsafe!)
    bytemuck::try_from_bytes_mut::<T>(&mut data[8..]).map_err(|_| ProgramError::InvalidAccountData)
}
