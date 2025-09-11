use bytemuck::{Pod, Zeroable};
use num_enum::TryFromPrimitive;

#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq, TryFromPrimitive)]
pub enum EventType {
    Unknown = 0,

    WriteEvent,
    UpdateEvent,
    FinalizeEvent,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
pub struct WriteEvent {
    pub num_added: u64,
    pub num_total: u64,
    pub prev_slot: u64,
    pub address: [u8; 32],
}

impl WriteEvent {
    const DISCRIMINATOR_SIZE: usize = 8;

    pub fn size_of() -> usize {
        core::mem::size_of::<Self>() + Self::DISCRIMINATOR_SIZE
    }

    pub fn to_bytes(&self) -> [u8; 56] {
        let mut result = [0u8; 56]; // 8 bytes discriminator + 48 bytes struct

        // Add 8-byte discriminator (first byte is the enum variant, rest are zeros)
        result[0] = EventType::WriteEvent as u8;
        // bytes 1-7 remain as zeros

        // Add struct bytes starting at index 8
        let struct_bytes = bytemuck::bytes_of(self);
        result[8..8 + struct_bytes.len()].copy_from_slice(struct_bytes);

        result
    }

    pub fn try_from_bytes(data: &[u8]) -> Result<&Self, &'static str> {
        if data.len() < 8 {
            return Err("Data too short for discriminator");
        }

        let discriminator = data[0];
        if discriminator != EventType::WriteEvent as u8 {
            return Err("Invalid discriminator");
        }

        let struct_size = core::mem::size_of::<Self>();
        if data.len() < 8 + struct_size {
            return Err("Data too short for struct");
        }

        bytemuck::try_from_bytes::<Self>(&data[8..8 + struct_size])
            .map_err(|_| "Invalid struct data")
    }

    pub fn log(&self) {
        let bytes = self.to_bytes();
        // pinocchio::msg!(bytes.to_string());
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
pub struct UpdateEvent {
    pub segment_number: u64,
    pub prev_slot: u64,
    pub address: [u8; 32],
}

impl UpdateEvent {
    const DISCRIMINATOR_SIZE: usize = 8;

    pub fn size_of() -> usize {
        core::mem::size_of::<Self>() + Self::DISCRIMINATOR_SIZE
    }

    pub fn to_bytes(&self) -> [u8; 56] {
        let mut result = [0u8; 56]; // 8 bytes discriminator + 48 bytes struct

        // Add 8-byte discriminator (first byte is the enum variant, rest are zeros)
        result[0] = EventType::UpdateEvent as u8;
        // bytes 1-7 remain as zeros

        // Add struct bytes starting at index 8
        let struct_bytes = bytemuck::bytes_of(self);
        result[8..8 + struct_bytes.len()].copy_from_slice(struct_bytes);

        result
    }

    pub fn try_from_bytes(data: &[u8]) -> Result<&Self, &'static str> {
        if data.len() < 8 {
            return Err("Data too short for discriminator");
        }

        let discriminator = data[0];
        if discriminator != EventType::UpdateEvent as u8 {
            return Err("Invalid discriminator");
        }

        let struct_size = core::mem::size_of::<Self>();
        if data.len() < 8 + struct_size {
            return Err("Data too short for struct");
        }

        bytemuck::try_from_bytes::<Self>(&data[8..8 + struct_size])
            .map_err(|_| "Invalid struct data")
    }

    pub fn log(&self) {
        let bytes = self.to_bytes();
        //TODO: add logging here
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
pub struct FinalizeEvent {
    pub tape: u64,
    pub address: [u8; 32],
}

impl FinalizeEvent {
    const DISCRIMINATOR_SIZE: usize = 8;

    pub fn size_of() -> usize {
        core::mem::size_of::<Self>() + Self::DISCRIMINATOR_SIZE
    }

    pub fn to_bytes(&self) -> [u8; 48] {
        let mut result = [0u8; 48]; // 8 bytes discriminator + 40 bytes struct

        // Add 8-byte discriminator (first byte is the enum variant, rest are zeros)
        result[0] = EventType::FinalizeEvent as u8;
        // bytes 1-7 remain as zeros

        // Add struct bytes starting at index 8
        let struct_bytes = bytemuck::bytes_of(self);
        result[8..8 + struct_bytes.len()].copy_from_slice(struct_bytes);

        result
    }

    pub fn try_from_bytes(data: &[u8]) -> Result<&Self, &'static str> {
        if data.len() < 8 {
            return Err("Data too short for discriminator");
        }

        let discriminator = data[0];
        if discriminator != EventType::FinalizeEvent as u8 {
            return Err("Invalid discriminator");
        }

        let struct_size = core::mem::size_of::<Self>();
        if data.len() < 8 + struct_size {
            return Err("Data too short for struct");
        }

        bytemuck::try_from_bytes::<Self>(&data[8..8 + struct_size])
            .map_err(|_| "Invalid struct data")
    }

    pub fn log(&self) {
        let bytes = self.to_bytes();
        //TODO: add logging here
    }
}
