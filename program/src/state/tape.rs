use crate::state::AccountType;
use crate::state::DataLen;
use crate::state::BLOCKS_PER_YEAR;
use crate::state::HEADER_SIZE;
use crate::state::NAME_LEN;
use crate::utils::AccountDiscriminator;
use bytemuck::{Pod, Zeroable};
use num_enum::{IntoPrimitive, TryFromPrimitive};
use pinocchio::pubkey::Pubkey;
use tape_api::RENT_PER_SEGMENT;

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
pub struct Tape {
    pub number: u64,
    pub state: u64,

    pub authority: Pubkey,

    pub name: [u8; NAME_LEN],
    pub merkle_seed: [u8; 32],
    pub merkle_root: [u8; 32],
    pub header: [u8; HEADER_SIZE],

    pub first_slot: u64,
    pub tail_slot: u64,
    pub balance: u64,
    pub last_rent_block: u64,
    pub total_segments: u64,
    // +Phantom Vec<Hash> for merkle subtree nodes (up to 4096).
}

#[repr(u64)]
#[derive(Clone, Copy, Debug, Eq, PartialEq, IntoPrimitive, TryFromPrimitive)]
pub enum TapeState {
    Unknown = 0,
    Created,
    Writing,
    Finalized,
}

impl AccountDiscriminator for Tape {
    fn discriminator() -> u8 {
        AccountType::Tape as u8
    }
}

impl DataLen for Tape {
    const LEN: usize = 8 + 8 + 32 + NAME_LEN + 32 + 32 + HEADER_SIZE + 8 + 8 + 8 + 8 + 8; // 248 bytes
}

impl Tape {
    // check if this tape is subsidized.
    pub fn has_minimum_rent(&self) -> bool {
        self.balance >= self.rent_per_block()
    }

    pub fn rent_per_block(&self) -> u64 {
        self.total_segments.saturating_mul(RENT_PER_SEGMENT)
    }

    // check if this tape has enough balance to cover finalization.
    pub fn can_finalize(&self) -> bool {
        self.balance >= self.rent_per_block().saturating_mul(BLOCKS_PER_YEAR)
    }

    // rent owed since last_rent_block.
    pub fn rent_owed(&self, current_block: u64) -> u64 {
        let blocks = current_block.saturating_sub(self.last_rent_block) as u128;
        (self.rent_per_block() as u128 * blocks) as u64
    }
}
