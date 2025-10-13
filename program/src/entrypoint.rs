#![allow(unexpected_cfgs)]

use crate::instruction::*;
use pinocchio::{
    account_info::AccountInfo, default_allocator, default_panic_handler, program_entrypoint,
    program_error::ProgramError, pubkey::Pubkey, ProgramResult,
};

// This is the entrypoint for the program.
program_entrypoint!(process_instruction);

// Use Pinocchio's built-in bump allocator (enables heap allocation for String/Vec)
// This is needed for Borsh serialization in metadata CPI
default_allocator!();

// Use the no_std panic handler.
default_panic_handler!();

#[inline(always)]
fn process_instruction(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    let (discriminator, data) = instruction_data
        .split_first()
        .ok_or(ProgramError::InvalidInstructionData)?;

    let ix = TapeInstruction::try_from(discriminator)?;

    match ix {
        // ProgramInstruction variants
        TapeInstruction::Unknown => return Err(ProgramError::InvalidInstructionData),
        TapeInstruction::Initialize => process_initialize(accounts, data),
        TapeInstruction::Airdrop => process_airdrop(accounts, data),

        // TapeInstruction variants
        TapeInstruction::TapeCreate => process_tape_create(accounts, data),
        TapeInstruction::TapeWrite => process_tape_write(accounts, data),
        TapeInstruction::TapeUpdate => process_tape_update(accounts, data),
        TapeInstruction::TapeFinalize => process_tape_finalize(accounts, data),
        TapeInstruction::TapeSetHeader => process_tape_set_header(accounts, data),
        TapeInstruction::TapeSubsidize => process_tape_subsidize_rent(accounts, data),

        // MinerInstruction variants
        TapeInstruction::MinerRegister => process_register(accounts, data),
        TapeInstruction::MinerUnregister => process_unregister(accounts, data),
        TapeInstruction::MinerMine => process_mine(accounts, data),
        TapeInstruction::MinerClaim => process_claim(accounts, data),

        // SpoolInstruction variants
        TapeInstruction::SpoolCreate => process_spool_create(accounts, data),
        TapeInstruction::SpoolDestroy => process_spool_destroy(accounts, data),
        TapeInstruction::SpoolPack => process_spool_pack(accounts, data),
        TapeInstruction::SpoolUnpack => process_spool_unpack(accounts, data),
        TapeInstruction::SpoolCommit => process_spool_commit(accounts, data),
    }
}
