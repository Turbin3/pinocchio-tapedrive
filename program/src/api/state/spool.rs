use steel::*;
use crate::state;
use crate::api::types::*;
use super::AccountType;
use crate::state::utils::{DataLen, Initialized, load_acc, load_acc_mut};
use pinocchio::program_error::ProgramError;

#[repr(C)] 
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
pub struct Spool {
    pub number: u64,

    pub authority: Pubkey,
    pub state:     TapeTree, 
    pub seed:      [u8; 32],
    pub contains:  [u8; 32], 

    pub total_tapes: u64,

    pub last_proof_block: u64,
    pub last_proof_at: i64,
}

impl DataLen for Spool {
    const LEN: usize = core::mem::size_of::<Spool>();
}

impl Initialized for Spool {
    fn is_initialized(&self) -> bool {
        true
    }
}

impl Spool {
    pub fn unpack(data: &[u8]) -> Result<&Self, ProgramError> {
        unsafe { load_acc::<Spool>(data) }
    }
    pub fn unpack_mut(data: &mut [u8]) -> Result<&mut Self, ProgramError> {
        unsafe { load_acc_mut::<Spool>(data) }
    }
}
