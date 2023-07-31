use anchor_lang::{
    context::CpiContext,
    prelude::*,
    solana_program::{instruction::{AccountMeta, Instruction}, program_memory::sol_memcmp, pubkey::PUBKEY_BYTES},
    Accounts,
};

use mpl_token_metadata::{
    instruction::{
        builders::TransferBuilder, InstructionBuilder, MetadataInstruction, TransferArgs,
    },
    pda::{find_master_edition_account, find_token_record_account},
};

use spl_associated_token_account::get_associated_token_address;

use crate::errors::*;

fn build_mpl_token_metadata_instruction_with_builder(
    authority: &Pubkey,
    source_owner: &Pubkey,
    token: &Pubkey,
    destination_owner: &Pubkey,
    destination_token: Option<Pubkey>,
    metadata: &Pubkey,
    payer: &Pubkey,
    authorization_rules: Option<Pubkey>,
    mint: &Pubkey,
    token_record: Option<Pubkey>,
    master_edition: Option<Pubkey>,
    amount: u64,
) -> Instruction {
    let args = TransferArgs::V1 {
        authorization_data: None,
        amount: amount,
    };

    let destination_token = if let Some(destination_token) = destination_token {
        destination_token
    } else {
        get_associated_token_address(destination_owner, &mint)
    };

    let mut builder = TransferBuilder::new();
    builder
        .authority(*authority)
        .token_owner(*source_owner)
        .token(*token) // Token account
        .destination_owner(*destination_owner) // Destination token account owner
        .destination(destination_token) // Destination token account
        .metadata(*metadata) // Metadata (pda of ['metadata', program id, mint id]
        .payer(*payer) // Payer
        .mint(*mint); // Mint of token asset

    let record = if let Some(token_record) = token_record {
        token_record
    } else {
        let (record, _) = find_token_record_account(&mint, &token);
        record
    };
    builder.owner_token_record(record);

    // This can be optional for non pNFTs but always include it for now.
    let (destination_token_record, _bump) = find_token_record_account(&mint, &destination_token);
    builder.destination_token_record(destination_token_record);

    let master = if let Some(master_edition) = master_edition {
        master_edition
    } else {
        let (master, _) = find_master_edition_account(&mint);
        master
    };
    builder.edition(master);

    if let Some(authorization_rules) = authorization_rules {
        builder.authorization_rules(authorization_rules);
        builder.authorization_rules_program(mpl_token_auth_rules::ID);
    }

    let transfer_ix = builder.build(args).unwrap().instruction();

    // msg!("[build_mpl_token_metadata_instruction_with_builder] with accounts:");
    // msg!("[destination_token] {:#?}", destination_token);
    // msg!("[destination_owner] {:#?}", destination_owner);
    // msg!("[source_token] {:#?}", token);
    // msg!("[source_owner] {:#?}", source_owner);
    // msg!("[edition] {:#?}", master_edition);
    // msg!("[meta] {:#?}", metadata);
    // msg!("[owner_token_rec] {:#?}", record);
    // msg!("[destination_token_rec] {:#?}", destination_token_record);

    transfer_ix
}

pub fn _build_mpl_token_metadata_transfer<'info>(
    token: Pubkey,
    token_owner: Pubkey,
    destination: Pubkey,
    destination_owner: Pubkey,
    mint: Pubkey,
    metadata: Pubkey,
    edition: Option<Pubkey>,
    owner_token_record: Option<Pubkey>,
    destination_token_record: Option<Pubkey>,
    authority: Pubkey,
    payer: Pubkey,
    system_program: Pubkey,
    sysvar_instructions: Pubkey,
    spl_token_program: Pubkey,
    spl_ata_program: Pubkey,
    authorization_rules: Option<Pubkey>,
    authorization_rules_program: Option<Pubkey>,
    amount: u64,
) -> anchor_lang::solana_program::instruction::Instruction {
    let args = TransferArgs::V1 {
        authorization_data: None,
        amount: amount,
    };

    let mut accounts = vec![
        AccountMeta::new(token, false),
        AccountMeta::new_readonly(token_owner, false),
        AccountMeta::new(destination, false),
        AccountMeta::new_readonly(destination_owner, false),
        AccountMeta::new_readonly(mint, false),
        AccountMeta::new(metadata, false),
        AccountMeta::new_readonly(edition.unwrap_or(mpl_token_metadata::ID), false),
        if let Some(owner_token_record) = owner_token_record {
            AccountMeta::new(owner_token_record, false)
        } else {
            AccountMeta::new_readonly(mpl_token_metadata::ID, false)
        },
        if let Some(destination_token_record) = destination_token_record {
            AccountMeta::new(destination_token_record, false)
        } else {
            AccountMeta::new_readonly(mpl_token_metadata::ID, false)
        },
        AccountMeta::new_readonly(authority, true),
        AccountMeta::new(payer, true),
        AccountMeta::new_readonly(system_program, false),
        AccountMeta::new_readonly(sysvar_instructions, false),
        AccountMeta::new_readonly(spl_token_program, false),
        AccountMeta::new_readonly(spl_ata_program, false),
    ];
    // Optional authorization rules accounts
    if let Some(rules) = &authorization_rules {
        accounts.push(AccountMeta::new_readonly(
            authorization_rules_program.unwrap_or(mpl_token_auth_rules::ID),
            false,
        ));
        accounts.push(AccountMeta::new_readonly(*rules, false));
    } else {
        accounts.push(AccountMeta::new_readonly(mpl_token_auth_rules::ID, false));
        accounts.push(AccountMeta::new_readonly(mpl_token_auth_rules::ID, false));
    }

    Instruction {
        program_id: mpl_token_metadata::ID,
        accounts,
        data: MetadataInstruction::Transfer(args.clone())
            .try_to_vec()
            .unwrap(),
    }
}

pub fn token_metadata_transfer<'info>(
    ctx: CpiContext<'_, '_, '_, 'info, MetadataTransfer<'info>>,
    amount: u64,
) -> Result<()> {
    let ix = build_mpl_token_metadata_instruction_with_builder(
        ctx.accounts.authority.key,
        ctx.accounts.authority.key,
        ctx.accounts.token.key,
        ctx.accounts.destination_owner.key,
        Some(*ctx.accounts.destination.key),
        ctx.accounts.metadata.key,
        ctx.accounts.payer.key,
        None,
        ctx.accounts.mint.key,
        None,
        Some(*ctx.accounts.edition.key),
        amount
    );

    solana_program::program::invoke_signed(
        &ix,
        &[
            ctx.accounts.token.clone(), // Token account
            ctx.accounts.token_owner.clone(), // Token account Owner
            ctx.accounts.destination.clone(), // Destination token account
            ctx.accounts.destination_owner.clone(), // Destination token account owner
            ctx.accounts.mint.clone(),  // Mint
            ctx.accounts.metadata.clone(), // Token Metadata
            ctx.accounts.edition.clone(), // Edition
            ctx.accounts.owner_token_record.clone(), // Owner token record
            ctx.accounts.destination_token_record.clone(), // Destination token record
            ctx.accounts.authority.clone(), // Authority
            ctx.accounts.payer.clone(), // Payer
            ctx.accounts.system_program.clone(), // System Program
            ctx.accounts.sysvar_instructions.clone(), // Sysvar Instructions
            ctx.accounts.spl_token_program.clone(), // SPL Token Program
            ctx.accounts.spl_ata_program.clone(), // System Program
            ctx.accounts.authorization_rules.clone(), // Authorization rules
            ctx.accounts.authorization_rules_program.clone(), // Authorization rules program
        ],
        ctx.signer_seeds,
    )
    .map_err(Into::into)
}

#[derive(Accounts)]
pub struct MetadataTransfer<'info> {
    pub token: AccountInfo<'info>,             // Token account
    pub token_owner: AccountInfo<'info>,       // Token account owner
    pub destination: AccountInfo<'info>,       // Destination token account
    pub destination_owner: AccountInfo<'info>, // Destination token account owner

    pub mint: AccountInfo<'info>,     // Mint of token asset
    pub metadata: AccountInfo<'info>, // Metadata (pda of ['metadata', program id, mint id])
    pub edition: AccountInfo<'info>,  // Edition of token asset

    pub owner_token_record: AccountInfo<'info>, // Owner token record account
    pub destination_token_record: AccountInfo<'info>, // Destination token record account

    pub authority: AccountInfo<'info>, // Transfer authority (token owner or delegate)
    pub payer: AccountInfo<'info>,     //  Payer

    pub system_program: AccountInfo<'info>, // System Program
    pub sysvar_instructions: AccountInfo<'info>, // Instructions sysvar account
    pub spl_token_program: AccountInfo<'info>, // SPL Token Program
    pub spl_ata_program: AccountInfo<'info>, // SPL Associated Token Account Program

    pub authorization_rules_program: AccountInfo<'info>, //Token Authorization Rules Program
    pub authorization_rules: AccountInfo<'info>,         //  Token Authorization Rules account
}

/* TODO: Could not find the trait anchor_lang::Id implementation for Authorization Rules Program, and Token Metadata */
#[derive(Clone)]
pub struct AuthRulesTokenProgram;
impl anchor_lang::Id for AuthRulesTokenProgram {
    fn id() -> Pubkey {
        mpl_token_auth_rules::ID
    }
}
pub struct TokenMetadataProgram;
impl anchor_lang::Id for TokenMetadataProgram {
    fn id() -> Pubkey {
        mpl_token_metadata::ID
    }
}
pub struct SysvarInstructions;
impl anchor_lang::Id for SysvarInstructions {
    fn id() -> Pubkey {
        solana_program::sysvar::instructions::ID
    }
}

/* Assert helpers */
pub fn assert_keys_equal(key1: Pubkey, key2: Pubkey) -> Result<()> {
    if sol_memcmp(key1.as_ref(), key2.as_ref(), PUBKEY_BYTES) != 0 {
        err!(AuctionError::PublicKeyMismatch)
    } else {
        Ok(())
    }
}
