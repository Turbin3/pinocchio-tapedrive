use crate::state::AccountType;
use crate::utils::AccountDiscriminator;
use bytemuck::{Pod, Zeroable};
use pinocchio::pubkey::Pubkey;

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
pub struct Writer {
    pub tape: Pubkey,
    // pub state: SegmentTree,
}

impl AccountDiscriminator for Writer {
    fn discriminator() -> u8 {
        AccountType::Writer as u8
    }
}
