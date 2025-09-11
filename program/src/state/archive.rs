use crate::state::{AccountType, DataLen};
use crate::utils::AccountDiscriminator;
use bytemuck::{Pod, Zeroable};
use tape_api::RENT_PER_SEGMENT;

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
pub struct Archive {
    pub tapes_stored: u64,
    pub segments_stored: u64,
}

impl AccountDiscriminator for Archive {
    fn discriminator() -> u8 {
        AccountType::Archive.into()
    }
}

impl DataLen for Archive {
    const LEN: usize = 8 + 8;
}

impl Archive {
    /// Global reward to miners for the current block.
    #[inline]
    pub fn block_reward(&self) -> u64 {
        self.segments_stored.saturating_mul(RENT_PER_SEGMENT)
    }
}