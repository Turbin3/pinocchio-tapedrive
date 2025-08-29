use pinocchio::{pubkey::Pubkey};
use crate::state::utils::DataLen;

#[repr(C)]
pub struct Miner {
    pub authority: Pubkey,

    pub name: [u8; 32],
    pub unclaimed_rewards: u64,

    pub challenge: [u8; 32],
    pub commitment: [u8; 32],

    pub multiplier: u64,

    pub last_proof_block: u64,
    pub last_proof_at: i64,

    pub total_proofs: u64,
    pub total_rewards: u64,
}

impl DataLen for Miner {
    const LEN: usize = core::mem::size_of::<Miner>();
}