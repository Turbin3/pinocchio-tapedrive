use num_enum::{IntoPrimitive, TryFromPrimitive};

pub mod constant;
pub mod pda;
pub mod utils;
pub mod mine;
pub mod types;

mod archive;
mod block;
mod epoch;
mod miner;
mod spool;
mod tape;
mod treasury;
mod writer;

pub use archive::*;
pub use block::*;
pub use constant::*;
pub use epoch::*;
pub use miner::*;
pub use spool::*;
pub use tape::*;
pub use treasury::*;
pub use utils::*;
pub use writer::*;
pub use types::*;
pub use mine::*;

#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq, IntoPrimitive, TryFromPrimitive)]
pub enum AccountType {
    Unknown = 0,
    Archive,
    Spool,
    Writer,
    Tape,
    Miner,
    Epoch,
    Block,
    Treasury,
}
