use steel::*;
use super::AccountType;
use crate::state;
use crate::state::utils::{DataLen, Initialized, load_acc, load_acc_mut};
use pinocchio::program_error::ProgramError;

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
pub struct Epoch {
    pub number: u64,
    pub progress: u64,

    pub mining_difficulty: u64,
    pub packing_difficulty: u64,
    pub target_participation: u64,
    pub reward_rate: u64,
    pub duplicates: u64,

    pub last_epoch_at: i64,
}

impl DataLen for Epoch {
    const LEN: usize = core::mem::size_of::<Epoch>();
}

impl Initialized for Epoch {
    fn is_initialized(&self) -> bool {
        true
    }
}


impl Epoch {
    pub fn unpack(data: &[u8]) -> Result<&Self, ProgramError> {
        unsafe { load_acc::<Epoch>(data) }
    }

    pub fn unpack_mut(data: &mut [u8]) -> Result<&mut Self, ProgramError> {
        unsafe { load_acc_mut::<Epoch>(data) }
    }
}