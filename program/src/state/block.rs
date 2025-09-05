use crate::state::AccountType;
use crate::utils::AccountDiscriminator;
use bytemuck::{Pod, Zeroable};

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

impl AccountDiscriminator for Block {
    fn discriminator() -> u8 {
        AccountType::Block.into()
    }
}
