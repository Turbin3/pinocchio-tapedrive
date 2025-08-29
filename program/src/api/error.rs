use steel::*;
use pinocchio::program_error::ProgramError;

#[repr(u32)]
#[derive(Clone, PartialEq, shank::ShankType)]
pub enum TapeError {
    // Unknown error
    UnknownError = 0,

    // The provided tape is in an unexpected state
    UnexpectedState         = 0x10,
    // The tape write failed
    WriteFailed             = 0x11,
    // The tape is too long
    TapeTooLong             = 0x12,
    // The tape does not have enough rent
    InsufficientRent        = 0x13,

    // The provided hash is invalid
    SolutionInvalid         = 0x20,
    // The provided tape doesn't match the expected tape
    UnexpectedTape          = 0x21,
    // The provided hash did not satisfy the minimum required difficulty
    SolutionTooEasy         = 0x22,
    // The provided solution is too early
    SolutionTooEarly        = 0x23,
    // The provided claim is too large
    ClaimTooLarge           = 0x24,
    // Computed commitment does not match the miner commitment
    CommitmentMismatch      = 0x25,

    // Faild to pack the tape into the spool
    SpoolPackFailed         = 0x30,
    // Failed to unpack the tape from the spool
    SpoolUnpackFailed       = 0x31,
    // Too many tapes in the spool
    SpoolTooManyTapes       = 0x32,
    // Spool commit failed
    SpoolCommitFailed       = 0x33,
}

impl From<TapeError> for ProgramError {
    fn from(e: TapeError) -> Self {
        Self::Custom(e as u32)
    }
}
