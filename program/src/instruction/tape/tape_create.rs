use {
    crate::{instruction::Create, utils::ByteConversion},
    bytemuck::Zeroable,
    pinocchio::{
        account_info::AccountInfo,
        instruction::{Seed, Signer},
        program_error::ProgramError,
        sysvars::{clock::Clock, rent::Rent, Sysvar},
        ProgramResult,
    },
    pinocchio_system::instructions::CreateAccount,
    tape_api::{
        consts::{HEADER_SIZE, TAPE, WRITER},
        pda::{tape_pda, writer_pda},
        state::{DataLen, Tape, TapeState, Writer},
    },
};

pub fn process_tape_create(accounts: &[AccountInfo], data: &[u8]) -> ProgramResult {
    let current_slot = Clock::get()?.slot;

    let args = Create::try_from_bytes(data)?;

    // dev : ignore system_program_info and rent_sysvar_info
    let [signer_info, tape_info, writer_info, _remaining @ ..] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    if !signer_info.is_signer() {
        return Err(ProgramError::MissingRequiredSignature);
    };

    let (tape_address, _tape_bump) = tape_pda(*signer_info.key(), &args.name);
    let (writer_address, _writer_bump) = writer_pda(tape_address);

    if !tape_info.data_is_empty() {
        return Err(ProgramError::AccountAlreadyInitialized);
    };

    if !tape_info.is_writable() {
        return Err(ProgramError::MissingRequiredSignature);
    };

    if tape_info.key().ne(&tape_address) {
        return Err(ProgramError::InvalidAccountData);
    };

    if !writer_info.data_is_empty() {
        return Err(ProgramError::AccountAlreadyInitialized);
    };

    if !writer_info.is_writable() {
        return Err(ProgramError::MissingRequiredSignature);
    };

    if writer_info.key().ne(&writer_address) {
        return Err(ProgramError::InvalidAccountData);
    };

    //   dev : ignore unnecessary checks
    //   - system program
    //   - rent sysvar

    // create tape_info PDA
    let tape_info_space = Tape::LEN;
    let tape_info_rent = Rent::get()?.minimum_balance(tape_info_space);
    let tape_bump_binding = [_tape_bump];

    let tape_info_seeds = &[
        Seed::from(TAPE),
        Seed::from(signer_info.key().as_ref()),
        Seed::from(&args.name),
        Seed::from(&tape_bump_binding),
    ];

    let tape_info_signature = Signer::from(tape_info_seeds);

    CreateAccount {
        from: signer_info,
        to: tape_info,
        lamports: tape_info_rent,
        space: tape_info_space as u64,
        owner: &tape_api::ID,
    }
    .invoke_signed(&[tape_info_signature])?;

    // create writer_info pda
    let writer_info_space = Writer::LEN;
    let writer_info_rent = Rent::get()?.minimum_balance(writer_info_space);
    let writer_bump_binding = [_writer_bump];

    let writer_info_seeds = &[
        Seed::from(WRITER),
        Seed::from(tape_info.key().as_ref()),
        Seed::from(&writer_bump_binding),
    ];

    let writer_info_signature = Signer::from(writer_info_seeds);

    CreateAccount {
        from: signer_info,
        to: writer_info,
        lamports: writer_info_rent,
        space: writer_info_space as u64,
        owner: &tape_api::ID,
    }
    .invoke_signed(&[writer_info_signature])?;

    // initialize tape_info data
    let mut tape_info_raw_data = tape_info.try_borrow_mut_data()?;
    let tape = Tape::unpack_mut(&mut tape_info_raw_data)?;

    *tape = Tape {
        number: 0, // (tapes get a number when finalized)
        authority: *signer_info.key(),
        name: args.name,
        state: TapeState::Created as u64,
        total_segments: 0,
        merkle_root: [0; 32],
        header: [0; HEADER_SIZE],
        first_slot: current_slot,
        tail_slot: current_slot,
        ..Tape::zeroed()
    };

    // initialize writer_info data
    let mut writer_info_raw_data = writer_info.try_borrow_mut_data()?;
    let writer = Writer::unpack_mut(&mut writer_info_raw_data)?;

    writer.tape = *tape_info.key();
    // writer.state = *;  # dev : not implemented in Writer layout !

    Ok(())
}
