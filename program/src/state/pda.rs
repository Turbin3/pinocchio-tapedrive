use crate::state::constant::{ MINT_BUMP, TREASURY_BUMP, MINT_ADDRESS, TREASURY_ADDRESS};
use pinocchio::pubkey::Pubkey;

#[inline(always)]
pub const fn treasury_pda() -> (Pubkey, u8) {
    (TREASURY_ADDRESS, TREASURY_BUMP)
}

#[inline(always)]
pub const fn mint_pda() -> (Pubkey, u8) {
    (MINT_ADDRESS, MINT_BUMP)
}
