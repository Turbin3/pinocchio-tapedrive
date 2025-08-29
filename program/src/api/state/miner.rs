use steel::*;
use crate::api::consts::*;
use crate::state::utils::{DataLen, Initialized, load_acc, load_acc_mut, try_from_account_info_mut};
use pinocchio::{program_error::ProgramError, ProgramResult, account_info::AccountInfo};

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
pub struct Miner {
    pub authority: Pubkey,
    pub name: [u8; NAME_LEN],

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

impl Initialized for Miner {
    fn is_initialized(&self) -> bool {
        true
    }
}

impl Miner {
    pub fn unpack(data: &[u8]) -> Result<&Self, ProgramError> {
        unsafe { load_acc::<Miner>(data) }
    }
    pub fn unpack_mut(data: &mut [u8]) -> Result<&mut Self, ProgramError> {
        unsafe { load_acc_mut::<Miner>(data) }
    }

    pub fn initialize(
        miner_info: &AccountInfo,
        name: [u8; NAME_LEN],
        authority: Pubkey,
        challenge: [u8; 32],
    ) -> ProgramResult {
        let miner_state = unsafe { try_from_account_info_mut::<Miner>(miner_info) }?;

        miner_state.authority = authority;
        miner_state.name = name;
        miner_state.unclaimed_rewards = 0;
        miner_state.challenge = challenge;
        miner_state.commitment = [0; 32];
        miner_state.multiplier = 0;
        miner_state.last_proof_block = 0;
        miner_state.last_proof_at = 0;
        miner_state.total_proofs = 0;
        miner_state.total_rewards = 0;

        Ok(())
    }
}