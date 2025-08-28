use const_crypto::ed25519;
use pinocchio::pubkey::Pubkey;

// tape9hFAE7jstfKB2QT1ovFNUZKKtDUyGZiGQpnBFdL

// mpl_token_metadata
// metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s

pub const TAPE_ID: Pubkey = [
    13, 54, 220, 252, 136, 247, 73, 20, 47, 6, 78, 137, 18, 160, 48, 203, 213, 61, 221, 159, 81,
    168, 160, 144, 213, 135, 83, 108, 248, 37, 140, 51,
];

pub const MPL_TOKEN_METADATA_ID: Pubkey = [
    11, 112, 101, 177, 227, 209, 124, 69, 56, 157, 82, 127, 107, 4, 195, 205, 88, 184, 108, 115,
    26, 160, 253, 181, 73, 182, 209, 188, 3, 248, 41, 70,
];
pub const ARCHIVE: &[u8] = b"archive";
pub const BLOCK: &[u8] = b"block";
pub const EPOCH: &[u8] = b"epoch";
pub const MINER: &[u8] = b"miner";
pub const SPOOL: &[u8] = b"spool";
pub const WRITER: &[u8] = b"writer";
pub const TAPE: &[u8] = b"tape";
pub const TREASURY: &[u8] = b"treasury";
pub const MINT: &[u8] = b"mint";
pub const METADATA: &[u8] = b"metadata";

/// Mint PDA seed (raw bytes)
pub const MINT_SEED: &[u8] = &[152, 68, 212, 200, 25, 113, 221, 71];

pub const MINT_BUMP: u8 = ed25519::derive_program_address(&[MINT, MINT_SEED], &TAPE_ID).1;

pub const TREASURY_BUMP: u8 = ed25519::derive_program_address(&[TREASURY], &TAPE_ID).1;

/// Maximum length for names
pub const NAME_LEN:   usize = 32;
/// Header size in bytes
pub const HEADER_SIZE: usize = 64;
/// Rent charged per segment per block
pub const RENT_PER_SEGMENT: u64 = 100; 

/// Duration of one block in seconds (~1 minute)
pub const BLOCK_DURATION_SECONDS: u64 = 60;
/// Number of blocks per epoch (~10 minutes)
pub const EPOCH_BLOCKS: u64 = 10;
/// Adjustment interval (in epochs)
pub const ADJUSTMENT_INTERVAL: u64 = 50;
/// Number of blocks per year
pub const BLOCKS_PER_YEAR: u64 = 60 * 60 * 24 * 365 / BLOCK_DURATION_SECONDS;

// ====================================================================
// Merkle Tree Configuration
// ====================================================================
/// Height of the Merkle tree containing segments (number of levels)
pub const SEGMENT_TREE_HEIGHT: usize = 18;
/// Number of hashes in a Merkle proof for a segment tree
pub const SEGMENT_PROOF_LEN: usize = SEGMENT_TREE_HEIGHT;

/// Maximum reward scaling factor for miners
pub const MAX_CONSISTENCY_MULTIPLIER: u64  = 32;
/// Minimum reward scaling factor for miners
pub const MIN_CONSISTENCY_MULTIPLIER: u64  = 1;

/// Minimum mining difficulty
pub const MIN_MINING_DIFFICULTY: u64       = 1;
/// Minimum block participation required to solve a block
pub const MIN_PARTICIPATION_TARGET: u64    = 1;
/// Maximum block participation required to solve a block
pub const MAX_PARTICIPATION_TARGET: u64    = 100;

/// Segment size in bytes
pub const SEGMENT_SIZE: usize = 128;
/// Empty segment of SEGMENT_SIZE bytes for tapes that don't have minimum rent
pub const EMPTY_SEGMENT: [u8; SEGMENT_SIZE] = [0; SEGMENT_SIZE];
