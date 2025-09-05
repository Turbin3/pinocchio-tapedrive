use crate::types::{AccountValidation, Discriminator};
use pinocchio::program_error::ProgramError;

#[macro_export]
macro_rules! impl_to_bytes {
    ($struct_name:ident) => {
        impl $struct_name {
            pub fn to_bytes(&self) -> &[u8] {
                bytemuck::bytes_of(self)
            }
        }
    };
}

#[macro_export]
macro_rules! account {
    ($discriminator_name:ident, $struct_name:ident) => {
        $crate::impl_to_bytes!($struct_name);

        impl $crate::types::Discriminator for $struct_name {
            fn discriminator() -> u8 {
                $discriminator_name::$struct_name.into()
            }
        }

        impl $crate::types::AccountValidation for $struct_name {
            #[track_caller]
            fn assert<F>(
                &self,
                condition: F,
            ) -> Result<&Self, pinocchio::program_error::ProgramError>
            where
                F: Fn(&Self) -> bool,
            {
                if !condition(self) {
                    return Err(pinocchio::program_error::ProgramError::InvalidAccountData);
                }
                Ok(self)
            }

            #[track_caller]
            fn assert_err<F>(
                &self,
                condition: F,
                err: pinocchio::program_error::ProgramError,
            ) -> Result<&Self, pinocchio::program_error::ProgramError>
            where
                F: Fn(&Self) -> bool,
            {
                if !condition(self) {
                    return Err(err);
                }
                Ok(self)
            }

            #[track_caller]
            fn assert_msg<F>(
                &self,
                condition: F,
                msg: &str,
            ) -> Result<&Self, pinocchio::program_error::ProgramError>
            where
                F: Fn(&Self) -> bool,
            {
                if !condition(self) {
                    pinocchio::log::sol_log(msg);
                    return Err(pinocchio::program_error::ProgramError::InvalidAccountData);
                }
                Ok(self)
            }

            #[track_caller]
            fn assert_mut<F>(
                &mut self,
                condition: F,
            ) -> Result<&mut Self, pinocchio::program_error::ProgramError>
            where
                F: Fn(&Self) -> bool,
            {
                if !condition(self) {
                    return Err(pinocchio::program_error::ProgramError::InvalidAccountData);
                }
                Ok(self)
            }

            #[track_caller]
            fn assert_mut_err<F>(
                &mut self,
                condition: F,
                err: pinocchio::program_error::ProgramError,
            ) -> Result<&mut Self, pinocchio::program_error::ProgramError>
            where
                F: Fn(&Self) -> bool,
            {
                if !condition(self) {
                    return Err(err);
                }
                Ok(self)
            }

            #[track_caller]
            fn assert_mut_msg<F>(
                &mut self,
                condition: F,
                msg: &str,
            ) -> Result<&mut Self, pinocchio::program_error::ProgramError>
            where
                F: Fn(&Self) -> bool,
            {
                if !condition(self) {
                    pinocchio::log::sol_log(msg);
                    return Err(pinocchio::program_error::ProgramError::InvalidAccountData);
                }
                Ok(self)
            }
        }
    };
}
