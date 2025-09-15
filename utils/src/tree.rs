#![allow(unexpected_cfgs)]

use super::{
    error::{BrineTreeError, ProgramResult},
    leaf::{hashv, Hash, Leaf},
    utils::check_condition,
};
use bytemuck::{Pod, Zeroable};

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
