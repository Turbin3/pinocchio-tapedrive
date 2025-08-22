use pinocchio::program_error::ProgramError;

pub trait DataLen {
    const LEN: usize;
}

#[inline(always)]
pub unsafe fn load_ix_data<T: DataLen>(bytes: &[u8]) -> Result<&T, ProgramError> {
    if bytes.len() != T::LEN {
        return Err(ProgramError::InvalidInstructionData);
    }
    Ok(&*(bytes.as_ptr() as *const T))
}
