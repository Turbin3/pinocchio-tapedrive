use steel::*;
use super::AccountType;
use crate::state;
use crate::state::utils::{DataLen, Initialized, load_acc, load_acc_mut};
use pinocchio::program_error::ProgramError;

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
pub struct Archive {
    pub tapes_stored: u64,
    pub segments_stored: u64,
}

impl DataLen for Archive {
    const LEN: usize = core::mem::size_of::<Archive>();
}

impl Initialized for Archive {
    fn is_initialized(&self) -> bool {
        true
    }
}

impl Archive {
    pub fn unpack(data: &[u8]) -> Result<&Self, ProgramError> {
        unsafe { load_acc::<Archive>(data) }
    }

    pub fn unpack_mut(data: &mut [u8]) -> Result<&mut Self, ProgramError> {
        unsafe { load_acc_mut::<Archive>(data) }
    }
}
