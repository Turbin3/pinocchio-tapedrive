use steel::*;
use super::AccountType;
use crate::state;
use crate::state::utils::{DataLen, Initialized, load_acc, load_acc_mut};
use pinocchio::program_error::ProgramError;

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
pub struct Block {
    pub number: u64,
    pub progress: u64,

    pub challenge: [u8; 32],
    pub challenge_set: u64,

    pub last_proof_at: i64,
    pub last_block_at: i64,
}

impl DataLen for Block {
    const LEN: usize = core::mem::size_of::<Block>();
}

impl Initialized for Block {
    fn is_initialized(&self) -> bool {
        true
    }
}


impl Block {
    pub fn unpack(data: &[u8]) -> Result<&Self, ProgramError> {
        unsafe { load_acc::<Block>(data) }
    }

    pub fn unpack_mut(data: &mut [u8]) -> Result<&mut Self, ProgramError> {
        unsafe { load_acc_mut::<Block>(data) }
    }
}
