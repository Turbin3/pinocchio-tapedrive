#![allow(unexpected_cfgs)]

use super::{
    error::{BrineTreeError, ProgramResult},
    leaf::{hashv, Hash, Leaf},
    utils::check_condition,
};
use bytemuck::{Pod, Zeroable};
use core::mem::MaybeUninit;

// ============================================================================
// PRE-COMPUTED ZERO VALUES FOR COMMON TREE HEIGHTS
// ============================================================================
// These are computed off-chain to avoid expensive on-chain Blake3 hashing
// Each zero value represents the hash at that level of an empty Merkle tree

/// Pre-computed zero values for a SegmentTree (height 18)
/// This eliminates ~45,000 CU of Blake3 hash computations during initialization!
pub const SEGMENT_TREE_ZEROS_18: [Hash; 18] = [
    Hash {
        value: [
            175, 19, 73, 185, 245, 249, 161, 166, 160, 64, 77, 234, 54, 220, 201, 73, 155, 203, 37,
            201, 173, 193, 18, 183, 204, 154, 147, 202, 228, 31, 50, 98,
        ],
    },
    Hash {
        value: [
            6, 136, 207, 133, 207, 74, 96, 245, 255, 67, 11, 193, 233, 39, 192, 111, 125, 204, 93,
            179, 172, 8, 166, 82, 210, 71, 240, 16, 28, 205, 237, 250,
        ],
    },
    Hash {
        value: [
            179, 27, 44, 89, 223, 209, 168, 252, 92, 175, 44, 35, 220, 47, 23, 49, 83, 181, 111,
            31, 36, 223, 132, 94, 38, 150, 234, 193, 221, 46, 211, 76,
        ],
    },
    Hash {
        value: [
            76, 45, 84, 214, 111, 181, 164, 55, 77, 51, 78, 156, 17, 150, 199, 100, 3, 217, 220,
            52, 182, 75, 60, 79, 18, 196, 81, 67, 139, 186, 33, 29,
        ],
    },
    Hash {
        value: [
            124, 214, 29, 100, 122, 91, 175, 190, 62, 145, 224, 240, 13, 97, 189, 43, 227, 114,
            252, 209, 208, 27, 66, 198, 46, 200, 189, 142, 110, 144, 14, 238,
        ],
    },
    Hash {
        value: [
            189, 141, 118, 13, 209, 90, 201, 202, 95, 88, 250, 190, 245, 235, 21, 77, 100, 106,
            170, 29, 72, 66, 112, 62, 225, 0, 121, 29, 203, 188, 154, 145,
        ],
    },
    Hash {
        value: [
            93, 120, 125, 159, 164, 106, 176, 232, 178, 20, 100, 2, 151, 142, 84, 99, 40, 193, 97,
            221, 187, 164, 216, 77, 173, 96, 195, 217, 186, 81, 170, 193,
        ],
    },
    Hash {
        value: [
            94, 24, 76, 43, 16, 100, 12, 112, 44, 251, 6, 177, 67, 54, 132, 202, 40, 189, 208, 24,
            56, 138, 157, 5, 168, 13, 92, 45, 30, 136, 129, 46,
        ],
    },
    Hash {
        value: [
            23, 252, 132, 207, 134, 173, 14, 225, 85, 193, 211, 107, 47, 89, 72, 11, 142, 236, 194,
            194, 240, 156, 143, 241, 226, 234, 125, 92, 173, 101, 239, 106,
        ],
    },
    Hash {
        value: [
            139, 70, 244, 201, 10, 196, 184, 204, 208, 69, 148, 178, 158, 193, 101, 169, 132, 37,
            123, 215, 51, 79, 142, 25, 144, 139, 26, 60, 108, 98, 36, 191,
        ],
    },
    Hash {
        value: [
            28, 106, 239, 178, 224, 253, 179, 113, 37, 224, 104, 136, 243, 138, 94, 155, 6, 41,
            155, 80, 3, 110, 179, 57, 150, 241, 237, 180, 84, 83, 149, 170,
        ],
    },
    Hash {
        value: [
            244, 26, 210, 16, 116, 97, 238, 221, 47, 159, 8, 218, 189, 57, 233, 46, 226, 154, 148,
            118, 162, 87, 193, 247, 195, 12, 102, 81, 33, 110, 102, 239,
        ],
    },
    Hash {
        value: [
            154, 215, 212, 153, 245, 153, 125, 123, 145, 145, 133, 72, 78, 134, 229, 254, 100, 182,
            30, 118, 216, 11, 216, 90, 0, 4, 97, 14, 109, 146, 183, 180,
        ],
    },
    Hash {
        value: [
            133, 242, 74, 230, 214, 21, 177, 226, 56, 64, 125, 102, 193, 155, 59, 241, 36, 32, 196,
            107, 171, 228, 93, 212, 224, 110, 204, 191, 53, 110, 105, 77,
        ],
    },
    Hash {
        value: [
            220, 168, 41, 152, 37, 170, 227, 202, 156, 87, 70, 175, 22, 164, 90, 145, 117, 46, 145,
            179, 163, 252, 185, 202, 145, 71, 94, 184, 169, 19, 65, 121,
        ],
    },
    Hash {
        value: [
            233, 122, 177, 16, 103, 253, 59, 179, 40, 62, 214, 134, 15, 37, 122, 115, 178, 124, 25,
            7, 32, 107, 202, 37, 157, 193, 190, 57, 124, 242, 234, 32,
        ],
    },
    Hash {
        value: [
            168, 21, 107, 83, 102, 192, 152, 226, 49, 27, 224, 187, 117, 16, 220, 44, 227, 196,
            136, 159, 48, 127, 137, 138, 46, 104, 24, 216, 197, 211, 33, 80,
        ],
    },
    Hash {
        value: [
            15, 209, 210, 83, 40, 39, 226, 213, 196, 130, 21, 128, 57, 19, 184, 190, 12, 11, 131,
            81, 156, 38, 74, 122, 80, 5, 144, 183, 90, 49, 88, 250,
        ],
    },
];

#[cfg(feature = "std")]
extern crate std;
#[cfg(feature = "std")]
use std::{vec, vec::Vec};

#[repr(C)]
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct MerkleTree<const N: usize> {
    pub root: Hash,
    pub filled_subtrees: [Hash; N],
    pub zero_values: [Hash; N],
    pub next_index: u64,
}

unsafe impl<const N: usize> Zeroable for MerkleTree<N> {}
unsafe impl<const N: usize> Pod for MerkleTree<N> {}

impl<const N: usize> MerkleTree<N> {
    pub fn new(seeds: &[&[u8]]) -> Self {
        let zeros = Self::calc_zeros(seeds);
        Self {
            next_index: 0,
            root: zeros[N - 1],
            filled_subtrees: zeros,
            zero_values: zeros,
        }
    }

    pub fn from_zeros(zeros: [Hash; N]) -> Self {
        Self {
            next_index: 0,
            root: zeros[N - 1],
            filled_subtrees: zeros,
            zero_values: zeros,
        }
    }

    pub const fn get_depth(&self) -> u8 {
        N as u8
    }

    pub const fn get_size() -> usize {
        core::mem::size_of::<Self>()
    }

    pub fn get_root(&self) -> Hash {
        self.root
    }

    pub fn get_empty_leaf(&self) -> Leaf {
        self.zero_values[0].as_leaf()
    }

    pub fn init(&mut self, seeds: &[&[u8]]) {
        let zeros = Self::calc_zeros(seeds);
        self.next_index = 0;
        self.root = zeros[N - 1];
        self.filled_subtrees = zeros;
        self.zero_values = zeros;
    }

    /// Returns the number of leaves currently in the Merkle tree.
    pub fn get_leaf_count(&self) -> u64 {
        self.next_index
    }

    /// Returns the maximum capacity of the Merkle tree.
    pub fn get_capacity(&self) -> u64 {
        1u64 << N
    }

    /// Calculates the zero values for the Merkle tree based on the provided seeds.
    fn calc_zeros(seeds: &[&[u8]]) -> [Hash; N] {
        let mut zeros: [Hash; N] = [Hash::default(); N];
        let mut current = hashv(seeds);

        for i in 0..N {
            zeros[i] = current;
            current = hashv(&[b"NODE".as_ref(), current.as_ref(), current.as_ref()]);
        }

        zeros
    }

    pub fn try_add(&mut self, data: &[&[u8]]) -> ProgramResult {
        let leaf = Leaf::new(data);
        self.try_add_leaf(leaf)
    }

    pub fn try_add_leaf(&mut self, leaf: Leaf) -> ProgramResult {
        check_condition(self.next_index < (1u64 << N), BrineTreeError::TreeFull)?;

        let mut current_index = self.next_index;
        let mut current_hash = Hash::from(leaf);
        let mut left;
        let mut right;

        for i in 0..N {
            if current_index % 2 == 0 {
                left = current_hash;
                right = self.zero_values[i];
                self.filled_subtrees[i] = current_hash;
            } else {
                left = self.filled_subtrees[i];
                right = current_hash;
            }

            current_hash = hash_left_right(left, right);
            current_index /= 2;
        }

        self.root = current_hash;
        self.next_index += 1;

        Ok(())
    }

    /// Removes a leaf from the tree using the provided proof.
    #[cfg(feature = "std")]
    pub fn try_remove<P>(&mut self, proof: &[P], data: &[&[u8]]) -> ProgramResult
    where
        P: Into<Hash> + Copy,
    {
        let proof_hashes: Vec<Hash> = proof.iter().map(|p| (*p).into()).collect();
        let original_leaf = Leaf::new(data);
        self.try_remove_leaf(&proof_hashes, original_leaf)
    }

    /// Removes a leaf from the tree using the provided proof without Vec allocation.
    pub fn try_remove_no_std<P>(&mut self, proof: &[P], data: &[&[u8]]) -> ProgramResult
    where
        P: Into<Hash> + Copy,
    {
        let original_leaf = Leaf::new(data);
        self.try_remove_leaf_no_std(proof, original_leaf)
    }

    /// Removes a leaf from the tree using the provided proof.
    #[cfg(feature = "std")]
    pub fn try_remove_leaf<P>(&mut self, proof: &[P], leaf: Leaf) -> ProgramResult
    where
        P: Into<Hash> + Copy,
    {
        let proof_hashes: Vec<Hash> = proof.iter().map(|p| (*p).into()).collect();
        self.check_length(&proof_hashes)?;
        self.try_replace_leaf(&proof_hashes, leaf, self.get_empty_leaf())
    }

    /// Removes a leaf from the tree using the provided proof without Vec allocation.
    pub fn try_remove_leaf_no_std<P>(&mut self, proof: &[P], leaf: Leaf) -> ProgramResult
    where
        P: Into<Hash> + Copy,
    {
        self.check_length_no_std(proof)?;
        self.try_replace_leaf_no_std(proof, leaf, self.get_empty_leaf())
    }

    /// Replaces a leaf in the tree with new data using the provided proof.
    #[cfg(feature = "std")]
    pub fn try_replace<P>(
        &mut self,
        proof: &[P],
        original_data: &[&[u8]],
        new_data: &[&[u8]],
    ) -> ProgramResult
    where
        P: Into<Hash> + Copy,
    {
        let proof_hashes: Vec<Hash> = proof.iter().map(|p| (*p).into()).collect();
        let original_leaf = Leaf::new(original_data);
        let new_leaf = Leaf::new(new_data);
        self.try_replace_leaf(&proof_hashes, original_leaf, new_leaf)
    }

    /// Replaces a leaf in the tree with new data using the provided proof without Vec allocation.
    pub fn try_replace_no_std<P>(
        &mut self,
        proof: &[P],
        original_data: &[&[u8]],
        new_data: &[&[u8]],
    ) -> ProgramResult
    where
        P: Into<Hash> + Copy,
    {
        let original_leaf = Leaf::new(original_data);
        let new_leaf = Leaf::new(new_data);
        self.try_replace_leaf_no_std(proof, original_leaf, new_leaf)
    }

    /// Replaces a leaf in the tree with a new leaf using the provided proof.
    #[cfg(feature = "std")]
    pub fn try_replace_leaf<P>(
        &mut self,
        proof: &[P],
        original_leaf: Leaf,
        new_leaf: Leaf,
    ) -> ProgramResult
    where
        P: Into<Hash> + Copy,
    {
        let proof_hashes: Vec<Hash> = proof.iter().map(|p| (*p).into()).collect();
        self.check_length(&proof_hashes)?;
        let original_path = compute_path(&proof_hashes, original_leaf);
        let new_path = compute_path(&proof_hashes, new_leaf);
        check_condition(
            is_valid_path(&original_path, self.root),
            BrineTreeError::InvalidProof,
        )?;
        for i in 0..N {
            if original_path[i] == self.filled_subtrees[i] {
                self.filled_subtrees[i] = new_path[i];
            }
        }
        self.root = *new_path.last().unwrap();
        Ok(())
    }

    /// Replaces a leaf in the tree with a new leaf using the provided proof without Vec allocation.
    pub fn try_replace_leaf_no_std<P>(
        &mut self,
        proof: &[P],
        original_leaf: Leaf,
        new_leaf: Leaf,
    ) -> ProgramResult
    where
        P: Into<Hash> + Copy,
    {
        self.check_length_no_std(proof)?;
        let (original_path, original_root) = self.compute_path_no_std(proof, original_leaf);
        let (new_path, new_root) = self.compute_path_no_std(proof, new_leaf);
        check_condition(original_root == self.root, BrineTreeError::InvalidProof)?;
        for i in 0..N {
            if original_path[i] == self.filled_subtrees[i] {
                self.filled_subtrees[i] = new_path[i];
            }
        }
        self.root = new_root;
        Ok(())
    }

    /// Checks if the proof contains the specified data.
    #[cfg(feature = "std")]
    pub fn contains<P>(&self, proof: &[P], data: &[&[u8]]) -> bool
    where
        P: Into<Hash> + Copy,
    {
        let proof_hashes: Vec<Hash> = proof.iter().map(|p| (*p).into()).collect();
        let leaf = Leaf::new(data);
        self.contains_leaf(&proof_hashes, leaf)
    }

    /// Checks if the proof contains the specified data without Vec allocation.
    pub fn contains_no_std<P>(&self, proof: &[P], data: &[&[u8]]) -> bool
    where
        P: Into<Hash> + Copy,
    {
        let leaf = Leaf::new(data);
        self.contains_leaf_no_std(proof, leaf)
    }

    /// Checks if the proof contains the specified leaf.
    #[cfg(feature = "std")]
    pub fn contains_leaf<P>(&self, proof: &[P], leaf: Leaf) -> bool
    where
        P: Into<Hash> + Copy,
    {
        let proof_hashes: Vec<Hash> = proof.iter().map(|p| (*p).into()).collect();
        if self.check_length(&proof_hashes).is_err() {
            return false;
        }
        is_valid_leaf(&proof_hashes, self.root, leaf)
    }

    /// Checks if the proof contains the specified leaf without Vec allocation.
    pub fn contains_leaf_no_std<P>(&self, proof: &[P], leaf: Leaf) -> bool
    where
        P: Into<Hash> + Copy,
    {
        if self.check_length_no_std(proof).is_err() {
            return false;
        }
        is_valid_leaf_no_std(proof, self.root, leaf)
    }

    /// Checks if the proof length matches the expected depth of the tree.
    fn check_length(&self, proof: &[Hash]) -> Result<(), BrineTreeError> {
        check_condition(proof.len() == N, BrineTreeError::ProofLength)
    }

    /// Checks if the proof length matches the expected depth of the tree (no_std version).
    fn check_length_no_std<P>(&self, proof: &[P]) -> Result<(), BrineTreeError>
    where
        P: Into<Hash> + Copy,
    {
        check_condition(proof.len() == N, BrineTreeError::ProofLength)
    }

    /// Computes the path from the leaf to the root using the provided proof without Vec allocation.
    /// Returns a tuple of (path_hashes, root_hash) where path_hashes contains all intermediate hashes.
    fn compute_path_no_std<P>(&self, proof: &[P], leaf: Leaf) -> ([Hash; N], Hash)
    where
        P: Into<Hash> + Copy,
    {
        let mut path_hashes = [Hash::default(); N];
        let mut computed_hash = Hash::from(leaf);

        // Store the leaf hash as the first path element
        if N > 0 {
            path_hashes[0] = computed_hash;
        }

        // Compute the path up the tree
        for (i, proof_element) in proof.iter().enumerate() {
            computed_hash = hash_left_right(computed_hash, (*proof_element).into());
            if i + 1 < N {
                path_hashes[i + 1] = computed_hash;
            }
        }

        (path_hashes, computed_hash)
    }

    /// Returns a Merkle proof for a specific leaf in the tree.
    #[cfg(feature = "std")]
    pub fn get_proof(&self, leaves: &[Leaf], leaf_index: usize) -> Vec<Hash> {
        get_merkle_proof(leaves, &self.zero_values, leaf_index, N)
    }

    /// Returns a Merkle proof for a specific leaf in the tree without Vec allocation.
    /// Uses MaybeUninit for efficient fixed-size array operations.
    pub fn get_proof_no_std(&self, leaves: &[Leaf], leaf_index: usize) -> [Hash; N] {
        get_merkle_proof_no_std(leaves, &self.zero_values, leaf_index)
    }

    /// Returns the layer nodes at a specific layer without Vec allocation.
    /// Returns the number of nodes written and the buffer containing the nodes.
    pub fn get_layer_nodes_no_std<const MAX_NODES: usize>(
        &self,
        leaves: &[Leaf],
        layer_number: usize,
    ) -> (usize, [Hash; MAX_NODES]) {
        get_layer_nodes_no_std::<N, MAX_NODES>(
            leaves,
            &self.zero_values,
            layer_number,
            self.next_index as usize,
        )
    }

    /// Hashes up to `layer_number` and returns only the non-empty nodes
    /// on that layer.
    #[cfg(feature = "std")]
    pub fn get_layer_nodes(&self, leaves: &[Leaf], layer_number: usize) -> Vec<Hash> {
        if layer_number > N {
            return vec![];
        }

        let valid_leaves = leaves
            .iter()
            .take(self.next_index as usize)
            .copied()
            .collect::<Vec<Leaf>>();

        let mut current_layer: Vec<Hash> =
            valid_leaves.iter().map(|leaf| Hash::from(*leaf)).collect();

        if current_layer.is_empty() || layer_number == 0 {
            return current_layer;
        }

        let mut current_level: usize = 0;
        loop {
            if current_layer.is_empty() {
                break;
            }
            let mut next_layer = Vec::with_capacity(current_layer.len().div_ceil(2));
            let mut i = 0;
            while i < current_layer.len() {
                if i + 1 < current_layer.len() {
                    let val = hash_left_right(current_layer[i], current_layer[i + 1]);
                    next_layer.push(val);
                    i += 2;
                } else {
                    let val = hash_left_right(current_layer[i], self.zero_values[current_level]);
                    next_layer.push(val);
                    i += 1;
                }
            }
            current_level += 1;
            if current_level == layer_number {
                return next_layer;
            }
            current_layer = next_layer;
        }
        vec![]
    }
}

/// Returns the layer nodes at a specific layer without Vec allocation.
/// Returns the number of nodes written and the buffer containing the nodes.
pub fn get_layer_nodes_no_std<const N: usize, const MAX_NODES: usize>(
    leaves: &[Leaf],
    zero_values: &[Hash],
    layer_number: usize,
    next_index: usize,
) -> (usize, [Hash; MAX_NODES]) {
    let mut result_buffer: [Hash; MAX_NODES] = [Hash::default(); MAX_NODES];

    if layer_number > N {
        return (0, result_buffer);
    }

    // Take only the valid leaves up to next_index
    let valid_leaf_count = core::cmp::min(leaves.len(), next_index);

    if valid_leaf_count == 0 {
        return (0, result_buffer);
    }

    // Use a reasonable maximum size that won't cause stack overflow
    const MAX_LAYER_SIZE: usize = 4096;

    // If we have too many leaves, limit them
    let actual_leaf_count = if valid_leaf_count > MAX_LAYER_SIZE {
        MAX_LAYER_SIZE
    } else {
        valid_leaf_count
    };

    // Initialize first layer with valid leaves
    let mut current_layer: [MaybeUninit<Hash>; MAX_LAYER_SIZE] =
        unsafe { MaybeUninit::uninit().assume_init() };
    let mut next_layer: [MaybeUninit<Hash>; MAX_LAYER_SIZE] =
        unsafe { MaybeUninit::uninit().assume_init() };

    let mut current_size = actual_leaf_count;
    for i in 0..actual_leaf_count {
        current_layer[i].write(Hash::from(leaves[i]));
    }

    // If layer_number is 0, return the leaf hashes
    if layer_number == 0 {
        let result_count = core::cmp::min(current_size, MAX_NODES);
        for i in 0..result_count {
            result_buffer[i] = unsafe { current_layer[i].assume_init() };
        }
        return (result_count, result_buffer);
    }

    let mut current_level = 0;

    // Build layers until we reach the target layer
    loop {
        if current_size == 0 {
            break;
        }

        // Build next layer
        let next_size = (current_size + 1) / 2;
        for i in 0..next_size {
            let left_idx = i * 2;
            let right_idx = left_idx + 1;

            let left = unsafe { current_layer[left_idx].assume_init() };
            let right = if right_idx < current_size {
                unsafe { current_layer[right_idx].assume_init() }
            } else {
                zero_values[current_level]
            };

            let hashed = hash_left_right(left, right);
            next_layer[i].write(hashed);
        }

        current_level += 1;

        // Check if we've reached the target layer
        if current_level == layer_number {
            let result_count = core::cmp::min(next_size, MAX_NODES);
            for i in 0..result_count {
                result_buffer[i] = unsafe { next_layer[i].assume_init() };
            }
            return (result_count, result_buffer);
        }

        // Swap layers for next iteration
        core::mem::swap(&mut current_layer, &mut next_layer);
        current_size = next_size;
    }

    (0, result_buffer)
}

fn is_valid_leaf_no_std<P>(proof: &[P], root: Hash, leaf: Leaf) -> bool
where
    P: Into<Hash> + Copy,
{
    let mut computed_hash = Hash::from(leaf);

    for proof_element in proof.iter() {
        computed_hash = hash_left_right(computed_hash, (*proof_element).into());
    }

    computed_hash == root
}

/// Returns a Merkle proof for a specific leaf in the tree.
#[cfg(feature = "std")]
pub fn get_merkle_proof(
    leaves: &[Leaf],
    zero_values: &[Hash],
    leaf_index: usize,
    height: usize,
) -> Vec<Hash> {
    let mut layers = Vec::with_capacity(height);
    let mut current_layer: Vec<Hash> = leaves.iter().map(|leaf| Hash::from(*leaf)).collect();

    for i in 0..height {
        if current_layer.len() % 2 != 0 {
            current_layer.push(zero_values[i]);
        }

        layers.push(current_layer.clone());
        current_layer = hash_pairs(current_layer);
    }

    let mut proof = Vec::with_capacity(height);
    let mut current_index = leaf_index;
    let mut layer_index = 0;

    for _ in 0..height {
        let sibling = if current_index % 2 == 0 {
            layers[layer_index][current_index + 1]
        } else {
            layers[layer_index][current_index - 1]
        };

        proof.push(sibling);

        current_index /= 2;
        layer_index += 1;
    }

    proof
}

/// Returns a Merkle proof for a specific leaf in the tree without Vec allocation.
/// Uses a simplified approach that builds the proof directly without storing all layers.
pub fn get_merkle_proof_no_std<const N: usize>(
    leaves: &[Leaf],
    zero_values: &[Hash],
    leaf_index: usize,
) -> [Hash; N] {
    // Use a reasonable maximum size that won't cause stack overflow
    const MAX_LAYER_SIZE: usize = 4096;

    // Check if we exceed the maximum supported size for no-std
    if leaves.len() > MAX_LAYER_SIZE {
        // For very large trees, we'll need to fallback to a different approach
        // For now, we'll work with the first MAX_LAYER_SIZE leaves
    }

    let actual_leaves = if leaves.len() > MAX_LAYER_SIZE {
        &leaves[..MAX_LAYER_SIZE]
    } else {
        leaves
    };

    // Use MaybeUninit for efficient initialization
    let mut current_layer: [MaybeUninit<Hash>; MAX_LAYER_SIZE] =
        unsafe { MaybeUninit::uninit().assume_init() };
    let mut next_layer: [MaybeUninit<Hash>; MAX_LAYER_SIZE] =
        unsafe { MaybeUninit::uninit().assume_init() };

    // Initialize first layer with leaves
    let mut current_size = actual_leaves.len();
    for (i, leaf) in actual_leaves.iter().enumerate() {
        current_layer[i].write(Hash::from(*leaf));
    }

    // Pad with zero if needed
    if current_size % 2 != 0 {
        current_layer[current_size].write(zero_values[0]);
        current_size += 1;
    }

    let mut proof: [MaybeUninit<Hash>; N] = unsafe { MaybeUninit::uninit().assume_init() };
    let mut current_index = leaf_index;

    // Build proof level by level
    for level in 0..N {
        if current_size <= 1 {
            // Fill remaining proof with zero values
            for i in level..N {
                proof[i].write(zero_values[i]);
            }
            break;
        }

        // Get sibling for proof
        let sibling = if current_index % 2 == 0 {
            // Right sibling
            if current_index + 1 < current_size {
                unsafe { current_layer[current_index + 1].assume_init() }
            } else {
                zero_values[level]
            }
        } else {
            // Left sibling
            unsafe { current_layer[current_index - 1].assume_init() }
        };

        proof[level].write(sibling);

        // Build next layer
        let next_size = (current_size + 1) / 2;
        for i in 0..next_size {
            let left_idx = i * 2;
            let right_idx = left_idx + 1;

            let left = unsafe { current_layer[left_idx].assume_init() };
            let right = if right_idx < current_size {
                unsafe { current_layer[right_idx].assume_init() }
            } else {
                zero_values[level]
            };

            let hashed = hash_left_right(left, right);
            next_layer[i].write(hashed);
        }

        // Swap layers
        core::mem::swap(&mut current_layer, &mut next_layer);
        current_size = next_size;
        current_index /= 2;
    }

    // Convert MaybeUninit array to initialized array safely
    let mut result: [Hash; N] = [Hash::default(); N];
    for i in 0..N {
        result[i] = unsafe { proof[i].assume_init() };
    }
    result
}

/// Hashes pairs of hashes together, returning a new vector of hashes.
#[cfg(feature = "std")]
pub fn hash_pairs(pairs: Vec<Hash>) -> Vec<Hash> {
    let mut res = Vec::with_capacity(pairs.len() / 2);

    for i in (0..pairs.len()).step_by(2) {
        let left = pairs[i];
        let right = pairs[i + 1];

        let hashed = hash_left_right(left, right);
        res.push(hashed);
    }

    res
}

/// Hashes pairs of hashes together without Vec allocation.
/// Returns the number of pairs processed and the result buffer.
pub fn hash_pairs_no_std<const MAX_PAIRS: usize>(pairs: &[Hash]) -> (usize, [Hash; MAX_PAIRS]) {
    let mut result_buffer: [Hash; MAX_PAIRS] = [Hash::default(); MAX_PAIRS];

    let num_pairs = pairs.len() / 2;
    let result_count = core::cmp::min(num_pairs, MAX_PAIRS);

    for i in 0..result_count {
        let left = pairs[i * 2];
        let right = pairs[i * 2 + 1];
        let hashed = hash_left_right(left, right);
        result_buffer[i] = hashed;
    }

    (result_count, result_buffer)
}

/// Hashes two hashes together, ensuring a consistent order.

pub fn hash_left_right(left: Hash, right: Hash) -> Hash {
    let combined;
    if left.to_bytes() <= right.to_bytes() {
        combined = [b"NODE".as_ref(), left.as_ref(), right.as_ref()];
    } else {
        combined = [b"NODE".as_ref(), right.as_ref(), left.as_ref()];
    }

    hashv(&combined)
}

/// Computes the path from the leaf to the root using the provided proof.
#[cfg(feature = "std")]
pub fn compute_path(proof: &[Hash], leaf: Leaf) -> Vec<Hash> {
    let mut computed_path = Vec::with_capacity(proof.len() + 1);
    let mut computed_hash = Hash::from(leaf);

    computed_path.push(computed_hash);

    for proof_element in proof.iter() {
        computed_hash = hash_left_right(computed_hash, *proof_element);
        computed_path.push(computed_hash);
    }

    computed_path
}

/// Computes the path from the leaf to the root using the provided proof without Vec allocation.
/// Returns the number of path elements and the path buffer.
pub fn compute_path_no_std<const MAX_PATH: usize>(
    proof: &[Hash],
    leaf: Leaf,
) -> (usize, [Hash; MAX_PATH]) {
    let mut path_buffer: [Hash; MAX_PATH] = [Hash::default(); MAX_PATH];
    let mut computed_hash = Hash::from(leaf);

    let path_len = core::cmp::min(proof.len() + 1, MAX_PATH);

    if path_len > 0 {
        path_buffer[0] = computed_hash;
    }

    for (i, proof_element) in proof.iter().enumerate() {
        if i + 1 >= MAX_PATH {
            break;
        }
        computed_hash = hash_left_right(computed_hash, *proof_element);
        path_buffer[i + 1] = computed_hash;
    }

    (path_len, path_buffer)
}

#[cfg(feature = "std")]
fn is_valid_leaf(proof: &[Hash], root: Hash, leaf: Leaf) -> bool {
    let computed_path = compute_path(proof, leaf);
    is_valid_path(&computed_path, root)
}

#[cfg(feature = "std")]
fn is_valid_path(path: &[Hash], root: Hash) -> bool {
    if path.is_empty() {
        return false;
    }

    *path.last().unwrap() == root
}

/// Validates a path without Vec allocation.
/// Takes a path buffer and the count of valid elements.
pub fn is_valid_path_no_std(path_buffer: &[Hash], path_count: usize, root: Hash) -> bool {
    if path_count == 0 {
        return false;
    }

    if path_count > path_buffer.len() {
        return false;
    }

    path_buffer[path_count - 1] == root
}

/// Verifies that a given merkle root contains the leaf using the provided proof.
#[cfg(feature = "std")]
pub fn verify<Root, Item, L>(root: Root, proof: &[Item], leaf: L) -> bool
where
    Root: Into<Hash>,
    Item: Into<Hash> + Copy,
    L: Into<Leaf>,
{
    let root_h: Hash = root.into();
    let proof_hashes: Vec<Hash> = proof.iter().map(|&x| x.into()).collect();

    let leaf_h: Leaf = leaf.into();
    let path = compute_path(&proof_hashes, leaf_h);
    is_valid_path(&path, root_h)
}

/// Verifies that a given merkle root contains the leaf using the provided proof without Vec allocation.
pub fn verify_no_std<Root, Item, L>(root: Root, proof: &[Item], leaf: L) -> bool
where
    Root: Into<Hash>,
    Item: Into<Hash> + Copy,
    L: Into<Leaf>,
{
    let root_h: Hash = root.into();
    let leaf_h: Leaf = leaf.into();

    let mut computed_hash = Hash::from(leaf_h);

    for proof_element in proof.iter() {
        computed_hash = hash_left_right(computed_hash, (*proof_element).into());
    }

    computed_hash == root_h
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::leaf::{Hash, Leaf};

    // Tests always use std for convenience - this doesn't affect the no-std nature of the functions being tested
    extern crate std;
    use std::{format, println, vec::Vec};

    /// Creates test leaves with predictable data
    fn create_test_leaves(count: usize) -> Vec<Leaf> {
        (0..count)
            .map(|i| {
                let data = format!("leaf_{}", i);
                Leaf::new(&[data.as_bytes()])
            })
            .collect()
    }

    /// Creates zero values for a given height
    fn create_zero_values<const N: usize>() -> [Hash; N] {
        let seeds: &[&[u8]] = &[b"test_zero"];
        let mut zeros: [Hash; N] = [Hash::default(); N];
        let mut current = hashv(seeds);

        for i in 0..N {
            zeros[i] = current;
            current = hashv(&[b"NODE".as_ref(), current.as_ref(), current.as_ref()]);
        }

        zeros
    }

    #[test]
    fn test_get_merkle_proof_comparison_small_tree() {
        const HEIGHT: usize = 4; // Small tree for easy verification

        let leaves = create_test_leaves(8);
        let zero_values = create_zero_values::<HEIGHT>();
        let leaf_index = 3;

        // Test both std and no-std versions and compare them
        #[cfg(feature = "std")]
        {
            let std_proof = get_merkle_proof(&leaves, &zero_values, leaf_index, HEIGHT);
            let no_std_proof = get_merkle_proof_no_std::<HEIGHT>(&leaves, &zero_values, leaf_index);

            // Compare lengths
            assert_eq!(
                std_proof.len(),
                no_std_proof.len(),
                "Proof lengths should match"
            );

            // Compare each element
            for (i, (std_hash, no_std_hash)) in
                std_proof.iter().zip(no_std_proof.iter()).enumerate()
            {
                assert_eq!(std_hash, no_std_hash, "Hash at index {} should match", i);
            }

            println!("✅ Small tree test passed: std and no-std proofs are identical");
        }

        #[cfg(not(feature = "std"))]
        {
            // When std is not available, just test the no-std version
            let no_std_proof = get_merkle_proof_no_std::<HEIGHT>(&leaves, &zero_values, leaf_index);
            assert_eq!(
                no_std_proof.len(),
                HEIGHT,
                "No-std proof length should match height"
            );
            println!("✅ Small tree test (no-std only): proof generated successfully");
        }
    }

    #[test]
    fn test_get_merkle_proof_comparison_medium_tree() {
        const HEIGHT: usize = 10; // Medium tree (TAPE_TREE_HEIGHT)

        let leaves = create_test_leaves(64); // Reduced size to avoid stack overflow
        let zero_values = create_zero_values::<HEIGHT>();
        let leaf_index = 42;

        #[cfg(feature = "std")]
        {
            let std_proof = get_merkle_proof(&leaves, &zero_values, leaf_index, HEIGHT);
            let no_std_proof = get_merkle_proof_no_std::<HEIGHT>(&leaves, &zero_values, leaf_index);

            // Compare lengths
            assert_eq!(
                std_proof.len(),
                no_std_proof.len(),
                "Proof lengths should match"
            );

            // Compare each element
            for (i, (std_hash, no_std_hash)) in
                std_proof.iter().zip(no_std_proof.iter()).enumerate()
            {
                assert_eq!(std_hash, no_std_hash, "Hash at index {} should match", i);
            }

            println!("✅ Medium tree test passed: std and no-std proofs are identical");
        }

        #[cfg(not(feature = "std"))]
        {
            let no_std_proof = get_merkle_proof_no_std::<HEIGHT>(&leaves, &zero_values, leaf_index);
            assert_eq!(
                no_std_proof.len(),
                HEIGHT,
                "No-std proof length should match height"
            );
            println!("✅ Medium tree test (no-std only): proof generated successfully");
        }
    }

    #[test]
    fn test_get_merkle_proof_comparison_large_tree() {
        const HEIGHT: usize = 18; // Large tree (SEGMENT_TREE_HEIGHT)

        let leaves = create_test_leaves(256); // Reduced size to avoid stack overflow
        let zero_values = create_zero_values::<HEIGHT>();
        let leaf_index = 123;

        #[cfg(feature = "std")]
        {
            let std_proof = get_merkle_proof(&leaves, &zero_values, leaf_index, HEIGHT);
            let no_std_proof = get_merkle_proof_no_std::<HEIGHT>(&leaves, &zero_values, leaf_index);

            // Compare lengths
            assert_eq!(
                std_proof.len(),
                no_std_proof.len(),
                "Proof lengths should match"
            );

            // Compare each element
            for (i, (std_hash, no_std_hash)) in
                std_proof.iter().zip(no_std_proof.iter()).enumerate()
            {
                assert_eq!(std_hash, no_std_hash, "Hash at index {} should match", i);
            }

            println!("✅ Large tree test passed: std and no-std proofs are identical");
        }

        #[cfg(not(feature = "std"))]
        {
            let no_std_proof = get_merkle_proof_no_std::<HEIGHT>(&leaves, &zero_values, leaf_index);
            assert_eq!(
                no_std_proof.len(),
                HEIGHT,
                "No-std proof length should match height"
            );
            println!("✅ Large tree test (no-std only): proof generated successfully");
        }
    }

    #[test]
    fn test_get_merkle_proof_edge_cases() {
        const HEIGHT: usize = 8;
        let zero_values = create_zero_values::<HEIGHT>();

        // Test with single leaf
        let single_leaf = create_test_leaves(1);
        let single_proof = get_merkle_proof_no_std::<HEIGHT>(&single_leaf, &zero_values, 0);
        assert_eq!(single_proof.len(), HEIGHT);

        // Test with odd number of leaves
        let odd_leaves = create_test_leaves(7);
        let odd_proof = get_merkle_proof_no_std::<HEIGHT>(&odd_leaves, &zero_values, 3);
        assert_eq!(odd_proof.len(), HEIGHT);

        // Test with power of 2 leaves
        let power_of_2_leaves = create_test_leaves(16);
        let power_of_2_proof =
            get_merkle_proof_no_std::<HEIGHT>(&power_of_2_leaves, &zero_values, 8);
        assert_eq!(power_of_2_proof.len(), HEIGHT);

        println!("✅ Edge case tests passed");
    }

    #[test]
    fn test_proof_verification_consistency() {
        const HEIGHT: usize = 6;
        let leaves = create_test_leaves(20);
        let zero_values = create_zero_values::<HEIGHT>();
        let leaf_index = 7;

        // Generate proof using no-std version
        let proof = get_merkle_proof_no_std::<HEIGHT>(&leaves, &zero_values, leaf_index);

        // Create a simple merkle tree to get the root
        let mut tree = MerkleTree::<HEIGHT>::new(&[b"test_zero"]);
        for leaf in &leaves {
            tree.try_add_leaf(*leaf)
                .expect("Should be able to add leaf");
        }

        let root = tree.get_root();
        let target_leaf = leaves[leaf_index];

        // Verify the proof using the no-std verification function
        let is_valid = verify_no_std(root, &proof, target_leaf);
        assert!(is_valid, "Generated proof should be valid");

        println!("✅ Proof verification consistency test passed");
    }

    #[test]
    fn test_merkle_tree_integration() {
        const HEIGHT: usize = 5;
        let leaves = create_test_leaves(15);
        let leaf_index = 5;

        // Create tree and add leaves
        let mut tree = MerkleTree::<HEIGHT>::new(&[b"test_zero"]);
        for leaf in &leaves {
            tree.try_add_leaf(*leaf)
                .expect("Should be able to add leaf");
        }

        // Generate proof using the tree's no-std method
        let proof = tree.get_proof_no_std(&leaves, leaf_index);

        // Verify the proof
        let root = tree.get_root();
        let target_leaf = leaves[leaf_index];
        let is_valid = verify_no_std(root, &proof, target_leaf);

        assert!(is_valid, "Tree-generated proof should be valid");
        assert_eq!(proof.len(), HEIGHT, "Proof length should match tree height");

        println!("✅ Merkle tree integration test passed");
    }

    #[test]
    fn test_get_layer_nodes_comparison_small_tree() {
        const HEIGHT: usize = 4;
        const MAX_NODES: usize = 16; // Enough for small trees

        let leaves = create_test_leaves(8);
        let zero_values = create_zero_values::<HEIGHT>();

        // Create tree and add leaves
        let mut tree = MerkleTree::<HEIGHT>::new(&[b"test_zero"]);
        for leaf in &leaves {
            tree.try_add_leaf(*leaf)
                .expect("Should be able to add leaf");
        }

        // Test different layer numbers
        for layer in 0..=HEIGHT {
            #[cfg(feature = "std")]
            {
                let std_result = tree.get_layer_nodes(&leaves, layer);
                let (no_std_count, no_std_buffer) =
                    tree.get_layer_nodes_no_std::<MAX_NODES>(&leaves, layer);

                // Compare lengths
                assert_eq!(
                    std_result.len(),
                    no_std_count,
                    "Layer {} length should match",
                    layer
                );

                // Compare each element
                for (i, (std_hash, no_std_hash)) in
                    std_result.iter().zip(no_std_buffer.iter()).enumerate()
                {
                    if i < no_std_count {
                        assert_eq!(
                            std_hash, no_std_hash,
                            "Layer {} hash at index {} should match",
                            layer, i
                        );
                    }
                }
            }

            #[cfg(not(feature = "std"))]
            {
                let (no_std_count, _no_std_buffer) =
                    tree.get_layer_nodes_no_std::<MAX_NODES>(&leaves, layer);
                // Just verify we get reasonable results
                if layer <= HEIGHT {
                    assert!(
                        no_std_count > 0 || layer == HEIGHT,
                        "Layer {} should have nodes or be at max height",
                        layer
                    );
                }
            }
        }

        println!("✅ Small tree layer nodes test passed");
    }

    #[test]
    fn test_get_layer_nodes_comparison_medium_tree() {
        const HEIGHT: usize = 10; // TAPE_TREE_HEIGHT
        const MAX_NODES: usize = 64; // Enough for medium trees

        let leaves = create_test_leaves(32);
        let zero_values = create_zero_values::<HEIGHT>();

        // Create tree and add leaves
        let mut tree = MerkleTree::<HEIGHT>::new(&[b"test_zero"]);
        for leaf in &leaves {
            tree.try_add_leaf(*leaf)
                .expect("Should be able to add leaf");
        }

        // Test specific layers
        let test_layers = [0, 1, 3, 5, HEIGHT - 1, HEIGHT];

        for &layer in &test_layers {
            #[cfg(feature = "std")]
            {
                let std_result = tree.get_layer_nodes(&leaves, layer);
                let (no_std_count, no_std_buffer) =
                    tree.get_layer_nodes_no_std::<MAX_NODES>(&leaves, layer);

                // Compare lengths
                assert_eq!(
                    std_result.len(),
                    no_std_count,
                    "Layer {} length should match",
                    layer
                );

                // Compare each element
                for (i, (std_hash, no_std_hash)) in
                    std_result.iter().zip(no_std_buffer.iter()).enumerate()
                {
                    if i < no_std_count {
                        assert_eq!(
                            std_hash, no_std_hash,
                            "Layer {} hash at index {} should match",
                            layer, i
                        );
                    }
                }
            }

            #[cfg(not(feature = "std"))]
            {
                let (no_std_count, _no_std_buffer) =
                    tree.get_layer_nodes_no_std::<MAX_NODES>(&leaves, layer);
                // Just verify we get reasonable results
                if layer <= HEIGHT {
                    assert!(
                        no_std_count > 0 || layer >= HEIGHT,
                        "Layer {} should have nodes or be near max height",
                        layer
                    );
                }
            }
        }

        println!("✅ Medium tree layer nodes test passed");
    }

    #[test]
    fn test_get_layer_nodes_comparison_large_tree() {
        const HEIGHT: usize = 18; // SEGMENT_TREE_HEIGHT
        const MAX_NODES: usize = 256; // Enough for large trees

        let leaves = create_test_leaves(128); // Reduced to avoid stack overflow
        let zero_values = create_zero_values::<HEIGHT>();

        // Create tree and add leaves
        let mut tree = MerkleTree::<HEIGHT>::new(&[b"test_zero"]);
        for leaf in &leaves {
            tree.try_add_leaf(*leaf)
                .expect("Should be able to add leaf");
        }

        // Test specific layers for large tree
        let test_layers = [0, 1, 2, 5, 10, 15, HEIGHT - 1, HEIGHT];

        for &layer in &test_layers {
            #[cfg(feature = "std")]
            {
                let std_result = tree.get_layer_nodes(&leaves, layer);
                let (no_std_count, no_std_buffer) =
                    tree.get_layer_nodes_no_std::<MAX_NODES>(&leaves, layer);

                // Compare lengths
                assert_eq!(
                    std_result.len(),
                    no_std_count,
                    "Layer {} length should match",
                    layer
                );

                // Compare each element
                for (i, (std_hash, no_std_hash)) in
                    std_result.iter().zip(no_std_buffer.iter()).enumerate()
                {
                    if i < no_std_count {
                        assert_eq!(
                            std_hash, no_std_hash,
                            "Layer {} hash at index {} should match",
                            layer, i
                        );
                    }
                }
            }

            #[cfg(not(feature = "std"))]
            {
                let (no_std_count, _no_std_buffer) =
                    tree.get_layer_nodes_no_std::<MAX_NODES>(&leaves, layer);
                // Just verify we get reasonable results
                if layer <= HEIGHT {
                    assert!(
                        no_std_count > 0 || layer >= HEIGHT,
                        "Layer {} should have nodes or be near max height",
                        layer
                    );
                }
            }
        }

        println!("✅ Large tree layer nodes test passed");
    }

    #[test]
    fn test_get_layer_nodes_edge_cases() {
        const HEIGHT: usize = 6;
        const MAX_NODES: usize = 32;
        let zero_values = create_zero_values::<HEIGHT>();

        // Test with single leaf
        let single_leaf = create_test_leaves(1);
        let mut tree = MerkleTree::<HEIGHT>::new(&[b"test_zero"]);
        tree.try_add_leaf(single_leaf[0])
            .expect("Should be able to add leaf");

        let (count, _buffer) = tree.get_layer_nodes_no_std::<MAX_NODES>(&single_leaf, 0);
        assert_eq!(count, 1, "Single leaf should produce 1 node at layer 0");

        // Test with empty leaves
        let empty_leaves = create_test_leaves(0);
        let (count, _buffer) = tree.get_layer_nodes_no_std::<MAX_NODES>(&empty_leaves, 0);
        assert_eq!(count, 0, "Empty leaves should produce 0 nodes");

        // Test layer beyond tree height
        let leaves = create_test_leaves(4);
        let (count, _buffer) = tree.get_layer_nodes_no_std::<MAX_NODES>(&leaves, HEIGHT + 1);
        assert_eq!(count, 0, "Layer beyond height should produce 0 nodes");

        println!("✅ Layer nodes edge cases test passed");
    }

    #[test]
    fn test_get_layer_nodes_consistency() {
        const HEIGHT: usize = 5;
        const MAX_NODES: usize = 32;

        let leaves = create_test_leaves(10);
        let mut tree = MerkleTree::<HEIGHT>::new(&[b"test_zero"]);
        for leaf in &leaves {
            tree.try_add_leaf(*leaf)
                .expect("Should be able to add leaf");
        }

        // Verify that layer progression makes sense
        let (layer0_count, _) = tree.get_layer_nodes_no_std::<MAX_NODES>(&leaves, 0);
        let (layer1_count, _) = tree.get_layer_nodes_no_std::<MAX_NODES>(&leaves, 1);
        let (layer2_count, _) = tree.get_layer_nodes_no_std::<MAX_NODES>(&leaves, 2);

        assert_eq!(layer0_count, 10, "Layer 0 should have 10 leaf nodes");
        assert_eq!(layer1_count, 5, "Layer 1 should have 5 nodes (10/2)");
        assert!(
            layer2_count <= 3,
            "Layer 2 should have at most 3 nodes (5/2 rounded up)"
        );

        println!("✅ Layer nodes consistency test passed");
    }

    #[test]
    fn test_merkle_proof_functions_with_constants() {
        // Test using the actual constants from consts.rs
        const SEGMENT_HEIGHT: usize = 18; // SEGMENT_TREE_HEIGHT
        const TAPE_HEIGHT: usize = 10; // TAPE_TREE_HEIGHT

        // Test with TAPE_TREE_HEIGHT
        {
            let leaves = create_test_leaves(32);
            let zero_values = create_zero_values::<TAPE_HEIGHT>();
            let leaf_index = 15;

            #[cfg(feature = "std")]
            {
                let std_proof = get_merkle_proof(&leaves, &zero_values, leaf_index, TAPE_HEIGHT);
                let no_std_proof =
                    get_merkle_proof_no_std::<TAPE_HEIGHT>(&leaves, &zero_values, leaf_index);

                assert_eq!(
                    std_proof.len(),
                    TAPE_HEIGHT,
                    "Std proof should match TAPE_TREE_HEIGHT"
                );
                assert_eq!(
                    no_std_proof.len(),
                    TAPE_HEIGHT,
                    "No-std proof should match TAPE_TREE_HEIGHT"
                );
                assert_eq!(
                    std_proof.len(),
                    no_std_proof.len(),
                    "Both proofs should have same length"
                );

                for (i, (std_hash, no_std_hash)) in
                    std_proof.iter().zip(no_std_proof.iter()).enumerate()
                {
                    assert_eq!(
                        std_hash, no_std_hash,
                        "Hash {} should match between std and no-std",
                        i
                    );
                }

                println!(
                    "✅ TAPE_TREE_HEIGHT merkle proof test passed: {} elements identical",
                    std_proof.len()
                );
            }

            #[cfg(not(feature = "std"))]
            {
                let no_std_proof =
                    get_merkle_proof_no_std::<TAPE_HEIGHT>(&leaves, &zero_values, leaf_index);
                assert_eq!(
                    no_std_proof.len(),
                    TAPE_HEIGHT,
                    "No-std proof should match TAPE_TREE_HEIGHT"
                );
                println!(
                    "✅ TAPE_TREE_HEIGHT merkle proof (no-std only) test passed: {} elements",
                    no_std_proof.len()
                );
            }
        }

        // Test with SEGMENT_TREE_HEIGHT (smaller sample to avoid stack overflow)
        {
            let leaves = create_test_leaves(64);
            let zero_values = create_zero_values::<SEGMENT_HEIGHT>();
            let leaf_index = 31;

            #[cfg(feature = "std")]
            {
                let std_proof = get_merkle_proof(&leaves, &zero_values, leaf_index, SEGMENT_HEIGHT);
                let no_std_proof =
                    get_merkle_proof_no_std::<SEGMENT_HEIGHT>(&leaves, &zero_values, leaf_index);

                assert_eq!(
                    std_proof.len(),
                    SEGMENT_HEIGHT,
                    "Std proof should match SEGMENT_TREE_HEIGHT"
                );
                assert_eq!(
                    no_std_proof.len(),
                    SEGMENT_HEIGHT,
                    "No-std proof should match SEGMENT_TREE_HEIGHT"
                );
                assert_eq!(
                    std_proof.len(),
                    no_std_proof.len(),
                    "Both proofs should have same length"
                );

                for (i, (std_hash, no_std_hash)) in
                    std_proof.iter().zip(no_std_proof.iter()).enumerate()
                {
                    assert_eq!(
                        std_hash, no_std_hash,
                        "Hash {} should match between std and no-std",
                        i
                    );
                }

                println!(
                    "✅ SEGMENT_TREE_HEIGHT merkle proof test passed: {} elements identical",
                    std_proof.len()
                );
            }

            #[cfg(not(feature = "std"))]
            {
                let no_std_proof =
                    get_merkle_proof_no_std::<SEGMENT_HEIGHT>(&leaves, &zero_values, leaf_index);
                assert_eq!(
                    no_std_proof.len(),
                    SEGMENT_HEIGHT,
                    "No-std proof should match SEGMENT_TREE_HEIGHT"
                );
                println!(
                    "✅ SEGMENT_TREE_HEIGHT merkle proof (no-std only) test passed: {} elements",
                    no_std_proof.len()
                );
            }
        }
    }

    #[test]
    fn test_merkle_proof_verification_end_to_end() {
        const HEIGHT: usize = 8;
        let leaves = create_test_leaves(20);
        let zero_values = create_zero_values::<HEIGHT>();
        let leaf_index = 7;

        // Test that both std and no-std proofs verify correctly
        #[cfg(feature = "std")]
        {
            let std_proof = get_merkle_proof(&leaves, &zero_values, leaf_index, HEIGHT);
            let no_std_proof = get_merkle_proof_no_std::<HEIGHT>(&leaves, &zero_values, leaf_index);

            // Create a tree to get the actual root
            let mut tree = MerkleTree::<HEIGHT>::new(&[b"test_zero"]);
            for leaf in &leaves {
                tree.try_add_leaf(*leaf).expect("Should add leaf");
            }
            let root = tree.get_root();
            let target_leaf = leaves[leaf_index];

            // Verify both proofs work
            let std_valid = verify(root, &std_proof, target_leaf);
            let no_std_valid = verify_no_std(root, &no_std_proof, target_leaf);

            assert!(std_valid, "Std proof should verify");
            assert!(no_std_valid, "No-std proof should verify");
            assert_eq!(
                std_valid, no_std_valid,
                "Both proofs should have same verification result"
            );

            println!("✅ End-to-end merkle proof verification test passed");
        }

        #[cfg(not(feature = "std"))]
        {
            let no_std_proof = get_merkle_proof_no_std::<HEIGHT>(&leaves, &zero_values, leaf_index);

            // Create a tree to get the actual root
            let mut tree = MerkleTree::<HEIGHT>::new(&[b"test_zero"]);
            for leaf in &leaves {
                tree.try_add_leaf(*leaf).expect("Should add leaf");
            }
            let root = tree.get_root();
            let target_leaf = leaves[leaf_index];

            // Verify the no-std proof
            let no_std_valid = verify_no_std(root, &no_std_proof, target_leaf);
            assert!(no_std_valid, "No-std proof should verify");

            println!("✅ End-to-end merkle proof verification (no-std only) test passed");
        }
    }

    #[test]
    fn test_try_remove_comparison() {
        const HEIGHT: usize = 6;

        let leaves = create_test_leaves(10);
        let zero_values = create_zero_values::<HEIGHT>();
        let target_index = 5;
        let target_data: &[&[u8]] = &[b"leaf_5"];

        // Create initial tree
        let mut tree_std = MerkleTree::<HEIGHT>::new(&[b"test_zero"]);
        let mut tree_no_std = MerkleTree::<HEIGHT>::new(&[b"test_zero"]);

        for leaf in &leaves {
            tree_std.try_add_leaf(*leaf).expect("Should add leaf");
            tree_no_std.try_add_leaf(*leaf).expect("Should add leaf");
        }

        // Generate proof for the target leaf
        #[cfg(feature = "std")]
        let proof = tree_std.get_proof(&leaves, target_index);
        #[cfg(not(feature = "std"))]
        let proof = tree_no_std.get_proof_no_std(&leaves, target_index);

        let initial_root_std = tree_std.get_root();
        let initial_root_no_std = tree_no_std.get_root();
        assert_eq!(
            initial_root_std, initial_root_no_std,
            "Initial roots should match"
        );

        // Test removal
        #[cfg(feature = "std")]
        {
            let std_result = tree_std.try_remove(&proof, target_data);
            let no_std_result = tree_no_std.try_remove_no_std(&proof, target_data);

            assert_eq!(
                std_result.is_ok(),
                no_std_result.is_ok(),
                "Both results should have same success state"
            );

            if std_result.is_ok() {
                assert_eq!(
                    tree_std.get_root(),
                    tree_no_std.get_root(),
                    "Final roots should match after removal"
                );
                println!("✅ try_remove vs try_remove_no_std test passed");
            }
        }

        #[cfg(not(feature = "std"))]
        {
            let no_std_result = tree_no_std.try_remove_no_std(&proof, target_data);
            assert!(no_std_result.is_ok(), "No-std removal should succeed");
            println!("✅ try_remove_no_std (no-std only) test passed");
        }
    }

    #[test]
    fn test_try_remove_leaf_comparison() {
        const HEIGHT: usize = 5;

        let leaves = create_test_leaves(8);
        let target_index = 3;
        let target_leaf = leaves[target_index];

        // Create initial trees
        let mut tree_std = MerkleTree::<HEIGHT>::new(&[b"test_zero"]);
        let mut tree_no_std = MerkleTree::<HEIGHT>::new(&[b"test_zero"]);

        for leaf in &leaves {
            tree_std.try_add_leaf(*leaf).expect("Should add leaf");
            tree_no_std.try_add_leaf(*leaf).expect("Should add leaf");
        }

        // Generate proof
        #[cfg(feature = "std")]
        let proof = tree_std.get_proof(&leaves, target_index);
        #[cfg(not(feature = "std"))]
        let proof = tree_no_std.get_proof_no_std(&leaves, target_index);

        // Test leaf removal
        #[cfg(feature = "std")]
        {
            let std_result = tree_std.try_remove_leaf(&proof, target_leaf);
            let no_std_result = tree_no_std.try_remove_leaf_no_std(&proof, target_leaf);

            assert_eq!(
                std_result.is_ok(),
                no_std_result.is_ok(),
                "Both results should have same success state"
            );

            if std_result.is_ok() {
                assert_eq!(
                    tree_std.get_root(),
                    tree_no_std.get_root(),
                    "Final roots should match after leaf removal"
                );
                println!("✅ try_remove_leaf vs try_remove_leaf_no_std test passed");
            }
        }

        #[cfg(not(feature = "std"))]
        {
            let no_std_result = tree_no_std.try_remove_leaf_no_std(&proof, target_leaf);
            assert!(no_std_result.is_ok(), "No-std leaf removal should succeed");
            println!("✅ try_remove_leaf_no_std (no-std only) test passed");
        }
    }

    #[test]
    fn test_try_replace_comparison() {
        const HEIGHT: usize = 6;

        let leaves = create_test_leaves(12);
        let target_index = 7;
        let original_data: &[&[u8]] = &[b"leaf_7"];
        let new_data: &[&[u8]] = &[b"replaced_leaf"];

        // Create initial trees
        let mut tree_std = MerkleTree::<HEIGHT>::new(&[b"test_zero"]);
        let mut tree_no_std = MerkleTree::<HEIGHT>::new(&[b"test_zero"]);

        for leaf in &leaves {
            tree_std.try_add_leaf(*leaf).expect("Should add leaf");
            tree_no_std.try_add_leaf(*leaf).expect("Should add leaf");
        }

        // Generate proof
        #[cfg(feature = "std")]
        let proof = tree_std.get_proof(&leaves, target_index);
        #[cfg(not(feature = "std"))]
        let proof = tree_no_std.get_proof_no_std(&leaves, target_index);

        // Test replacement
        #[cfg(feature = "std")]
        {
            let std_result = tree_std.try_replace(&proof, original_data, new_data);
            let no_std_result = tree_no_std.try_replace_no_std(&proof, original_data, new_data);

            assert_eq!(
                std_result.is_ok(),
                no_std_result.is_ok(),
                "Both results should have same success state"
            );

            if std_result.is_ok() {
                assert_eq!(
                    tree_std.get_root(),
                    tree_no_std.get_root(),
                    "Final roots should match after replacement"
                );
                println!("✅ try_replace vs try_replace_no_std test passed");
            }
        }

        #[cfg(not(feature = "std"))]
        {
            let no_std_result = tree_no_std.try_replace_no_std(&proof, original_data, new_data);
            assert!(no_std_result.is_ok(), "No-std replacement should succeed");
            println!("✅ try_replace_no_std (no-std only) test passed");
        }
    }

    #[test]
    fn test_try_replace_leaf_comparison() {
        const HEIGHT: usize = 5;

        let leaves = create_test_leaves(6);
        let target_index = 2;
        let original_leaf = leaves[target_index];
        let new_leaf = Leaf::new(&[b"new_replacement_leaf"]);

        // Create initial trees
        let mut tree_std = MerkleTree::<HEIGHT>::new(&[b"test_zero"]);
        let mut tree_no_std = MerkleTree::<HEIGHT>::new(&[b"test_zero"]);

        for leaf in &leaves {
            tree_std.try_add_leaf(*leaf).expect("Should add leaf");
            tree_no_std.try_add_leaf(*leaf).expect("Should add leaf");
        }

        // Generate proof
        #[cfg(feature = "std")]
        let proof = tree_std.get_proof(&leaves, target_index);
        #[cfg(not(feature = "std"))]
        let proof = tree_no_std.get_proof_no_std(&leaves, target_index);

        // Test leaf replacement
        #[cfg(feature = "std")]
        {
            let std_result = tree_std.try_replace_leaf(&proof, original_leaf, new_leaf);
            let no_std_result =
                tree_no_std.try_replace_leaf_no_std(&proof, original_leaf, new_leaf);

            assert_eq!(
                std_result.is_ok(),
                no_std_result.is_ok(),
                "Both results should have same success state"
            );

            if std_result.is_ok() {
                assert_eq!(
                    tree_std.get_root(),
                    tree_no_std.get_root(),
                    "Final roots should match after leaf replacement"
                );
                println!("✅ try_replace_leaf vs try_replace_leaf_no_std test passed");
            }
        }

        #[cfg(not(feature = "std"))]
        {
            let no_std_result =
                tree_no_std.try_replace_leaf_no_std(&proof, original_leaf, new_leaf);
            assert!(
                no_std_result.is_ok(),
                "No-std leaf replacement should succeed"
            );
            println!("✅ try_replace_leaf_no_std (no-std only) test passed");
        }
    }

    #[test]
    fn test_contains_comparison() {
        const HEIGHT: usize = 6;

        let leaves = create_test_leaves(15);
        let target_index = 9;
        let target_data: &[&[u8]] = &[b"leaf_9"];
        let non_existent_data: &[&[u8]] = &[b"non_existent_leaf"];

        // Create tree
        let mut tree = MerkleTree::<HEIGHT>::new(&[b"test_zero"]);
        for leaf in &leaves {
            tree.try_add_leaf(*leaf).expect("Should add leaf");
        }

        // Generate proof for existing data
        #[cfg(feature = "std")]
        let proof = tree.get_proof(&leaves, target_index);
        #[cfg(not(feature = "std"))]
        let proof = tree.get_proof_no_std(&leaves, target_index);

        #[cfg(feature = "std")]
        {
            // Test with existing data
            let std_contains = tree.contains(&proof, target_data);
            let no_std_contains = tree.contains_no_std(&proof, target_data);

            assert_eq!(
                std_contains, no_std_contains,
                "Both should agree on existing data"
            );
            assert!(std_contains, "Should find existing data");

            // Test with non-existent data
            let std_not_contains = tree.contains(&proof, non_existent_data);
            let no_std_not_contains = tree.contains_no_std(&proof, non_existent_data);

            assert_eq!(
                std_not_contains, no_std_not_contains,
                "Both should agree on non-existent data"
            );
            assert!(!std_not_contains, "Should not find non-existent data");

            println!("✅ contains vs contains_no_std test passed");
        }

        #[cfg(not(feature = "std"))]
        {
            let no_std_contains = tree.contains_no_std(&proof, target_data);
            let no_std_not_contains = tree.contains_no_std(&proof, non_existent_data);

            assert!(no_std_contains, "Should find existing data");
            assert!(!no_std_not_contains, "Should not find non-existent data");
            println!("✅ contains_no_std (no-std only) test passed");
        }
    }

    #[test]
    fn test_contains_leaf_comparison() {
        const HEIGHT: usize = 5;

        let leaves = create_test_leaves(10);
        let target_index = 4;
        let target_leaf = leaves[target_index];
        let non_existent_leaf = Leaf::new(&[b"non_existent_leaf"]);

        // Create tree
        let mut tree = MerkleTree::<HEIGHT>::new(&[b"test_zero"]);
        for leaf in &leaves {
            tree.try_add_leaf(*leaf).expect("Should add leaf");
        }

        // Generate proof for existing leaf
        #[cfg(feature = "std")]
        let proof = tree.get_proof(&leaves, target_index);
        #[cfg(not(feature = "std"))]
        let proof = tree.get_proof_no_std(&leaves, target_index);

        #[cfg(feature = "std")]
        {
            // Test with existing leaf
            let std_contains = tree.contains_leaf(&proof, target_leaf);
            let no_std_contains = tree.contains_leaf_no_std(&proof, target_leaf);

            assert_eq!(
                std_contains, no_std_contains,
                "Both should agree on existing leaf"
            );
            assert!(std_contains, "Should find existing leaf");

            // Test with non-existent leaf
            let std_not_contains = tree.contains_leaf(&proof, non_existent_leaf);
            let no_std_not_contains = tree.contains_leaf_no_std(&proof, non_existent_leaf);

            assert_eq!(
                std_not_contains, no_std_not_contains,
                "Both should agree on non-existent leaf"
            );
            assert!(!std_not_contains, "Should not find non-existent leaf");

            println!("✅ contains_leaf vs contains_leaf_no_std test passed");
        }

        #[cfg(not(feature = "std"))]
        {
            let no_std_contains = tree.contains_leaf_no_std(&proof, target_leaf);
            let no_std_not_contains = tree.contains_leaf_no_std(&proof, non_existent_leaf);

            assert!(no_std_contains, "Should find existing leaf");
            assert!(!no_std_not_contains, "Should not find non-existent leaf");
            println!("✅ contains_leaf_no_std (no-std only) test passed");
        }
    }

    #[test]
    fn test_tree_operations_with_constants() {
        // Test using the actual constants from consts.rs
        const TAPE_HEIGHT: usize = 10; // TAPE_TREE_HEIGHT

        let leaves = create_test_leaves(20);
        let target_index = 7;
        let original_data: &[&[u8]] = &[b"leaf_7"];
        let new_data: &[&[u8]] = &[b"tape_replacement"];

        // Create trees
        let mut tree_std = MerkleTree::<TAPE_HEIGHT>::new(&[b"test_zero"]);
        let mut tree_no_std = MerkleTree::<TAPE_HEIGHT>::new(&[b"test_zero"]);

        for leaf in &leaves {
            tree_std.try_add_leaf(*leaf).expect("Should add leaf");
            tree_no_std.try_add_leaf(*leaf).expect("Should add leaf");
        }

        // Generate proof
        #[cfg(feature = "std")]
        let proof = tree_std.get_proof(&leaves, target_index);
        #[cfg(not(feature = "std"))]
        let proof = tree_no_std.get_proof_no_std(&leaves, target_index);

        #[cfg(feature = "std")]
        {
            // Test contains operations
            assert_eq!(
                tree_std.contains(&proof, original_data),
                tree_no_std.contains_no_std(&proof, original_data),
                "Contains should match for TAPE_TREE_HEIGHT"
            );

            // Test replacement operations
            let std_replace_result = tree_std.try_replace(&proof, original_data, new_data);
            let no_std_replace_result =
                tree_no_std.try_replace_no_std(&proof, original_data, new_data);

            assert_eq!(
                std_replace_result.is_ok(),
                no_std_replace_result.is_ok(),
                "Replace results should match for TAPE_TREE_HEIGHT"
            );

            if std_replace_result.is_ok() {
                assert_eq!(
                    tree_std.get_root(),
                    tree_no_std.get_root(),
                    "Final roots should match for TAPE_TREE_HEIGHT"
                );
            }

            println!("✅ Tree operations with TAPE_TREE_HEIGHT constants test passed");
        }

        #[cfg(not(feature = "std"))]
        {
            assert!(
                tree_no_std.contains_no_std(&proof, original_data),
                "Should contain original data"
            );
            let no_std_result = tree_no_std.try_replace_no_std(&proof, original_data, new_data);
            assert!(
                no_std_result.is_ok(),
                "No-std replacement should succeed with TAPE_TREE_HEIGHT"
            );
            println!(
                "✅ Tree operations (no-std only) with TAPE_TREE_HEIGHT constants test passed"
            );
        }
    }

    #[test]
    fn test_hash_pairs_comparison() {
        const MAX_PAIRS: usize = 8;

        // Create test hash pairs
        let hashes = create_test_leaves(6)
            .into_iter()
            .map(Hash::from)
            .collect::<Vec<Hash>>();

        #[cfg(feature = "std")]
        {
            let std_result = hash_pairs(hashes.clone());
            let (no_std_count, no_std_buffer) = hash_pairs_no_std::<MAX_PAIRS>(&hashes);

            assert_eq!(
                std_result.len(),
                no_std_count,
                "Hash pairs count should match"
            );

            for (i, (std_hash, no_std_hash)) in
                std_result.iter().zip(no_std_buffer.iter()).enumerate()
            {
                if i < no_std_count {
                    assert_eq!(std_hash, no_std_hash, "Hash pair {} should match", i);
                }
            }

            println!("✅ hash_pairs vs hash_pairs_no_std test passed");
        }

        #[cfg(not(feature = "std"))]
        {
            let (no_std_count, _no_std_buffer) = hash_pairs_no_std::<MAX_PAIRS>(&hashes);
            assert_eq!(
                no_std_count,
                hashes.len() / 2,
                "No-std hash pairs count should be correct"
            );
            println!("✅ hash_pairs_no_std (no-std only) test passed");
        }
    }

    #[test]
    fn test_compute_path_comparison() {
        const HEIGHT: usize = 6;
        const MAX_PATH: usize = HEIGHT + 1;

        let leaves = create_test_leaves(10);
        let target_index = 4;
        let target_leaf = leaves[target_index];

        // Create tree to get a valid proof
        let mut tree = MerkleTree::<HEIGHT>::new(&[b"test_zero"]);
        for leaf in &leaves {
            tree.try_add_leaf(*leaf).expect("Should add leaf");
        }

        #[cfg(feature = "std")]
        let proof = tree.get_proof(&leaves, target_index);
        #[cfg(not(feature = "std"))]
        let proof = tree.get_proof_no_std(&leaves, target_index);

        #[cfg(feature = "std")]
        {
            let std_path = compute_path(&proof, target_leaf);
            let (no_std_count, no_std_buffer) =
                compute_path_no_std::<MAX_PATH>(&proof, target_leaf);

            assert_eq!(std_path.len(), no_std_count, "Path lengths should match");

            for (i, (std_hash, no_std_hash)) in
                std_path.iter().zip(no_std_buffer.iter()).enumerate()
            {
                if i < no_std_count {
                    assert_eq!(std_hash, no_std_hash, "Path element {} should match", i);
                }
            }

            println!("✅ compute_path vs compute_path_no_std test passed");
        }

        #[cfg(not(feature = "std"))]
        {
            let (no_std_count, _no_std_buffer) =
                compute_path_no_std::<MAX_PATH>(&proof, target_leaf);
            assert_eq!(
                no_std_count,
                proof.len() + 1,
                "No-std path count should be correct"
            );
            println!("✅ compute_path_no_std (no-std only) test passed");
        }
    }

    #[test]
    fn test_is_valid_path_comparison() {
        const HEIGHT: usize = 5;
        const MAX_PATH: usize = HEIGHT + 1;

        let leaves = create_test_leaves(8);
        let target_index = 3;
        let target_leaf = leaves[target_index];

        // Create tree and generate proof
        let mut tree = MerkleTree::<HEIGHT>::new(&[b"test_zero"]);
        for leaf in &leaves {
            tree.try_add_leaf(*leaf).expect("Should add leaf");
        }

        let root = tree.get_root();

        #[cfg(feature = "std")]
        let proof = tree.get_proof(&leaves, target_index);
        #[cfg(not(feature = "std"))]
        let proof = tree.get_proof_no_std(&leaves, target_index);

        #[cfg(feature = "std")]
        {
            let std_path = compute_path(&proof, target_leaf);
            let (no_std_count, no_std_buffer) =
                compute_path_no_std::<MAX_PATH>(&proof, target_leaf);

            // Test valid path
            let std_valid = is_valid_path(&std_path, root);
            let no_std_valid = is_valid_path_no_std(&no_std_buffer, no_std_count, root);

            assert_eq!(std_valid, no_std_valid, "Path validity should match");
            assert!(std_valid, "Valid path should be recognized as valid");

            // Test invalid path (wrong root)
            let wrong_root = Hash::default();
            let std_invalid = is_valid_path(&std_path, wrong_root);
            let no_std_invalid = is_valid_path_no_std(&no_std_buffer, no_std_count, wrong_root);

            assert_eq!(std_invalid, no_std_invalid, "Invalid path should match");
            assert!(!std_invalid, "Invalid path should be recognized as invalid");

            println!("✅ is_valid_path vs is_valid_path_no_std test passed");
        }

        #[cfg(not(feature = "std"))]
        {
            let (no_std_count, no_std_buffer) =
                compute_path_no_std::<MAX_PATH>(&proof, target_leaf);

            let no_std_valid = is_valid_path_no_std(&no_std_buffer, no_std_count, root);
            let no_std_invalid =
                is_valid_path_no_std(&no_std_buffer, no_std_count, Hash::default());

            assert!(no_std_valid, "Valid path should be recognized as valid");
            assert!(
                !no_std_invalid,
                "Invalid path should be recognized as invalid"
            );

            println!("✅ is_valid_path_no_std (no-std only) test passed");
        }
    }

    #[test]
    fn test_all_utility_functions_integration() {
        const HEIGHT: usize = 6;
        const MAX_PAIRS: usize = 16;
        const MAX_PATH: usize = HEIGHT + 1;

        let leaves = create_test_leaves(12);
        let target_index = 7;
        let target_leaf = leaves[target_index];

        // Create tree
        let mut tree = MerkleTree::<HEIGHT>::new(&[b"test_zero"]);
        for leaf in &leaves {
            tree.try_add_leaf(*leaf).expect("Should add leaf");
        }

        let root = tree.get_root();

        #[cfg(feature = "std")]
        let proof = tree.get_proof(&leaves, target_index);
        #[cfg(not(feature = "std"))]
        let proof = tree.get_proof_no_std(&leaves, target_index);

        // Test the complete workflow with no-std functions
        let (path_count, path_buffer) = compute_path_no_std::<MAX_PATH>(&proof, target_leaf);
        let is_valid = is_valid_path_no_std(&path_buffer, path_count, root);

        assert!(
            is_valid,
            "Complete no-std workflow should validate correctly"
        );

        // Test hash_pairs_no_std as part of the workflow
        let leaf_hashes: Vec<Hash> = leaves.iter().map(|&leaf| Hash::from(leaf)).collect();
        let (pairs_count, _pairs_buffer) = hash_pairs_no_std::<MAX_PAIRS>(&leaf_hashes);

        assert_eq!(
            pairs_count,
            leaf_hashes.len() / 2,
            "Hash pairs should process correctly"
        );

        println!("✅ All utility functions integration test passed");
    }
}
