#![no_std]

pub mod account;
pub mod consts;
pub mod error;
pub mod event;
pub mod loaders;
pub mod pda;
pub mod rent;
pub mod state;
pub mod types;
pub mod utils;

pub use crate::consts::*;

pub mod prelude {
    pub use crate::consts::*;
    pub use crate::error::*;
    pub use crate::event::*;
    pub use crate::loaders::*;
    pub use crate::pda::*;
    pub use crate::rent::*;
    pub use crate::state::*;
    pub use crate::types::*;
    pub use crate::utils::*;
}

pinocchio_pubkey::declare_id!("tape9hFAE7jstfKB2QT1ovFNUZKKtDUyGZiGQpnBFdL");
