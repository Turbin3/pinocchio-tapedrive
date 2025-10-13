use crate::{
    api::utils::{compute_challenge, compute_next_challenge},
    state::{
        try_from_account_info_mut, Archive, Block, Epoch, Mine, Miner, PoA, PoW, Tape,
        ADJUSTMENT_INTERVAL, BLOCK_DURATION_SECONDS, EPOCH_BLOCKS,
    },
};
use brine_tree::{verify, Leaf};
use pinocchio::{
    account_info::AccountInfo,
    program_error::ProgramError,
    pubkey::Pubkey,
    sysvars::{clock::Clock, Sysvar},
    ProgramResult,
};
use tape_api::{
    error::TapeError, pda::miner_pda, EMPTY_SEGMENT, MAX_CONSISTENCY_MULTIPLIER,
    MAX_PARTICIPATION_TARGET, MIN_CONSISTENCY_MULTIPLIER, MIN_MINING_DIFFICULTY,
    MIN_PARTICIPATION_TARGET, SEGMENT_PROOF_LEN,
};

const EPOCHS_PER_YEAR: u64 = 365 * 24 * 60 / EPOCH_BLOCKS;

pub fn process_mine(accounts: &[AccountInfo], data: &[u8]) -> ProgramResult {
    let [signer_info, epoch_info, block_info, miner_info, tape_info, archive_info, slot_hashes_info] =
        accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    if !signer_info.is_signer() {
        return Err(ProgramError::MissingRequiredSignature);
    }

    if archive_info.owner() != &crate::id() {
        return Err(ProgramError::InvalidAccountData);
    }

    if epoch_info.owner() != &crate::id() {
        return Err(ProgramError::InvalidAccountData);
    }

    if block_info.owner() != &crate::id() {
        return Err(ProgramError::InvalidAccountData);
    }

    if tape_info.owner() != &crate::id() {
        return Err(ProgramError::InvalidAccountData);
    }

    if miner_info.owner() != &crate::id() {
        return Err(ProgramError::InvalidAccountData);
    }

    let archive = unsafe { try_from_account_info_mut::<Archive>(archive_info)? };
    let epoch = unsafe { try_from_account_info_mut::<Epoch>(epoch_info)? };
    let block = unsafe { try_from_account_info_mut::<Block>(block_info)? };
    let tape = unsafe { try_from_account_info_mut::<Tape>(tape_info)? };
    let miner = unsafe { try_from_account_info_mut::<Miner>(miner_info)? };

    let (miner_address, _miner_bump) = miner_pda(miner.authority, miner.name);

    if miner_info.key() != &miner_address {
        return Err(ProgramError::InvalidSeeds);
    }

    if signer_info.key() != &miner.authority {
        return Err(ProgramError::InvalidAccountOwner);
    }

    let current_time = Clock::get()?.unix_timestamp;
    check_submission(miner, block, epoch, current_time)?;

    let miner_challenge = compute_challenge(&block.challenge, &miner.challenge);

    let tape_number = compute_recall_tape(&miner_challenge, block.challenge_set);

    if tape.number != tape_number {
        return Err(TapeError::UnexpectedTape.into());
    }

    let args = Mine::try_from_bytes(data)?;

    verify_solution(
        epoch,
        tape,
        &miner.authority,
        &miner_challenge,
        args.pow,
        args.poa,
    )?;

    // Update miner
    update_multiplier(miner, block);

    let next_challenge = compute_next_challenge(&miner.challenge, slot_hashes_info)?;

    let reward = calculate_reward(epoch, tape, miner.multiplier);

    update_miner_state(miner, block, reward, current_time, next_challenge);

    update_tape_balance(tape, block.number);

    block.progress = block.progress.saturating_add(1);

    if block.progress >= epoch.target_participation {
        advance_block(block, current_time)?;

        let next_block_challenges = compute_next_challenge(&block.challenge, slot_hashes_info)?;

        block.challenge = next_block_challenges;
        block.challenge_set = archive.tapes_stored;
    }

    update_epoch(epoch, archive, current_time)?;

    Ok(())
}

// Helper: Advance the block state
fn advance_block(block: &mut Block, current_time: i64) -> ProgramResult {
    //  reset the block state
    block.progress = 0;
    block.last_proof_at = current_time;
    block.last_block_at = current_time;
    block.number = block.number.saturating_add(1);
    Ok(())
}

/// Helper: compute the recall tape number from a given challenge
#[inline(always)]
pub fn compute_recall_tape(challenge: &[u8; 32], total_tapes: u64) -> u64 {
    // Prevent division by zero
    if total_tapes == 0 {
        return 1;
    }
    u64::from_le_bytes(challenge[0..8].try_into().unwrap()) % total_tapes + 1
}

/// Helper: compute the recall segment number from a given challenge
#[inline(always)]
pub fn compute_recall_segment(challenge: &[u8; 32], total_segments: u64) -> u64 {
    // Prevent division by zero
    if total_segments == 0 {
        return 0;
    }

    u64::from_le_bytes(challenge[8..16].try_into().unwrap()) % total_segments
}

// Helper: Check if the block has stalled, meaning no solutions have been submitted for a while.
fn has_stalled(block: &Block, current_time: i64) -> bool {
    current_time
        > block
            .last_proof_at
            .saturating_add(BLOCK_DURATION_SECONDS as i64)
}

fn check_submission(
    miner: &Miner,
    block: &Block,
    epoch: &mut Epoch,
    current_time: i64,
) -> ProgramResult {
    // Check if the proof is too early, just in case someone aquires insane hardware
    // and can solve the challenge faster than we can adjust the difficulty.

    if miner.last_proof_block == block.number {
        if has_stalled(block, current_time) {
            epoch.duplicates = epoch.duplicates.saturating_add(1);
            Ok(())
        } else {
            Err(ProgramError::InvalidInstructionData)
        }
    } else {
        Ok(())
    }
}

fn verify_solution(
    epoch: &Epoch,
    tape: &Tape,
    miner_address: &Pubkey,
    miner_challenge: &[u8; 32],
    pow: PoW,
    poa: PoA,
) -> ProgramResult {
    let pow_solution = pow.as_solution();
    let poa_solution = poa.as_solution();

    let pow_difficulty = pow_solution.difficulty() as u64;
    let poa_difficulty = poa_solution.difficulty() as u64;

    check_condition(
        pow_difficulty >= epoch.mining_difficulty,
        TapeError::SolutionTooEasy,
    )?;

    check_condition(
        poa_difficulty >= epoch.packing_difficulty,
        TapeError::SolutionTooEasy,
    )?;

    // Check if the tape can be mined.
    if tape.has_minimum_rent() {
        let segment_number = compute_recall_segment(miner_challenge, tape.total_segments);

        let merkle_proof = poa.path.as_ref();
        let merkle_root = tape.merkle_root;
        let recall_segment = poa_solution.unpack(&miner_address);

        assert!(merkle_proof.len() == SEGMENT_PROOF_LEN);

        let leaf = Leaf::new(&[
            segment_number.to_le_bytes().as_ref(),
            recall_segment.as_ref(),
        ]);

        check_condition(
            verify(merkle_root, merkle_proof, leaf),
            TapeError::SolutionInvalid,
        )?;

        // Verify PoW using the actual recalled segment
        check_condition(
            pow_solution
                .is_valid(miner_challenge, &recall_segment)
                .is_ok(),
            TapeError::SolutionInvalid,
        )?;

        // For expired tapes, enforce use of the fixed segment
    } else {
        // Verify PoW using the fixed segment
        check_condition(
            pow_solution
                .is_valid(miner_challenge, &EMPTY_SEGMENT)
                .is_ok(),
            TapeError::SolutionInvalid,
        )?;
    }

    Ok(())
}

fn update_multiplier(miner: &mut Miner, block: &Block) {
    if miner.last_proof_block.saturating_add(1) == block.number {
        miner.multiplier = miner
            .multiplier
            .saturating_add(1)
            .min(MAX_CONSISTENCY_MULTIPLIER);
    } else {
        miner.multiplier = miner
            .multiplier
            .saturating_sub(1)
            .max(MIN_CONSISTENCY_MULTIPLIER);
    }
}

/// Helper: check a condition is true and return an error if not
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

// Helper: Get the scaled reward based on miner's consistency multiplier.
fn get_scaled_reward(reward: u64, multiplier: u64) -> u64 {
    assert!(multiplier >= MIN_CONSISTENCY_MULTIPLIER);
    assert!(multiplier <= MAX_CONSISTENCY_MULTIPLIER);

    reward
        .saturating_mul(multiplier)
        .saturating_div(MAX_CONSISTENCY_MULTIPLIER)
}

fn calculate_reward(epoch: &Epoch, tape: &Tape, multiplier: u64) -> u64 {
    // divide the scaled reward by the target participation, each miner gets an equal share
    let available_reward = epoch.reward_rate.saturating_div(epoch.target_participation);

    let scaled_reward = get_scaled_reward(available_reward, multiplier);

    // if the tape is subsidized, miner will get full rewards
    if tape.has_minimum_rent() {
        scaled_reward
    } else {
        scaled_reward.saturating_div(2)
    }
}

fn update_miner_state(
    miner: &mut Miner,
    block: &Block,
    final_reward: u64,
    current_time: i64,
    next_miner_challenge: [u8; 32],
) {
    miner.unclaimed_rewards += final_reward;
    miner.total_rewards += final_reward;
    miner.total_proofs += 1;
    miner.last_proof_block = block.number;
    miner.challenge = next_miner_challenge;
    miner.last_proof_at = current_time;
}

fn update_tape_balance(tape: &mut Tape, block_number: u64) {
    let rent = tape.rent_owed(block_number);
    tape.balance = tape.balance.saturating_sub(rent);
}

fn update_epoch(epoch: &mut Epoch, archive: &Archive, current_time: i64) -> ProgramResult {
    // check if we need to advance the epoch
    if epoch.progress >= EPOCH_BLOCKS {
        advance_epoch(epoch, current_time)?;

        let base_rate = get_base_rate(epoch.number);
        let storage_rate = archive.block_reward();

        epoch.reward_rate = storage_rate.saturating_add(base_rate);
    // Epoch is still in progress, increment the progress
    } else {
        epoch.progress = epoch.progress.saturating_add(1);
    }
    Ok(())
}

// helper - advance epoch state
fn advance_epoch(epoch: &mut Epoch, current_time: i64) -> ProgramResult {
    adjust_participation(epoch);
    adjust_difficulty(epoch, current_time);

    epoch.number = epoch.number.saturating_add(1);
    epoch.last_epoch_at = current_time;
    epoch.progress = 0;
    epoch.duplicates = 0;
    epoch.mining_difficulty = epoch.mining_difficulty.max(MIN_MINING_DIFFICULTY);
    epoch.target_participation = epoch.target_participation.max(MIN_PARTICIPATION_TARGET);

    Ok(())
}

fn adjust_participation(epoch: &mut Epoch) {
    if epoch.duplicates == 0 {
        if epoch.number % ADJUSTMENT_INTERVAL == 0 {
            epoch.target_participation = epoch
                .target_participation
                .saturating_add(1)
                .min(MAX_PARTICIPATION_TARGET);
        }
    } else {
        epoch.target_participation = epoch
            .target_participation
            .saturating_sub(1)
            .max(MIN_PARTICIPATION_TARGET);
    }
}

fn adjust_difficulty(epoch: &mut Epoch, current_time: i64) {
    let elapsed_time = current_time.saturating_sub(epoch.last_epoch_at);
    let average_time_per_block = elapsed_time / EPOCH_BLOCKS as i64;

    if average_time_per_block < BLOCK_DURATION_SECONDS as i64 {
        epoch.mining_difficulty = epoch.mining_difficulty.saturating_add(1);
    } else {
        epoch.mining_difficulty = epoch
            .mining_difficulty
            .saturating_sub(1)
            .max(MIN_MINING_DIFFICULTY);
    }
}

/// Pre-computed base rate based on current epoch number. After which, the archive
/// storage fees would take over, with no further inflation.
///
/// The hard-coded values avoid CU overhead.
#[inline(always)]
pub fn get_base_rate(current_epoch: u64) -> u64 {
    match current_epoch {
        n if n < 1 * EPOCHS_PER_YEAR => 10000000000, // Year ~1,  about 1.00 TAPE/min
        n if n < 2 * EPOCHS_PER_YEAR => 7500000000,  // Year ~2,  about 0.75 TAPE/min
        n if n < 3 * EPOCHS_PER_YEAR => 5625000000,  // Year ~3,  about 0.56 TAPE/min
        n if n < 4 * EPOCHS_PER_YEAR => 4218750000,  // Year ~4,  about 0.42 TAPE/min
        n if n < 5 * EPOCHS_PER_YEAR => 3164062500,  // Year ~5,  about 0.32 TAPE/min
        n if n < 6 * EPOCHS_PER_YEAR => 2373046875,  // Year ~6,  about 0.24 TAPE/min
        n if n < 7 * EPOCHS_PER_YEAR => 1779785156,  // Year ~7,  about 0.18 TAPE/min
        n if n < 8 * EPOCHS_PER_YEAR => 1334838867,  // Year ~8,  about 0.13 TAPE/min
        n if n < 9 * EPOCHS_PER_YEAR => 1001129150,  // Year ~9,  about 0.10 TAPE/min
        n if n < 10 * EPOCHS_PER_YEAR => 750846862,  // Year ~10, about 0.08 TAPE/min
        n if n < 11 * EPOCHS_PER_YEAR => 563135147,  // Year ~11, about 0.06 TAPE/min
        n if n < 12 * EPOCHS_PER_YEAR => 422351360,  // Year ~12, about 0.04 TAPE/min
        n if n < 13 * EPOCHS_PER_YEAR => 316763520,  // Year ~13, about 0.03 TAPE/min
        n if n < 14 * EPOCHS_PER_YEAR => 237572640,  // Year ~14, about 0.02 TAPE/min
        n if n < 15 * EPOCHS_PER_YEAR => 178179480,  // Year ~15, about 0.02 TAPE/min
        n if n < 16 * EPOCHS_PER_YEAR => 133634610,  // Year ~16, about 0.01 TAPE/min
        n if n < 17 * EPOCHS_PER_YEAR => 100225957,  // Year ~17, about 0.01 TAPE/min
        n if n < 18 * EPOCHS_PER_YEAR => 75169468,   // Year ~18, about 0.01 TAPE/min
        n if n < 19 * EPOCHS_PER_YEAR => 56377101,   // Year ~19, about 0.01 TAPE/min
        n if n < 20 * EPOCHS_PER_YEAR => 42282825,   // Year ~20, about 0.00 TAPE/min
        n if n < 21 * EPOCHS_PER_YEAR => 31712119,   // Year ~21, about 0.00 TAPE/min
        n if n < 22 * EPOCHS_PER_YEAR => 23784089,   // Year ~22, about 0.00 TAPE/min
        n if n < 23 * EPOCHS_PER_YEAR => 17838067,   // Year ~23, about 0.00 TAPE/min
        n if n < 24 * EPOCHS_PER_YEAR => 13378550,   // Year ~24, about 0.00 TAPE/min
        n if n < 25 * EPOCHS_PER_YEAR => 10033913,   // Year ~25, about 0.00 TAPE/min
        _ => 0,
    }
}
