use crate::instruction::mine::miner_mine::get_base_rate;
use crate::state::*;
use crate::utils::account_traits::AccountInfoExt;
use crate::utils::get_pda::GetPda;
use crate::utils::helpers::{cast_account_data_mut, create_program_account};
use core::cmp::min;
use pinocchio::{
    account_info::AccountInfo,
    cpi::{slice_invoke, slice_invoke_signed},
    instruction::{AccountMeta, Instruction, Seed, Signer},
    msg,
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
use tape_api::utils::compute_next_challenge;

// Borsh serialization for metadata CPI
use borsh::BorshSerialize;

extern crate alloc;
use alloc::string::{String, ToString};
use alloc::vec;
use alloc::vec::Vec;

/// Helper to convert string to fixed-size array
#[inline(always)]
fn string_to_bytes<const N: usize>(s: &str) -> [u8; N] {
    let mut out = [0u8; N];
    let bytes = s.as_bytes();
    let len = min(bytes.len(), N);
    out[..len].copy_from_slice(&bytes[..len]);
    out
}

/// Helper to convert URI string to [u64; 32] array (unused)
#[inline(always)]
#[allow(dead_code)]
fn string_to_uri(s: &str) -> [u64; 32] {
    let bytes = s.as_bytes();
    let mut uri = [0u64; 32];

    // Pack bytes into u64s
    for i in 0..min(bytes.len(), 256) {
        let byte_pos = i / 8;
        let shift = (i % 8) * 8;
        uri[byte_pos] |= (bytes[i] as u64) << shift;
    }

    uri
}

/// Metaplex Token Metadata DataV2 struct (Borsh-serializable)
#[derive(BorshSerialize)]
struct MetadataDataV2 {
    name: String,
    symbol: String,
    uri: String,
    seller_fee_basis_points: u16,
    creators: Option<Vec<MetadataCreator>>,
    collection: Option<MetadataCollection>,
    uses: Option<MetadataUses>,
}

/// Metaplex Creator struct
#[derive(BorshSerialize)]
struct MetadataCreator {
    address: [u8; 32],
    verified: bool,
    share: u8,
}

/// Metaplex Collection struct
#[derive(BorshSerialize)]
struct MetadataCollection {
    verified: bool,
    key: [u8; 32],
}

/// Metaplex Uses struct
#[derive(BorshSerialize)]
struct MetadataUses {
    use_method: u8,
    remaining: u64,
    total: u64,
}

/// CreateMetadataAccountV3 instruction args
#[derive(BorshSerialize)]
struct CreateMetadataAccountV3Args {
    data: MetadataDataV2,
    is_mutable: bool,
    collection_details: Option<u8>, // Simplified - None for our use case
}

/// Build Borsh-serialized metadata instruction data using proper borsh crate with std
fn build_metadata_instruction_data_borsh(
    name: &str,
    symbol: &str,
    uri: &str,
    seller_fee_basis_points: u16,
    is_mutable: bool,
) -> Result<Vec<u8>, ProgramError> {
    let args = CreateMetadataAccountV3Args {
        data: MetadataDataV2 {
            name: name.to_string(),
            symbol: symbol.to_string(),
            uri: uri.to_string(),
            seller_fee_basis_points,
            creators: None,
            collection: None,
            uses: None,
        },
        is_mutable,
        collection_details: None,
    };

    // Build instruction data
    let mut data = vec![33]; // CreateMetadataAccountV3 discriminator
    args.serialize(&mut data)
        .map_err(|_| ProgramError::InvalidInstructionData)?;

    Ok(data)
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

    // Only check that tape_program_info matches TAPE_ID
    // Verify program ownership
    tape_program_info.is_program_check()?;

    // Initialize epoch
    create_program_account::<Epoch>(
        epoch_info,
        system_program_info,
        signer_info,
        &TAPE_ID,
        &[EPOCH],
    )?;

    // Set epoch fields
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

    // Initialize treasury
    create_program_account::<Treasury>(
        treasury_info,
        system_program_info,
        signer_info,
        &TAPE_ID,
        &[TREASURY],
    )?;

    // Initialize mint
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

    // Initialize mint metadata using Pinocchio CPI with Borsh serialization
    {
        let instruction_data = build_metadata_instruction_data_borsh(
            METADATA_NAME,
            METADATA_SYMBOL,
            METADATA_URI,
            0,    // seller_fee_basis_points
            true, // is_mutable
        )?;

        // Build CPI instruction to Metaplex
        // Account order for CreateMetadataAccountV3:
        // 0. metadata (writable)
        // 1. mint (readonly)
        // 2. mint_authority (readonly, signer via PDA)
        // 3. payer (writable, signer)
        // 4. update_authority (writable, signer)
        // 5. system_program (readonly)
        // 6. rent (readonly)
        let instruction = Instruction {
            program_id: &MPL_TOKEN_METADATA_ID,
            accounts: &[
                AccountMeta::writable(metadata_info.key()),
                AccountMeta::readonly(mint_info.key()),
                AccountMeta::readonly_signer(treasury_info.key()),
                AccountMeta::writable_signer(signer_info.key()),
                AccountMeta::writable_signer(signer_info.key()),
                AccountMeta::readonly(system_program_info.key()),
                AccountMeta::readonly(rent_sysvar_info.key()),
            ],
            data: &instruction_data,
        };

        // Prepare account infos for CPI
        let account_infos = [
            metadata_info,
            mint_info,
            treasury_info,
            signer_info,
            signer_info,
            system_program_info,
            rent_sysvar_info,
        ];

        // Invoke with treasury PDA as signer
        let treasury_bump_binding = [TREASURY_BUMP];
        let treasury_seeds = [
            Seed::from(TREASURY),
            Seed::from(treasury_bump_binding.as_slice()),
        ];
        let treasury_signer = [Signer::from(&treasury_seeds)];

        slice_invoke_signed(&instruction, &account_infos, &treasury_signer)?;
    }

    // Initialize treasury ATA
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

    Ok(())
}
