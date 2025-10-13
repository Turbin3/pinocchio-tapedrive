use crate::consts::*;
use crate::error::*;
use crate::types::*;
use blake3::Hasher;
// use brine_tree::Leaf;
use bytemuck::{Pod, Zeroable};
use core::cmp::min;
use pinocchio::{
    account_info::AccountInfo,
    program_error::ProgramError,
    sysvars::{clock::Clock, Sysvar},
    ProgramResult,
};
use pinocchio_system::instructions::CreateAccount;

/// SlotHash from Solana's slot_hashes sysvar (Slot + Hash = 8 + 32 = 40 bytes)
const SLOTHASH_SIZE: usize = 40;

#[inline(always)]
pub fn check_condition<E>(condition: bool, err: E) -> ProgramResult
where
    E: Into<ProgramError>,
{
    if !condition {
        return Err(err.into());
    }
    Ok(())
}

#[inline(always)]
pub fn padded_array<const N: usize>(input: &[u8]) -> [u8; N] {
    let mut out = [0u8; N];
    let len = min(input.len(), N);
    out[..len].copy_from_slice(&input[..len]);
    out
}

#[inline(always)]
pub fn to_name<T>(val: T) -> [u8; NAME_LEN]
where
    T: AsRef<[u8]>,
{
    let bytes = val.as_ref();
    assert!(
        bytes.len() <= NAME_LEN,
        "name too long ({} > {})",
        bytes.len(),
        NAME_LEN
    );
    padded_array::<NAME_LEN>(bytes)
}

#[inline(always)]
pub fn from_name(val: &[u8; NAME_LEN]) -> &str {
    let end = val.iter().position(|&b| b == 0).unwrap_or(NAME_LEN);
    core::str::from_utf8(&val[..end]).unwrap()
}

// #[inline(always)]
// pub fn compute_leaf(segment_id: u64, segment: &[u8; SEGMENT_SIZE]) -> Leaf {
//     let segment_id = segment_id.to_le_bytes();
//     Leaf::new(&[segment_id.as_ref(), segment])
// }

// #[inline(always)]
// pub fn write_segment(
//     tree: &mut SegmentTree,
//     segment_id: u64,
//     segment: &[u8; SEGMENT_SIZE],
// ) -> ProgramResult {
//     let leaf = compute_leaf(segment_id, segment);
//     check_condition(tree.try_add_leaf(leaf).is_ok(), TapeError::WriteFailed)?;
//     Ok(())
// }

// #[inline(always)]
// pub fn update_segment(
//     tree: &mut SegmentTree,
//     segment_id: u64,
//     old_segment: &[u8; SEGMENT_SIZE],
//     new_segment: &[u8; SEGMENT_SIZE],
//     proof: &[[u8; 32]; SEGMENT_PROOF_LEN],
// ) -> ProgramResult {
//     let old_leaf = compute_leaf(segment_id, old_segment);
//     let new_leaf = compute_leaf(segment_id, new_segment);
//     check_condition(
//         tree.try_replace_leaf(proof, old_leaf, new_leaf).is_ok(),
//         TapeError::WriteFailed,
//     )?;
//     Ok(())
// }

#[inline(always)]
pub fn compute_next_challenge(
    current_challenge: &[u8; 32],
    slot_hashes_info: &AccountInfo,
) -> Result<[u8; 32], ProgramError> {
    // Read the first SlotHash from the SlotHashes sysvar
    // SlotHash is 40 bytes: 8 bytes (slot) + 32 bytes (hash)
    let slothash_data = slot_hashes_info.try_borrow_data()?;

    if slothash_data.len() < SLOTHASH_SIZE {
        return Err(ProgramError::InvalidAccountData);
    }

    let slothash = &slothash_data[0..SLOTHASH_SIZE];

    // Hash current_challenge + slothash using blake3
    let mut hasher = Hasher::new();
    hasher.update(current_challenge);
    hasher.update(slothash);
    let challenge = hasher.finalize();

    Ok(challenge.into())
}

#[inline(always)]
pub fn compute_challenge(block_challenge: &[u8; 32], miner_challenge: &[u8; 32]) -> [u8; 32] {
    let mut hasher = Hasher::new();

    hasher.update(block_challenge);
    hasher.update(miner_challenge);
    let challenge = hasher.finalize();

    challenge.into()
}

#[inline(always)]
pub fn compute_recall_tape(challenge: &[u8; 32], total_tapes: u64) -> u64 {
    if total_tapes == 0 {
        return 1;
    }
    u64::from_le_bytes(challenge[0..8].try_into().unwrap()) % total_tapes + 1
}

#[inline(always)]
pub fn compute_recall_segment(challenge: &[u8; 32], total_segments: u64) -> u64 {
    if total_segments == 0 {
        return 0;
    }
    u64::from_le_bytes(challenge[8..16].try_into().unwrap()) % total_segments
}
