use bytemuck::{Pod, Zeroable};
use pinocchio::program_error::ProgramError;

pub trait AccountDiscriminator {
    fn discriminator() -> u8;
}

pub trait AccountMutation: Pod + Zeroable + AccountDiscriminator {
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

pub trait AccountValidation: Pod + AccountDiscriminator {
    fn to_bytes(&self) -> &[u8] {
        bytemuck::bytes_of(self)
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

pub trait ByteConversion: Pod {
    fn from_bytes(data: &[u8]) -> &Self {
        bytemuck::from_bytes::<Self>(data)
    }

    fn try_from_bytes(data: &[u8]) -> Result<&Self, ProgramError> {
        bytemuck::try_from_bytes::<Self>(data).or(Err(ProgramError::InvalidInstructionData))
    }

    fn try_from_bytes_mut(data: &mut [u8]) -> Result<&mut Self, ProgramError> {
        bytemuck::try_from_bytes_mut::<Self>(data).or(Err(ProgramError::InvalidInstructionData))
    }
}

// Blanket implementations for any type that meets the requirements
impl<T> AccountMutation for T where T: Pod + Zeroable + AccountDiscriminator {}
impl<T> AccountValidation for T where T: Pod + AccountDiscriminator {}
impl<T> ByteConversion for T where T: Pod {}
