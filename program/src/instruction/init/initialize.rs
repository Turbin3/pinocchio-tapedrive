use crate::instruction::mine::miner_mine::get_base_rate;
use crate::state::*;
use crate::utils::account_traits::AccountInfoExt;
use crate::utils::get_pda::GetPda;
use crate::utils::helpers::{cast_account_data_mut, create_program_account};
use pinocchio::{
    account_info::AccountInfo,
    instruction::{Seed, Signer},
    program_error::ProgramError,
    sysvars::{rent::Rent, Sysvar},
    ProgramResult,
};
use pinocchio_associated_token_account::instructions::Create as CreateATA;
use pinocchio_system::instructions::CreateAccount;
use pinocchio_token::instructions::{InitializeMint2, MintTo};
use tape_api::consts::{
    BLOCK_ADDRESS, MAX_SUPPLY, MINT_BUMP, MINT_SEED, MIN_MINING_DIFFICULTY, MIN_PACKING_DIFFICULTY,
    MIN_PARTICIPATION_TARGET, TOKEN_DECIMALS, TREASURY_BUMP,
};
// TODO: Uncomment when tape instruction builders are available
// use tape_api::pda::{tape_pda, writer_pda};
// use tape_api::rent::min_finalization_rent;
// use tape_api::utils::to_name;
use tape_api::utils::compute_next_challenge;

pub fn process_initialize(accounts: &[AccountInfo], _data: &[u8]) -> ProgramResult {
    // if !data.is_empty() {
    //     return Err(ProgramError::InvalidInstructionData);
    // }

    let [signer_info, archive_info, epoch_info, block_info, metadata_info, mint_info, treasury_info, treasury_ata_info, tape_info, writer_info, tape_program_info, system_program_info, token_program_info, associated_token_program_info, metadata_program_info, rent_sysvar_info, slot_hashes_info] =
        accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    archive_info.check_account(ARCHIVE)?;
    epoch_info.check_account(EPOCH)?;
    block_info.check_account(BLOCK)?;

    let (mint_address, mint_bump) = GetPda::Mint.address();
    let (treasury_address, treasury_bump) = GetPda::Treasury.address();
    let (metadata_address, _metadata_bump) = GetPda::Metadata(mint_address).address();

    assert_eq!(mint_bump, MINT_BUMP);
    assert_eq!(treasury_bump, TREASURY_BUMP);

    mint_info.check_account_with_address(&mint_address)?;
    metadata_info.check_account_with_address(&metadata_address)?;
    treasury_info.check_account_with_address(&treasury_address)?;

    if !treasury_ata_info.data_is_empty() {
        return Err(ProgramError::AccountAlreadyInitialized);
    }
    if !treasury_ata_info.is_writable() {
        return Err(ProgramError::Immutable);
    }

    tape_program_info.is_program_check()?;
    system_program_info.is_program_check()?;
    token_program_info.is_program_check()?;
    associated_token_program_info.is_program_check()?;
    metadata_program_info.is_program_check()?;
    rent_sysvar_info.is_program_check()?;
    slot_hashes_info.is_program_check()?;

    // Initialize epoch
    create_program_account::<Epoch>(
        epoch_info,
        system_program_info,
        signer_info,
        &TAPE_ID,
        &[EPOCH],
    )?;

    // Set epoch fields using bytemuck (safe, no unsafe!)
    {
        let mut epoch_data = epoch_info.try_borrow_mut_data()?;
        let epoch = cast_account_data_mut::<Epoch>(&mut epoch_data)?;
        epoch.number = 1;
        epoch.progress = 0;
        epoch.target_participation = MIN_PARTICIPATION_TARGET;
        epoch.mining_difficulty = MIN_MINING_DIFFICULTY;
        epoch.packing_difficulty = MIN_PACKING_DIFFICULTY;
        epoch.reward_rate = get_base_rate(1);
        epoch.duplicates = 0;
        epoch.last_epoch_at = 0;
    }

    // Initialize block
    create_program_account::<Block>(
        block_info,
        system_program_info,
        signer_info,
        &TAPE_ID,
        &[BLOCK],
    )?;

    // Set block fields
    {
        let mut block_data = block_info.try_borrow_mut_data()?;
        let block = cast_account_data_mut::<Block>(&mut block_data)?;
        block.number = 1;
        block.progress = 0;
        block.last_proof_at = 0;
        block.last_block_at = 0;

        // Compute next challenge using slot hashes
        let next_challenge = compute_next_challenge(&BLOCK_ADDRESS.into(), slot_hashes_info)?;
        block.challenge = next_challenge;
        block.challenge_set = 1;
    }

    // Initialize archive
    create_program_account::<Archive>(
        archive_info,
        system_program_info,
        signer_info,
        &TAPE_ID,
        &[ARCHIVE],
    )?;

    // Set archive fields
    {
        let mut archive_data = archive_info.try_borrow_mut_data()?;
        let archive = cast_account_data_mut::<Archive>(&mut archive_data)?;
        archive.tapes_stored = 0;
        archive.segments_stored = 0;
    }

    // Initialize treasury (empty struct, no fields to initialize)
    create_program_account::<Treasury>(
        treasury_info,
        system_program_info,
        signer_info,
        &TAPE_ID,
        &[TREASURY],
    )?;

    // Initialize mint (allocate + initialize)
    {
        let rent = Rent::get()?;
        let mint_space = pinocchio_token::state::Mint::LEN;
        let lamports = rent.minimum_balance(mint_space);

        // Allocate mint account with PDA
        let mint_seed_binding = MINT_SEED;
        let mint_bump_binding = [MINT_BUMP];
        let mint_seeds = [
            Seed::from(MINT),
            Seed::from(mint_seed_binding),
            Seed::from(mint_bump_binding.as_slice()),
        ];
        let mint_signer = [Signer::from(&mint_seeds)];

        CreateAccount {
            from: signer_info,
            to: mint_info,
            lamports,
            space: mint_space as u64,
            owner: &pinocchio_token::ID,
        }
        .invoke_signed(&mint_signer)?;

        // Initialize the mint
        InitializeMint2 {
            mint: mint_info,
            decimals: TOKEN_DECIMALS,
            mint_authority: treasury_info.key(),
            freeze_authority: None,
        }
        .invoke()?;
    }

    // ============================================================
    // TODO: Initialize mint metadata (mpl_token_metadata)
    // ============================================================
    // mpl_token_metadata::instructions::CreateMetadataAccountV3Cpi {
    //     __program: metadata_program_info,
    //     metadata: metadata_info,
    //     mint: mint_info,
    //     mint_authority: treasury_info,
    //     payer: signer_info,
    //     update_authority: (signer_info, true),
    //     system_program: system_program_info,
    //     rent: Some(rent_sysvar_info),
    //     __args: mpl_token_metadata::instructions::CreateMetadataAccountV3InstructionArgs {
    //         data: mpl_token_metadata::types::DataV2 {
    //             name: METADATA_NAME.to_string(),
    //             symbol: METADATA_SYMBOL.to_string(),
    //             uri: METADATA_URI.to_string(),
    //             seller_fee_basis_points: 0,
    //             creators: None,
    //             collection: None,
    //             uses: None,
    //         },
    //         is_mutable: true,
    //         collection_details: None,
    //     },
    // }
    // .invoke_signed(&[&[TREASURY, &[TREASURY_BUMP]]])?;
    // ============================================================

    // Initialize treasury token account (ATA)
    CreateATA {
        funding_account: signer_info,
        account: treasury_ata_info,
        wallet: treasury_info,
        mint: mint_info,
        system_program: system_program_info,
        token_program: token_program_info,
    }
    .invoke()?;

    // Fund the treasury token account with MAX_SUPPLY
    {
        let treasury_bump_binding = [TREASURY_BUMP];
        let treasury_seeds = [
            Seed::from(TREASURY),
            Seed::from(treasury_bump_binding.as_slice()),
        ];
        let treasury_signer = [Signer::from(&treasury_seeds)];

        MintTo {
            mint: mint_info,
            account: treasury_ata_info,
            mint_authority: treasury_info,
            amount: MAX_SUPPLY,
        }
        .invoke_signed(&treasury_signer)?;
    }

    // ============================================================
    // TODO: Create the genesis tape
    // NOTE: These instruction builders (build_create_ix, build_write_ix, etc.)
    // are not yet available in the pinocchio tape-api. They need to be ported
    // from the native implementation before this section can be completed.
    // ============================================================
    /*
    let genesis_name = "genesis";
    let genesis_name_bytes = to_name(genesis_name);
    let (tape_address, _tape_bump) = tape_pda(*signer_info.key(), &genesis_name_bytes);
    let (writer_address, _writer_bump) = writer_pda(tape_address);

    // Build and invoke create_tape instruction
    {
        let create_ix = tape_api::instruction::tape::build_create_ix(*signer_info.key(), genesis_name);

        pinocchio::program::invoke(
            &create_ix,
            &[
                signer_info,
                tape_info,
                writer_info,
                system_program_info,
                rent_sysvar_info,
                slot_hashes_info,
            ],
        )?;
    }

    // Write "hello, world" to the tape
    {
        let write_ix = tape_api::instruction::tape::build_write_ix(
            *signer_info.key(),
            tape_address,
            writer_address,
            b"hello, world",
        );

        pinocchio::program::invoke(&write_ix, &[signer_info, tape_info, writer_info])?;
    }

    // Subsidize the tape for 1 block
    {
        let subsidize_ix = tape_api::instruction::tape::build_subsidize_ix(
            *treasury_info.key(),
            *treasury_ata_info.key(),
            tape_address,
            min_finalization_rent(1),
        );

        let treasury_bump_binding = [TREASURY_BUMP];
        let treasury_seeds = [Seed::from(TREASURY), Seed::from(treasury_bump_binding.as_slice())];
        let treasury_signer = [Signer::from(&treasury_seeds)];

        pinocchio::program::invoke_signed(
            &subsidize_ix,
            &[treasury_info, treasury_ata_info, tape_info],
            &treasury_signer,
        )?;
    }

    // Finalize the tape
    {
        let finalize_ix = tape_api::instruction::tape::build_finalize_ix(
            *signer_info.key(),
            tape_address,
            writer_address,
        );

        pinocchio::program::invoke(
            &finalize_ix,
            &[
                signer_info,
                tape_info,
                writer_info,
                archive_info,
                system_program_info,
                rent_sysvar_info,
            ],
        )?;
    }
    */
    // ============================================================

    // Mark unused accounts to avoid warnings (remove when tape operations are uncommented)
    let _ = (tape_info, writer_info);

    Ok(())
}
