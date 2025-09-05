use crate::state::AccountType;
use crate::utils::AccountDiscriminator;
use bytemuck::{Pod, Zeroable};

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
pub struct Treasury {}

impl AccountDiscriminator for Treasury {
    fn discriminator() -> u8 {
        AccountType::Treasury as u8
    }
}
