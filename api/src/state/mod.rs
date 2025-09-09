mod archive;
mod block;
mod epoch;
mod miner;
mod spool;
mod tape;
mod treasury;
pub mod utils;
mod writer;

pub use archive::*;
pub use block::*;
pub use epoch::*;
pub use miner::*;
pub use spool::*;
pub use tape::*;
pub use treasury::*;
pub use utils::*;
pub use writer::*;

#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
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

impl Into<u8> for AccountType {
    fn into(self) -> u8 {
        self as u8
    }
}
