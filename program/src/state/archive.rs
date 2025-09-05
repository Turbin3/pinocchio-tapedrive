use crate::state::AccountType;
use crate::utils::AccountDiscriminator;
use bytemuck::{Pod, Zeroable};

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
