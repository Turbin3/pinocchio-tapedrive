use crate::state::AccountType;
use bytemuck::{Pod, Zeroable};
use pinocchio::program_error::ProgramError;

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
pub struct Epoch {
    pub number: u64,
    pub progress: u64,

    pub mining_difficulty: u64,
    pub packing_difficulty: u64,
    pub target_participation: u64,
    pub reward_rate: u64,
    pub duplicates: u64,

    pub last_epoch_at: i64,
}

impl AccountMutation for Epoch {
    /// 8 bytes for the discriminator + the POD struct size
    fn get_size() -> usize {
        8 + core::mem::size_of::<Self>()
    }

    /// Immutably unpack from a raw account data slice
    fn unpack(data: &[u8]) -> Result<&Self, ProgramError> {
        let data = &data[..Self::get_size()];
        Self::try_from_bytes(data)
    }

    /// Mutably unpack from a raw account data slice
    fn unpack_mut(data: &mut [u8]) -> Result<&mut Self, ProgramError> {
        let data = &mut data[..Self::get_size()];
        Self::try_from_bytes_mut(data)
    }
}

pub trait AccountMutation {
    fn get_size() -> usize;

    fn unpack(data: &[u8]) -> Result<&Self, ProgramError>;

    fn unpack_mut(data: &mut [u8]) -> Result<&mut Self, ProgramError>;
}

pub trait AccountValidation {
    fn to_bytes(&self) -> &[u8];

    fn discriminator() -> u8;

    fn assert<F: Fn(&Self) -> bool>(&self, condition: F) -> Result<&Self, ProgramError>;

    fn assert_err<F: Fn(&Self) -> bool>(
        &self,
        condition: F,
        err: ProgramError,
    ) -> Result<&Self, ProgramError>;

    fn assert_msg<F: Fn(&Self) -> bool>(
        &self,
        condition: F,
        msg: &str,
    ) -> Result<&Self, ProgramError>;

    fn assert_mut<F: Fn(&Self) -> bool>(&mut self, condition: F)
        -> Result<&mut Self, ProgramError>;

    fn assert_mut_err<F: Fn(&Self) -> bool>(
        &mut self,
        condition: F,
        err: ProgramError,
    ) -> Result<&mut Self, ProgramError>;

    fn assert_mut_msg<F: Fn(&Self) -> bool>(
        &mut self,
        condition: F,
        msg: &str,
    ) -> Result<&mut Self, ProgramError>;
}

impl AccountValidation for Epoch {
    fn to_bytes(&self) -> &[u8] {
        bytemuck::bytes_of(self)
    }

    fn discriminator() -> u8 {
        AccountType::Epoch.into()
    }

    fn assert<F: Fn(&Self) -> bool>(&self, condition: F) -> Result<&Self, ProgramError> {
        if !condition(self) {
            return Err(ProgramError::InvalidAccountData);
        }
        Ok(self)
    }

    fn assert_err<F: Fn(&Self) -> bool>(
        &self,
        condition: F,
        err: ProgramError,
    ) -> Result<&Self, ProgramError> {
        if !condition(self) {
            return Err(err);
        }
        Ok(self)
    }

    //incomplete: send back the msg
    fn assert_msg<F: Fn(&Self) -> bool>(
        &self,
        condition: F,
        _msg: &str,
    ) -> Result<&Self, ProgramError> {
        if !condition(self) {
            return Err(ProgramError::InvalidAccountData);
        }
        Ok(self)
    }

    fn assert_mut<F: Fn(&Self) -> bool>(
        &mut self,
        condition: F,
    ) -> Result<&mut Self, ProgramError> {
        if !condition(self) {
            return Err(ProgramError::InvalidAccountData);
        }
        Ok(self)
    }

    fn assert_mut_err<F: Fn(&Self) -> bool>(
        &mut self,
        condition: F,
        err: ProgramError,
    ) -> Result<&mut Self, ProgramError> {
        if !condition(self) {
            return Err(err);
        }
        Ok(self)
    }

    //incomplete: send back the msg
    fn assert_mut_msg<F: Fn(&Self) -> bool>(
        &mut self,
        condition: F,
        _msg: &str,
    ) -> Result<&mut Self, ProgramError> {
        if !condition(self) {
            return Err(ProgramError::InvalidAccountData);
        }
        Ok(self)
    }
}

impl Epoch {
    pub fn to_bytes(&self) -> &[u8] {
        bytemuck::bytes_of(self)
    }

    pub fn from_bytes(data: &[u8]) -> &Self {
        bytemuck::from_bytes::<Self>(data)
    }

    pub fn try_from_bytes(data: &[u8]) -> Result<&Self, ProgramError> {
        bytemuck::try_from_bytes::<Self>(data).or(Err(ProgramError::InvalidInstructionData))
    }

    pub fn try_from_bytes_mut(data: &mut [u8]) -> Result<&mut Self, ProgramError> {
        bytemuck::try_from_bytes_mut::<Self>(data).or(Err(ProgramError::InvalidInstructionData))
    }
}
