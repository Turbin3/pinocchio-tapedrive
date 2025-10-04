use {
    crate::{instruction::Update, utils::ByteConversion},
    pinocchio::{
        account_info::AccountInfo,
        program_error::ProgramError,
        sysvars::{clock::Clock, Sysvar},
        ProgramResult,
    },
    tape_api::{
        consts::{SEGMENT_PROOF_LEN, SEGMENT_SIZE},
        error::TapeError,
        event::UpdateEvent,
        pda::{tape_pda, writer_pda},
        state::{Tape, TapeState, Writer},
        utils::check_condition,
    },
    tape_utils::leaf::Leaf,
};

pub fn process_tape_update(accounts: &[AccountInfo], data: &[u8]) -> ProgramResult {
    let current_slot = Clock::get()?.slot;
    let args = Update::try_from_bytes(data)?;

    let [signer_info, tape_info, writer_info] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    let mut tape_info_raw_data = tape_info.try_borrow_mut_data()?;
    let tape = Tape::unpack_mut(&mut tape_info_raw_data)?;

    let mut writer_info_raw_data = writer_info.try_borrow_mut_data()?;
    let writer = Writer::unpack_mut(&mut writer_info_raw_data)?;

    if !signer_info.is_signer() {
        return Err(ProgramError::MissingRequiredSignature);
    };

    if signer_info.key().ne(&tape.authority) {
        return Err(ProgramError::MissingRequiredSignature);
    };

    if tape_info.key().ne(&writer.tape) {
        return Err(ProgramError::InvalidAccountData);
    }

    let (tape_address, _) = tape_pda(*signer_info.key(), &tape.name);
    let (writer_address, _) = writer_pda(tape_address);

    if tape_info.key().ne(&tape_address) {
        return Err(ProgramError::InvalidAccountData);
    };

    if writer_info.key().ne(&writer_address) {
        return Err(ProgramError::InvalidAccountData);
    };

    check_condition(
        tape.state == TapeState::Created as u64 || tape.state == TapeState::Writing as u64,
        TapeError::UnexpectedState,
    )?;

    let segment_number = args.segment_number;
    let merkle_proof = args.proof.as_ref();

    check_condition(
        args.old_data.len() == SEGMENT_SIZE,
        ProgramError::InvalidInstructionData,
    )?;
    check_condition(
        args.new_data.len() == SEGMENT_SIZE,
        ProgramError::InvalidInstructionData,
    )?;
    check_condition(
        merkle_proof.len() == SEGMENT_PROOF_LEN,
        ProgramError::InvalidInstructionData,
    )?;

    let old_leaf = Leaf::new(&[
        segment_number.as_ref(), // u64_le_bytes
        args.old_data.as_ref(),
    ]);

    let new_leaf = Leaf::new(&[
        segment_number.as_ref(), // u64_le_bytes
        args.new_data.as_ref(),
    ]);

    writer
        .state
        .try_replace_leaf_no_std(merkle_proof, old_leaf, new_leaf)
        .map_err(|_| TapeError::WriteFailed)?;

    let prev_slot = tape.tail_slot;

    tape.merkle_root = writer.state.get_root().to_bytes();
    tape.tail_slot = current_slot;

    UpdateEvent {
        prev_slot,
        segment_number: u64::from_le_bytes(segment_number),
        address: tape_address,
    }
    .log();

    Ok(())
}
