#![no_std]

#[cfg(not(feature = "no-entrypoint"))]
mod entrypoint;

#[cfg(feature = "std")]
extern crate std;

pub mod error;
pub mod instruction;
pub mod metadata;
pub mod state;
pub mod utils;

// Import the API crate
pub use tape_api as api;

pinocchio_pubkey::declare_id!("7wApqqrfJo2dAGAKVgheccaVEgeDoqVKogtJSTbFRWn2");
