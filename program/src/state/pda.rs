use crate::state::constant::{TAPE_ID, TREASURY, MINT, MINT_SEED};
use pinocchio::pubkey::{self, Pubkey};

pub fn get_mint_pda() -> (Pubkey, u8) {
    pubkey::find_program_address(&[MINT, MINT_SEED], &TAPE_ID)
}

pub fn get_treasury_pda() -> (Pubkey, u8) {
    pubkey::find_program_address(&[TREASURY], &TAPE_ID)
}
