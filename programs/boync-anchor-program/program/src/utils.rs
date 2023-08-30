use anchor_lang::{
    context::CpiContext,
    prelude::*,
    solana_program::{
        instruction::{AccountMeta, Instruction},
        program_memory::sol_memcmp,
        pubkey::PUBKEY_BYTES,
    },
    Accounts,
};

use mpl_token_metadata::{
    instruction::{
        builders::TransferBuilder, InstructionBuilder, MetadataInstruction, TransferArgs,
    },
    processor::AuthorizationData,
    state::{Metadata, ProgrammableConfig, TokenMetadataAccount, TokenStandard},
    utils::assert_derivation,
};

use mpl_token_auth_rules::payload::{Payload, PayloadType, SeedsVec};

use anchor_spl::token::Transfer;

use crate::constants::*;
use crate::errors::*;
use crate::account::BoyncAuction2;

fn build_mpl_token_metadata_instruction_with_builder<'info>(
    ctx: CpiContext<'_, '_, '_, 'info, BoyncTokenTransfer<'info>>,
    app_index: &i64,
    amount: u64,
) -> Result<(Instruction, Vec<AccountInfo<'info>>)> {
    let authority = ctx.accounts.authority.to_account_info();
    let mint = ctx.accounts.mint.to_account_info();

    let args = TransferArgs::V1 {
        authorization_data: Some(AuthorizationData {
            payload: Payload::from([
                ("Amount".to_string(), PayloadType::Number(amount)),
                (
                    "Authority".to_string(),
                    PayloadType::Pubkey(authority.key()),
                ),
                (
                    "AuthoritySeeds".to_string(),
                    PayloadType::Seeds(SeedsVec {
                        seeds: vec![
                            AUCTION_PREFIX.as_bytes().to_vec(),
                            authority.key.to_bytes().to_vec(),
                            mint.key.to_bytes().to_vec(),
                            app_index.to_be_bytes().to_vec(),
                        ],
                    }),
                ),
            ]),
        }),
        amount,
    };

    let mut builder = TransferBuilder::new();
    builder
        .token(ctx.accounts.token.key()) // Token account
        .token_owner(ctx.accounts.token_owner.key()) // Token account owner
        .destination(ctx.accounts.destination.key()) // Destination token account
        .destination_owner(ctx.accounts.destination_owner.key()) // Destination token account owner
        .mint(ctx.accounts.mint.key()) // Mint of token asset
        .metadata(ctx.accounts.metadata.key()) // Metadata (pda of ['metadata', program id, mint id]
        .authority(ctx.accounts.authority.key())
        .payer(ctx.accounts.payer.key()); // Payer

    let mut transfer_infos = vec![
        ctx.accounts.token.to_account_info(),
        ctx.accounts.authority.to_account_info(),
        ctx.accounts.destination.to_account_info(),
        ctx.accounts.destination_owner.to_account_info(),
        ctx.accounts.mint.to_account_info(),
        ctx.accounts.metadata.to_account_info(),
        ctx.accounts.system_program.to_account_info(),
        ctx.accounts.sysvar_instructions.to_account_info(),
        ctx.accounts.spl_token_program.to_account_info(),
        ctx.accounts.spl_ata_program.to_account_info(),
    ];

    let metadata = Metadata::from_account_info(&ctx.accounts.metadata)?;

    if matches!(
        metadata.token_standard,
        Some(TokenStandard::ProgrammableNonFungible)
    ) {
        let master_edition = ctx.accounts.edition.to_account_info();
        let owner_token_record = ctx.accounts.owner_token_record.to_account_info();
        let destination_token_record = ctx.accounts.destination_token_record.to_account_info();

        builder
            .edition(master_edition.key())
            .owner_token_record(owner_token_record.key())
            .destination_token_record(destination_token_record.key());

        transfer_infos.push(master_edition);
        transfer_infos.push(owner_token_record);
        transfer_infos.push(destination_token_record);

        if let Some(ProgrammableConfig::V1 { rule_set: Some(_) }) = metadata.programmable_config {
            let auth_rules_program = ctx.accounts.auth_rules_program.clone();
            let auth_rules = ctx.accounts.auth_rules.clone();

            builder
                .authorization_rules_program(auth_rules_program.key())
                .authorization_rules(auth_rules.key());
            transfer_infos.push(auth_rules_program);
            transfer_infos.push(auth_rules);
        }
    }

    let transfer_ix = builder.build(args).unwrap().instruction();

    Ok((transfer_ix, transfer_infos))
}

/* Another way of building the token metadata transfer instruction.
  Leave it be, for now...
*/
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

pub fn token_transfer<'info>(
    ctx: CpiContext<'_, '_, '_, 'info, BoyncTokenTransfer<'info>>,
    app_index: &i64,
    amount: u64,
) -> Result<()> {
    /*
     *   Both spl_transfer and mpl_token_metadata::Transfer need a valid destination_associate_token_account, so
     *   create it if its data is empty.
     *
     *  [TODO] Possibly reduntant because of anchor constraints - review.
     */
    if ctx.accounts.destination.data_is_empty() {
        // if the token account is empty, we will initialize a new one but it must
        // be a ATA account
        assert_derivation(
            &spl_associated_token_account::id(),
            &ctx.accounts.destination,
            &[
                ctx.accounts.destination_owner.key.as_ref(),
                spl_token::id().as_ref(),
                ctx.accounts.mint.key.as_ref(),
            ],
        )?;

        // creating the associated token account
        solana_program::program::invoke(
            &spl_associated_token_account::instruction::create_associated_token_account(
                ctx.accounts.payer.key,
                ctx.accounts.destination_owner.key,
                ctx.accounts.mint.key,
                &spl_token::id(),
            ),
            &[
                ctx.accounts.payer.clone(),
                ctx.accounts.destination_owner.clone(),
                ctx.accounts.mint.clone(),
                ctx.accounts.destination.clone(),
            ],
        )?;
    }

    let signer_seeds = &ctx.signer_seeds.clone();
    let metadata = Metadata::from_account_info(&ctx.accounts.metadata)?;

    match metadata.token_standard {
        Some(TokenStandard::ProgrammableNonFungible) => {
            let (ix, accounts) =
                build_mpl_token_metadata_instruction_with_builder(ctx, app_index, amount).unwrap();

            solana_program::program::invoke_signed(&ix, &accounts, &signer_seeds)?;
        }
        _ => {
            let transfer_instruction = Transfer {
                from: ctx.accounts.token.to_account_info(),
                to: ctx.accounts.destination.to_account_info(),
                authority: ctx.accounts.authority.to_account_info(),
            };

            let cpi_ctx = CpiContext::new_with_signer(
                ctx.accounts.spl_token_program.to_account_info(),
                transfer_instruction,
                &signer_seeds,
            );

            anchor_spl::token::transfer(cpi_ctx, 1)?
        }
    }

    Ok(())
}

#[derive(Accounts, Debug)]
pub struct BoyncTokenTransfer<'info> {
    pub auction_state: AccountInfo<'info>, // Auction state account

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

    pub auth_rules_program: AccountInfo<'info>, // Token Authorization Rules Program
    pub auth_rules: AccountInfo<'info>,         // Token Authorization Rules account
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

pub fn assert_auction_active(listing_config: &Account<BoyncAuction2>) -> Result<()> {
    let clock = Clock::get()?;
    let current_timestamp = clock.unix_timestamp * MS_IN_SEC;

    if current_timestamp < listing_config.start_auction_at {
        return err!(AuctionError::AuctionNotStarted);
    } else if current_timestamp > listing_config.end_auction_at {
        return err!(AuctionError::AuctionEnded);
    }

    Ok(())
}

pub fn assert_auction_over(listing_config: &Account<BoyncAuction2>) -> Result<()> {
    let clock = Clock::get()?;
    let current_timestamp = clock.unix_timestamp * MS_IN_SEC;

    if current_timestamp < listing_config.end_auction_at {
        return err!(AuctionError::AuctionActive);
    }

    Ok(())
}

pub fn process_time_extension(listing_config: &mut Account<BoyncAuction2>) -> Result<()> {
    let clock = Clock::get()?;
    let current_timestamp = clock.unix_timestamp * MS_IN_SEC;

    if current_timestamp <= listing_config.end_auction_at {
        listing_config.end_auction_at += i64::from(60 * MS_IN_SEC);
    }

    Ok(())
}
