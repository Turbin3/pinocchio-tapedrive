//! Tape instruction builders for Pinocchio (no_std)
//!
//! These are simplified helpers for building instruction data for CPIs.
//! In Pinocchio, we typically invoke CPIs directly rather than building Instruction structs.

use crate::consts::*;
use crate::pda::*;
use crate::utils::to_name;
use bytemuck::{bytes_of, Pod, Zeroable};
use pinocchio::pubkey::Pubkey;

// Sysvar IDs (well-known addresses on Solana)
// Rent sysvar: SysvarRent111111111111111111111111111111111
const RENT_SYSVAR_ID: Pubkey = [
    6, 167, 213, 23, 24, 199, 116, 201, 40, 86, 99, 152, 105, 29, 94, 182, 139, 94, 184, 163, 155,
    75, 109, 92, 115, 85, 91, 33, 0, 0, 0, 0,
];

// SlotHashes sysvar: SysvarS1otHashes111111111111111111111111111
const SLOT_HASHES_SYSVAR_ID: Pubkey = [
    6, 167, 213, 23, 25, 44, 92, 81, 33, 140, 201, 76, 61, 74, 241, 127, 88, 218, 238, 8, 155, 161,
    253, 68, 227, 219, 217, 138, 0, 0, 0, 0,
];

// Re-export instruction data structures

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct Create {
    pub name: [u8; NAME_LEN],
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct Write {
    // Empty struct - actual data follows
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct Finalize {}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct Subsidize {
    pub amount: [u8; 8],
}

/// Instruction discriminators (must match TapeInstruction enum in program)
pub const DISCRIMINATOR_CREATE: u8 = 0x10;
pub const DISCRIMINATOR_WRITE: u8 = 0x11;
pub const DISCRIMINATOR_FINALIZE: u8 = 0x13;
pub const DISCRIMINATOR_SUBSIDIZE: u8 = 0x15;

/// Build instruction data for "create tape"
///
/// Returns: (instruction_data, tape_pda, writer_pda)
#[inline(always)]
pub fn build_create_ix_data(
    signer: &Pubkey,
    name: &str,
    data_buffer: &mut [u8],
) -> (usize, Pubkey, Pubkey) {
    let name_bytes = to_name(name);
    let (tape_address, _tape_bump) = tape_pda(*signer, &name_bytes);
    let (writer_address, _writer_bump) = writer_pda(tape_address);

    // Build instruction data: [discriminator | Create struct]
    let data_len = 1 + core::mem::size_of::<Create>();
    assert!(data_buffer.len() >= data_len, "Data buffer too small");

    data_buffer[0] = DISCRIMINATOR_CREATE;
    data_buffer[1..data_len].copy_from_slice(bytes_of(&Create { name: name_bytes }));

    (data_len, tape_address, writer_address)
}

/// Build instruction data for "write to tape"
///
/// Returns: instruction_data_length
#[inline(always)]
pub fn build_write_ix_data(write_data: &[u8], data_buffer: &mut [u8]) -> usize {
    let total_len = 1 + core::mem::size_of::<Write>() + write_data.len();
    assert!(data_buffer.len() >= total_len, "Data buffer too small");

    // Build instruction data: [discriminator | Write struct | actual data]
    data_buffer[0] = DISCRIMINATOR_WRITE;
    let write_struct_bytes = bytes_of(&Write {});
    data_buffer[1..1 + write_struct_bytes.len()].copy_from_slice(write_struct_bytes);
    data_buffer[1 + write_struct_bytes.len()..total_len].copy_from_slice(write_data);

    total_len
}

/// Build instruction data for "finalize tape"
///
/// Returns: instruction_data_length
#[inline(always)]
pub fn build_finalize_ix_data(data_buffer: &mut [u8]) -> usize {
    let data_len = 1 + core::mem::size_of::<Finalize>();
    assert!(data_buffer.len() >= data_len, "Data buffer too small");

    data_buffer[0] = DISCRIMINATOR_FINALIZE;
    data_buffer[1..data_len].copy_from_slice(bytes_of(&Finalize {}));

    data_len
}

/// Build instruction data for "subsidize tape"
///
/// Returns: instruction_data_length
#[inline(always)]
pub fn build_subsidize_ix_data(amount: u64, data_buffer: &mut [u8]) -> usize {
    let data_len = 1 + core::mem::size_of::<Subsidize>();
    assert!(data_buffer.len() >= data_len, "Data buffer too small");

    data_buffer[0] = DISCRIMINATOR_SUBSIDIZE;
    data_buffer[1..data_len].copy_from_slice(bytes_of(&Subsidize {
        amount: amount.to_le_bytes(),
    }));

    data_len
}

// Helper constants for account counts
pub const CREATE_ACCOUNTS_COUNT: usize = 6;
pub const WRITE_ACCOUNTS_COUNT: usize = 3;
pub const FINALIZE_ACCOUNTS_COUNT: usize = 6;
pub const SUBSIDIZE_ACCOUNTS_COUNT: usize = 5;

// Re-export commonly used constants
pub use crate::consts::{ARCHIVE_ADDRESS, TREASURY_ATA};
pub use pinocchio_system;
pub use pinocchio_token;

// Helper to get sysvar IDs
#[inline(always)]
pub fn get_rent_sysvar_id() -> &'static Pubkey {
    &RENT_SYSVAR_ID
}

#[inline(always)]
pub fn get_slot_hashes_sysvar_id() -> &'static Pubkey {
    &SLOT_HASHES_SYSVAR_ID
}
