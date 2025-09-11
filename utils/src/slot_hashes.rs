pub type SlotHash = (u64, Hash);

pub struct Hash(pub(crate) [u8; HASH_BYTES]);

/// Size of a hash in bytes.
pub const HASH_BYTES: usize = 32;
