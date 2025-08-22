use pinocchio::{account_info::AccountInfo, pubkey::Pubkey};

use crate::instruction::DataLen;

#[repr(C)]
pub struct Miner {
    pub authority: Pubkey,
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
    const LEN: usize = core::mem::size_of::<Miner>();
}
impl Miner {
    #[inline]
    pub fn from_account_info_unchecked(account_info: &AccountInfo) -> &mut Self {
        unsafe {
            let data = account_info.borrow_mut_data_unchecked();
            &mut *(data.as_mut_ptr() as *mut Miner)
        }
    }
}
