use {
    bytemuck::{Pod, Zeroable},
    pinocchio::program_error::ProgramError,
    tape_api::consts::{HEADER_SIZE, NAME_LEN, SEGMENT_SIZE},
    tape_api::types::ProofPath,
};

pub mod init;
pub mod mine;
pub mod spool;
pub mod tape;

pub use {init::*, mine::*, spool::*, tape::*};

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct Create {
    pub name: [u8; NAME_LEN],
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct Write {
    // Phantom Vec<u8> to ensure the size is dynamic
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct Update {
    pub segment_number: [u8; 8],
    pub old_data: [u8; SEGMENT_SIZE],
    pub new_data: [u8; SEGMENT_SIZE],
    pub proof: ProofPath,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct Finalize {}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct SetHeader {
    pub header: [u8; HEADER_SIZE],
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct Subsidize {
    pub amount: [u8; 8],
}

#[repr(u8)]
pub enum TapeInstruction {
    // ProgramInstruction variants
    Unknown = 0,
    Initialize = 1, // ProgramInstruction::Initialize
    Airdrop = 2,    // ProgramInstruction::Airdrop

    // TapeInstruction variants
    TapeCreate = 0x10,    // TapeInstruction::Create = 0x10
    TapeWrite = 0x11,     // TapeInstruction::Write
    TapeUpdate = 0x12,    // TapeInstruction::Update
    TapeFinalize = 0x13,  // TapeInstruction::Finalize
    TapeSetHeader = 0x14, // TapeInstruction::SetHeader
    TapeSubsidize = 0x15, // TapeInstruction::Subsidize

    // MinerInstruction variants
    MinerRegister = 0x20,   // MinerInstruction::Register = 0x20
    MinerUnregister = 0x21, // MinerInstruction::Unregister
    MinerMine = 0x22,       // MinerInstruction::Mine
    MinerClaim = 0x23,      // MinerInstruction::Claim

    // SpoolInstruction variants
    SpoolCreate = 0x40,  // SpoolInstruction::Create = 0x40
    SpoolDestroy = 0x41, // SpoolInstruction::Destroy
    SpoolPack = 0x42,    // SpoolInstruction::Pack
    SpoolUnpack = 0x43,  // SpoolInstruction::Unpack
    SpoolCommit = 0x44,  // SpoolInstruction::Commit
}

impl TryFrom<&u8> for TapeInstruction {
    type Error = ProgramError;

    fn try_from(value: &u8) -> Result<Self, Self::Error> {
        match *value {
            // ProgramInstruction variants
            0 => Ok(TapeInstruction::Unknown),
            1 => Ok(TapeInstruction::Initialize),
            2 => Ok(TapeInstruction::Airdrop),

            // TapeInstruction variants
            0x10 => Ok(TapeInstruction::TapeCreate),
            0x11 => Ok(TapeInstruction::TapeWrite),
            0x12 => Ok(TapeInstruction::TapeUpdate),
            0x13 => Ok(TapeInstruction::TapeFinalize),
            0x14 => Ok(TapeInstruction::TapeSetHeader),
            0x15 => Ok(TapeInstruction::TapeSubsidize),

            // MinerInstruction variants
            0x20 => Ok(TapeInstruction::MinerRegister),
            0x21 => Ok(TapeInstruction::MinerUnregister),
            0x22 => Ok(TapeInstruction::MinerMine),
            0x23 => Ok(TapeInstruction::MinerClaim),

            // SpoolInstruction variants
            0x40 => Ok(TapeInstruction::SpoolCreate),
            0x41 => Ok(TapeInstruction::SpoolDestroy),
            0x42 => Ok(TapeInstruction::SpoolPack),
            0x43 => Ok(TapeInstruction::SpoolUnpack),
            0x44 => Ok(TapeInstruction::SpoolCommit),

            _ => Err(ProgramError::InvalidInstructionData),
        }
    }
}

// mod idl_gen {
//     use super::InitializeMyStateV1IxData;

//     #[derive(shank::ShankInstruction)]
//     enum _MyProgramInstruction {
//         #[account(0, writable, signer, name = "payer_acc", desc = "Fee payer account")]
//         #[account(1, writable, name = "state_acc", desc = "New State account")]
//         #[account(2, name = "sysvar_rent_acc", desc = "Sysvar rent account")]
//         #[account(3, name = "system_program_acc", desc = "System program account")]
//         InitializeState(InitializeMyStateV1IxData),
//         #[account(0, writable, signer, name = "payer_acc", desc = "Fee payer account")]
//         #[account(1, writable, name = "state_acc", desc = "State account")]
//         UpdateState,
//     }
// }
