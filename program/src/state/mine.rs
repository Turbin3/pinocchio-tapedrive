use pinocchio::program_error::ProgramError;

use crate::state::{DataLen, PoA, PoW};

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct Mine {
    pub pow: PoW,
    pub poa: PoA,
}

impl DataLen for Mine {
    const LEN: usize = PoW::LEN + PoA::LEN;
}

impl Mine {
    pub fn try_from_bytes(
        data: &[u8],
    ) -> Result<&mut Self, ProgramError> {
        if data.len() != Self::LEN {
            return Err(ProgramError::InvalidAccountData);
        }
        // SAFETY: Caller provides a mutable slice with exact size Self::LEN; we transmute to &mut Self.
        Ok(unsafe { &mut *(data.as_ptr() as *mut Self) })
    }
}