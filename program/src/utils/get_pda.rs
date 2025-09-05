use crate::state::MPL_TOKEN_METADATA_ID;
use crate::state::TAPE_ID;
use pinocchio::pubkey::{find_program_address, Pubkey};

pub enum GetPda {
    Metadata(Pubkey),
    Mint,
    Treasury,
}

impl GetPda {
    pub fn address(&self) -> (Pubkey, u8) {
        match self {
            GetPda::Mint => {
                find_program_address(&[b"mint", &[152, 68, 212, 200, 25, 113, 221, 71]], &TAPE_ID)
            }
            GetPda::Treasury => find_program_address(&[b"treasury"], &TAPE_ID),
            GetPda::Metadata(mint) => find_program_address(
                &[b"metadata", MPL_TOKEN_METADATA_ID.as_ref(), mint.as_ref()],
                &MPL_TOKEN_METADATA_ID,
            ),
        }
    }
}
