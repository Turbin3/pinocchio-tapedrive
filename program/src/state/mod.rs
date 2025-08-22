use num_enum::{IntoPrimitive, TryFromPrimitive};

pub mod constant;
pub mod epoch;
pub mod utils;

pub use constant::*;
pub use epoch::*;
pub use utils::*;

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
