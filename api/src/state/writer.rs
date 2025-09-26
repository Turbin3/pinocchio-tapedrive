use crate::state::utils::{load_acc, load_acc_mut, DataLen, Initialized};
use crate::types::SegmentTree;

use bytemuck::{Pod, Zeroable};
use pinocchio::{program_error::ProgramError, pubkey::Pubkey};

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
pub struct Writer {
    pub tape: Pubkey,
    pub state: SegmentTree,
}

impl DataLen for Writer {
    const LEN: usize = core::mem::size_of::<Writer>();
}

impl Initialized for Writer {
    fn is_initialized(&self) -> bool {
        true
    }
}

impl Writer {
    pub fn unpack(data: &[u8]) -> Result<&Self, ProgramError> {
        unsafe { load_acc::<Writer>(data) }
    }
    pub fn unpack_mut(data: &mut [u8]) -> Result<&mut Self, ProgramError> {
        unsafe { load_acc_mut::<Writer>(data) }
    }
}

// account!(AccountType, Writer);
