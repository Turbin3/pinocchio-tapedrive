use {
    crate::{instruction::SetHeader, utils::ByteConversion},
    pinocchio::{account_info::AccountInfo, program_error::ProgramError, ProgramResult},
    tape_api::{
        error::TapeError,
        pda::tape_pda,
        state::{Tape, TapeState},
        utils::check_condition,
    },
};

pub fn process_tape_set_header(accounts: &[AccountInfo], data: &[u8]) -> ProgramResult {
    let args = SetHeader::try_from_bytes(data)?;
    let [signer_info, tape_info] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    let mut tape_info_raw_data = tape_info.try_borrow_mut_data()?;
    let tape = Tape::unpack_mut(&mut tape_info_raw_data)?;

    if !signer_info.is_signer() {
        return Err(ProgramError::MissingRequiredSignature);
    };

    if signer_info.key().ne(&tape.authority) {
        return Err(ProgramError::MissingRequiredSignature);
    };

    let (tape_address, _) = tape_pda(*signer_info.key(), &tape.name);

    if tape_info.key().ne(&tape_address) {
        return Err(ProgramError::InvalidAccountData);
    };

    check_condition(
        tape.state.eq(&(u64::from(TapeState::Writing as u8))),
        TapeError::UnexpectedState,
    )?;

    tape.header = args.header;

    Ok(())
}
