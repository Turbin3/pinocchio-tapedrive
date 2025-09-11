use bytemuck::{Pod, Zeroable};

pub const HASH_BYTES: usize = 32;

#[repr(C)]
#[derive(Clone, Copy, PartialEq, Debug, Default, Pod, Zeroable)]
pub struct Hash {
    pub(crate) value: [u8; 32],
}

#[repr(C)]
#[derive(Clone, Copy, PartialEq, Debug, Pod, Zeroable)]
pub struct Leaf(Hash);

impl From<Hash> for [u8; HASH_BYTES] {
    fn from(from: Hash) -> Self {
        from.value
    }
}

impl From<[u8; HASH_BYTES]> for Hash {
    fn from(from: [u8; 32]) -> Self {
        Self { value: from }
    }
}

impl From<[u8; HASH_BYTES]> for Leaf {
    fn from(from: [u8; 32]) -> Self {
        Self(Hash { value: from })
    }
}

impl AsRef<[u8]> for Hash {
    fn as_ref(&self) -> &[u8] {
        &self.value
    }
}

impl AsRef<[u8]> for Leaf {
    fn as_ref(&self) -> &[u8] {
        &self.0.value
    }
}

impl From<Leaf> for Hash {
    fn from(leaf: Leaf) -> Self {
        leaf.0
    }
}

impl Hash {
    pub const LEN: usize = HASH_BYTES;

    pub fn new(hash_slice: &[u8]) -> Self {
        Hash {
            value: <[u8; HASH_BYTES]>::try_from(hash_slice).unwrap(),
        }
    }

    pub const fn new_from_array(hash_array: [u8; HASH_BYTES]) -> Self {
        Self { value: hash_array }
    }

    pub fn to_bytes(self) -> [u8; HASH_BYTES] {
        self.value
    }

    pub fn as_leaf(self) -> Leaf {
        Leaf(self)
    }
}

impl Leaf {
    pub fn new(data: &[&[u8]]) -> Self {
        // let mut inputs = vec![b"LEAF".as_ref()];
        // inputs.extend(data);
        // Leaf(hashv(&inputs))
        let input = b"LEAF".as_ref();
        let mut hasher = blake3::Hasher::new();
        hasher.update(input);
        for d in data {
            hasher.update(d);
        }
        Leaf(Hash::new_from_array(hasher.finalize().into()))
    }

    pub fn to_bytes(self) -> [u8; HASH_BYTES] {
        self.0.value
    }
}

#[inline(always)]
pub fn hashv(data: &[&[u8]]) -> Hash {
    let mut hasher = blake3::Hasher::new();
    for d in data {
        hasher.update(d);
    }
    Hash::new_from_array(hasher.finalize().into())
}

#[inline(always)]
pub fn hash(data: &[u8]) -> Hash {
    let mut hasher = blake3::Hasher::new();
    hasher.update(data);
    Hash::new_from_array(hasher.finalize().into())
}
