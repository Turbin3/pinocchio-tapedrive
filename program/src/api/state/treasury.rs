use steel::*;
use super::AccountType;
use crate::state;
use crate::state::utils::{DataLen, Initialized, load_acc, load_acc_mut};
use pinocchio::program_error::ProgramError;

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
pub struct Treasury {}

impl DataLen for Treasury {
    const LEN: usize = core::mem::size_of::<Treasury>();
}

impl Initialized for Treasury {
    fn is_initialized(&self) -> bool {
        true
    }
}

impl Treasury {
    pub fn unpack(data: &[u8]) -> Result<&Self, ProgramError> {
        unsafe { load_acc::<Treasury>(data) }
    }
    pub fn unpack_mut(data: &mut [u8]) -> Result<&mut Self, ProgramError> {
        unsafe { load_acc_mut::<Treasury>(data) }
    }
}