use crate::state::AccountType;
use crate::state::NAME_LEN;
use crate::utils::AccountDiscriminator;
use bytemuck::{Pod, Zeroable};
use pinocchio::pubkey::Pubkey;

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
pub struct Miner {
    pub authority: Pubkey,
    pub name: [u8; NAME_LEN],

    pub unclaimed_rewards: u64,

    pub challenge: [u8; 32],
    pub commitment: [u8; 32],

    pub multiplier: u64,

    pub last_proof_block: u64,
    pub last_proof_at: i64,

    pub total_proofs: u64,
    pub total_rewards: u64,
}

impl AccountDiscriminator for Miner {
    fn discriminator() -> u8 {
        AccountType::Miner.into()
    }
}
