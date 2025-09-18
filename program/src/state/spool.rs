use crate::state::AccountType;
use crate::utils::AccountDiscriminator;
use bytemuck::{Pod, Zeroable};
use pinocchio::pubkey::Pubkey;
use tape_api::types::TapeTree;

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
pub struct Spool {
    pub number: u64,

    pub authority: Pubkey,
    pub state: TapeTree,
    pub seed: [u8; 32],
    pub contains: [u8; 32],

    pub total_tapes: u64,

    pub last_proof_block: u64,
    pub last_proof_at: i64,
}

impl AccountDiscriminator for Spool {
    fn discriminator() -> u8 {
        AccountType::Spool as u8
    }
}
