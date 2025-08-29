#[macro_export]
macro_rules! state {
    // $acct_ty is your AccountType enum variant, $data_ty is the struct name
    ($acct_ty:ident, $data_ty:ident) => {
        impl $data_ty {
            /// 8 bytes for the discriminator + the POD struct size
            pub const fn get_size() -> usize {
                8 + core::mem::size_of::<Self>()
            }

            /// Immutably unpack from a raw account data slice
            pub fn unpack(data: &[u8]) -> Result<&Self, pinocchio::program_error::ProgramError> {
                let data = &data[..Self::get_size()];
                // This now correctly returns a ProgramError
                Self::try_from_bytes(data)
            }

            /// Mutably unpack from a raw account data slice
            pub fn unpack_mut(data: &mut [u8]) -> Result<&mut Self, pinocchio::program_error::ProgramError> {
                let data = &mut data[..Self::get_size()];
                // This now correctly returns a ProgramError
                Self::try_from_bytes_mut(data)
            }
        }
        
        account!($acct_ty, $data_ty);
    };
}

#[macro_export]
macro_rules! impl_to_bytes {
    ($struct_name:ident, $discriminator_name:ident) => {
        impl $struct_name {
            pub fn to_bytes(&self) -> [u8; 8] {
                let mut discriminator = [0u8; 8];
                discriminator[0] = $discriminator_name::$struct_name as u8;
                let mut bytes = [0u8; 8];
                bytes[0] = discriminator[0];
                bytes[1..8].copy_from_slice(&[0, 0, 0, 0, 0, 0, 0]);
                bytes
            }
        }
    };
}

#[macro_export]
macro_rules! impl_try_from_bytes {
    ($struct_name:ident, $discriminator_name:ident) => {
        impl $struct_name {
            pub fn try_from_bytes(data: &[u8]) -> Result<&Self, pinocchio::program_error::ProgramError> {
                if data.len() < 8 {
                    return Err(pinocchio::program_error::ProgramError::InvalidAccountData);
                }
                
                let discriminator = data[0];
                if discriminator != $discriminator_name::$struct_name as u8 {
                    pinocchio::log::sol_log_data(&[
                        b"Error: Invalid discriminator",
                        &($discriminator_name::$struct_name as u8).to_le_bytes(),
                        &discriminator.to_le_bytes(),
                    ]);
                    return Err(pinocchio::program_error::ProgramError::InvalidAccountData);
                }

                let struct_size = core::mem::size_of::<Self>();
                if data.len() < 8 + struct_size {
                    return Err(pinocchio::program_error::ProgramError::InvalidAccountData);
                }

                bytemuck::try_from_bytes::<Self>(&data[8..8 + struct_size])
                    .map_err(|_| pinocchio::program_error::ProgramError::InvalidAccountData)
            }
        }
    };
}


#[macro_export]
macro_rules! event {
    ($discriminator_name:ident, $struct_name:ident) => {
        $crate::impl_to_bytes!($struct_name, $discriminator_name);
        $crate::impl_try_from_bytes!($struct_name, $discriminator_name);

        impl $struct_name {
            const DISCRIMINATOR_SIZE: usize = 8;

            pub fn size_of() -> usize {
                core::mem::size_of::<Self>() + Self::DISCRIMINATOR_SIZE
            }

            pub fn log(&self) {
                pinocchio::log::sol_log_data(&[&self.to_bytes()]);
            }
        }
    };
}