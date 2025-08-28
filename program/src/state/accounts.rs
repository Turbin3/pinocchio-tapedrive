use pinocchio::program_error::ProgramError;

use crate::state::{DataLen, PoA, PoW, BLOCKS_PER_YEAR, HEADER_SIZE, NAME_LEN, RENT_PER_SEGMENT};

/// archive account to store global tape data
#[repr(C)]
pub struct Archive {
    pub tapes_stored: u64,
    pub segments_stored: u64,
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

/// epoch account for mining difficulty and rewards
#[repr(C)]
pub struct Epoch {
    pub number: u64,
    pub progress: u64,

    pub mining_difficulty: u64,
    pub packing_difficulty: u64,
    pub target_participation: u64,
    pub reward_rate: u64,
    pub duplicates: u64,

    pub last_epoch_at: i64,
}

impl DataLen for Epoch {
    const LEN: usize = 8 + 8 + 8 + 8 + 8 + 8 + 8 + 8; // 64 bytes
}

/// block account for current mining challenge
#[repr(C)]
pub struct Block {
    pub number: u64,
    pub progress: u64,

    pub challenge: [u8; 32],
    pub challenge_set: u64,

    pub last_proof_at: i64,
    pub last_block_at: i64,
}

impl DataLen for Block {
    const LEN: usize = 8 + 8 + 32 + 8 + 8 + 8; // 72 bytes
}

#[repr(C)]
pub struct Tape {
    pub number: u64,
    pub state: u64,

    pub authority: [u8; 32],

    pub name:        [u8; NAME_LEN],
    pub merkle_seed: [u8; 32],
    pub merkle_root: [u8; 32],
    pub header:      [u8; HEADER_SIZE],

    pub first_slot:      u64,
    pub tail_slot:       u64,
    pub balance:         u64,
    pub last_rent_block: u64,
    pub total_segments:  u64,
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

#[repr(C)]
pub struct Miner {
    pub authority: [u8; 32],
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
    const LEN: usize = 32 + 32 + 8 + 32 + 32 + 8 + 8 + 8 + 8 + 8; // 176 bytes
}


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