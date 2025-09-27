use bytemuck::{Pod, Zeroable};
use tape_api::SEGMENT_PROOF_LEN;

use crate::{state::{DataLen}};


#[derive(Copy, Clone, Debug)]
pub struct ProofPath(pub [[u8; 32]; SEGMENT_PROOF_LEN]);

impl DataLen for ProofPath {
    const LEN: usize = 32 * SEGMENT_PROOF_LEN;
}

impl ProofPath {
    /// Borrow the inner array.
    pub fn as_array(&self) -> &[[u8; 32]; SEGMENT_PROOF_LEN] {
        &self.0
    }
}

impl AsRef<[[u8; 32]; SEGMENT_PROOF_LEN]> for ProofPath {
    fn as_ref(&self) -> &[[u8; 32]; SEGMENT_PROOF_LEN] {
        self.as_array()
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
/// Proof-of-work solution needed to mine a block using CrankX
pub struct PoW {
    pub digest: [u8; 16],
    pub nonce: [u8; 8],
}

impl DataLen for PoW {
    const LEN: usize = 16 + 8; // 24 bytes
}

impl PoW {
    pub fn from_solution(solution: crankx::Solution) -> Self {
        Self { 
            digest: solution.d,
            nonce: solution.n,
        }
    }

    pub fn as_solution(&self) -> crankx::Solution {
        crankx::Solution::new(self.digest, self.nonce)
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
/// Proof-of-access solution for the tape segment, cryptographically tied to the miner using PackX.
pub struct PoA {
    pub bump: [u8; 8],
    pub seed: [u8; 16],
    pub nonce: [u8; 128],
    pub path: ProofPath,
}

impl DataLen for PoA {
    const LEN: usize = 8 + 16 + 128 + ProofPath::LEN; // 8 + 16 + 128 + (32 * SEGMENT_PROOF_LEN)
}

impl PoA {
    pub fn from_solution(solution: &packx::Solution, path: impl Into<ProofPath>) -> Self {
        Self { 
            bump: solution.bump, 
            seed: solution.seeds, 
            nonce: solution.nonces, 
            path: path.into() 
        }
    }

    pub fn as_solution(&self) ->packx::Solution {
        packx::Solution::new(self.seed, self.nonce, self.bump)
    }
}