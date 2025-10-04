use crate::instruction::mine::miner_mine::get_base_rate;
use crate::metadata::{
    collection_details::CollectionDetails,
    create_metadata_account_v3::{
        CreateMetadataAccountV3Cpi, CreateMetadataAccountV3InstructionArgs,
    },
    data_v2::DataV2,
};
use crate::state::*;
use crate::utils::account_traits::AccountInfoExt;
use crate::utils::get_pda::GetPda;
use crate::utils::helpers::{cast_account_data_mut, create_program_account};
use core::cmp::min;
use pinocchio::{
    account_info::AccountInfo,
    cpi::{slice_invoke, slice_invoke_signed},
    instruction::{AccountMeta, Instruction, Seed, Signer},
    program_error::ProgramError,
    sysvars::{rent::Rent, Sysvar},
    ProgramResult,
};
use pinocchio_associated_token_account::instructions::Create as CreateATA;
use pinocchio_system::instructions::CreateAccount;
use pinocchio_token::instructions::{InitializeMint2, MintTo};
use tape_api::consts::{
    BLOCK_ADDRESS, MAX_SUPPLY, METADATA_NAME, METADATA_SYMBOL, METADATA_URI, MINT_BUMP, MINT_SEED,
    MIN_MINING_DIFFICULTY, MIN_PACKING_DIFFICULTY, MIN_PARTICIPATION_TARGET, TOKEN_DECIMALS,
    TREASURY_BUMP,
};
use tape_api::instruction::tape::{
    build_create_ix_data, build_finalize_ix_data, build_subsidize_ix_data, build_write_ix_data,
};
use tape_api::pda::{tape_pda, writer_pda};
use tape_api::rent::min_finalization_rent;
use tape_api::utils::{compute_next_challenge, to_name};

/// Helper to convert string to fixed-size [u8; N] array (padded with zeros)
#[inline(always)]
fn string_to_bytes<const N: usize>(s: &str) -> [u8; N] {
    let mut out = [0u8; N];
    let bytes = s.as_bytes();
    let len = min(bytes.len(), N);
    out[..len].copy_from_slice(&bytes[..len]);
    out
}

/// Helper to convert URI string to [u64; 32] array for DataV2
#[inline(always)]
fn string_to_uri(s: &str) -> [u64; 32] {
    let bytes = s.as_bytes();
    let mut uri = [0u64; 32];

    // Pack bytes into u64s (8 bytes per u64)
    for i in 0..min(bytes.len(), 256) {
        let byte_pos = i / 8;
        let shift = (i % 8) * 8;
        uri[byte_pos] |= (bytes[i] as u64) << shift;
    }

    uri
}

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

    // Initialize mint metadata
    {
        // Build DataV2 metadata
        let metadata_data = DataV2 {
            name: string_to_bytes::<32>(METADATA_NAME),
            symbol: string_to_bytes::<10>(METADATA_SYMBOL),
            uri: string_to_uri(METADATA_URI),
            seller_fee_basis_points: 0,
            creator_count: 0,
            creators: [crate::metadata::creator::Creator::none(); 5],
            collection: crate::metadata::collection::Collection::none(),
            uses: crate::metadata::uses::Uses::none(),
        };

        // Build instruction args
        let args = CreateMetadataAccountV3InstructionArgs {
            data: metadata_data,
            is_mutable: 1, // true
            collection_details: CollectionDetails::default(),
            collection_details_present: 0, // None
            _padding: [0; 6],
        };

        // Create treasury signer for signing the metadata creation
        let treasury_bump_binding = [TREASURY_BUMP];
        let treasury_seeds = [
            Seed::from(TREASURY),
            Seed::from(treasury_bump_binding.as_slice()),
        ];
        let treasury_signer = [Signer::from(&treasury_seeds)];

        // Invoke CreateMetadataAccountV3
        CreateMetadataAccountV3Cpi {
            __program: metadata_program_info,
            metadata: metadata_info,
            mint: mint_info,
            mint_authority: treasury_info,
            payer: signer_info,
            update_authority: (signer_info, true),
            system_program: system_program_info,
            rent_present: 1, // Some
            rent: rent_sysvar_info,
            __args: args,
        }
        .invoke_signed(&treasury_signer)?;
    }

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
    // Create the genesis tape
    // ============================================================

    let genesis_name = "genesis";

    // Build create tape instruction data
    let mut create_ix_data = [0u8; 128]; // Buffer for instruction data
    let (create_data_len, tape_address, writer_address) =
        build_create_ix_data(signer_info.key(), genesis_name, &mut create_ix_data);

    // Create the tape
    {
        let create_account_metas = [
            AccountMeta::new(signer_info.key(), true, true), // writable, signer
            AccountMeta::new(&tape_address, true, false),    // writable, not signer
            AccountMeta::new(&writer_address, true, false),  // writable, not signer
            AccountMeta::new(system_program_info.key(), false, false), // readonly, not signer
            AccountMeta::new(rent_sysvar_info.key(), false, false), // readonly, not signer
            AccountMeta::new(slot_hashes_info.key(), false, false), // readonly, not signer
        ];

        let create_instruction = Instruction {
            program_id: &TAPE_ID,
            accounts: &create_account_metas,
            data: &create_ix_data[..create_data_len],
        };

        slice_invoke(
            &create_instruction,
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
        let mut write_ix_data = [0u8; 256]; // Buffer for write instruction data
        let write_data = b"hello, world";
        let write_data_len = build_write_ix_data(write_data, &mut write_ix_data);

        let write_account_metas = [
            AccountMeta::new(signer_info.key(), true, true), // writable, signer
            AccountMeta::new(&tape_address, true, false),    // writable, not signer
            AccountMeta::new(&writer_address, true, false),  // writable, not signer
        ];

        let write_instruction = Instruction {
            program_id: &TAPE_ID,
            accounts: &write_account_metas,
            data: &write_ix_data[..write_data_len],
        };

        slice_invoke(&write_instruction, &[signer_info, tape_info, writer_info])?;
    }

    // Subsidize the tape for 1 block
    {
        let mut subsidize_ix_data = [0u8; 64]; // Buffer for subsidize instruction data
        let subsidize_amount = min_finalization_rent(1);
        let subsidize_data_len = build_subsidize_ix_data(subsidize_amount, &mut subsidize_ix_data);

        let subsidize_account_metas = [
            AccountMeta::new(treasury_info.key(), true, true), // writable, signer
            AccountMeta::new(treasury_ata_info.key(), true, false), // writable, not signer
            AccountMeta::new(&tape_address, true, false),      // writable, not signer
            AccountMeta::new(treasury_ata_info.key(), true, false), // writable, not signer (destination)
            AccountMeta::new(token_program_info.key(), false, false), // readonly, not signer
        ];

        let subsidize_instruction = Instruction {
            program_id: &TAPE_ID,
            accounts: &subsidize_account_metas,
            data: &subsidize_ix_data[..subsidize_data_len],
        };

        let treasury_bump_binding = [TREASURY_BUMP];
        let treasury_seeds = [
            Seed::from(TREASURY),
            Seed::from(treasury_bump_binding.as_slice()),
        ];
        let treasury_signer = [Signer::from(&treasury_seeds)];

        slice_invoke_signed(
            &subsidize_instruction,
            &[
                treasury_info,
                treasury_ata_info,
                tape_info,
                treasury_ata_info, // same account again for destination
                token_program_info,
            ],
            &treasury_signer,
        )?;
    }

    // Finalize the tape
    {
        let mut finalize_ix_data = [0u8; 64]; // Buffer for finalize instruction data
        let finalize_data_len = build_finalize_ix_data(&mut finalize_ix_data);

        let finalize_account_metas = [
            AccountMeta::new(signer_info.key(), true, true), // writable, signer
            AccountMeta::new(&tape_address, true, false),    // writable, not signer
            AccountMeta::new(&writer_address, true, false),  // writable, not signer
            AccountMeta::new(archive_info.key(), true, false), // writable, not signer
            AccountMeta::new(system_program_info.key(), false, false), // readonly, not signer
            AccountMeta::new(rent_sysvar_info.key(), false, false), // readonly, not signer
        ];

        let finalize_instruction = Instruction {
            program_id: &TAPE_ID,
            accounts: &finalize_account_metas,
            data: &finalize_ix_data[..finalize_data_len],
        };

        slice_invoke(
            &finalize_instruction,
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

    // ============================================================

    Ok(())
}
