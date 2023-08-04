use super::digital_asset::*;
use solana_program_test::*;
use anchor_lang::*;

use anchor_client::solana_sdk::{
    instruction::Instruction,
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    system_program, sysvar,
    transaction::Transaction,
};

use boync_anchor_program::{
    accounts::InitializeAuction2 as InitializeAuction2Accounts,
    instruction::InitializeAuction2 as InitializeAuction2Data,
};
use mpl_token_metadata::{
    pda::{find_token_record_account, find_master_edition_account},
    state::TokenStandard
};

const ONE_SOL: u64 = 1_000_000_000;
const ONE_MINUTE_IN_MSEC: i64 = 60 * 60 * 1000;

pub fn boync_initialize_2(
    context: &mut ProgramTestContext,
    creator: &Keypair,
    digital_asset: &DigitalAsset,
    auction: &Pubkey,
    auction_bump: u8,
    treasury: &Pubkey,
    bidders_chest: &Pubkey,
    timestamp: &i64,
    creator_ata: &Pubkey,
    treasury_ata: &Pubkey,
) -> (InitializeAuction2Accounts, Transaction) {

    // let token = &digital_asset.token.pubkey();
    let mint = &digital_asset.mint.pubkey();

    let edition = if let Some(edition) = digital_asset.master_edition {
        edition
    } else {
        let (edition, _) = find_master_edition_account(mint);
        edition
    };

    let (owner_token_record, _) = find_token_record_account(mint, &creator_ata);
    // This can be optional for non pNFTs but always include it for now.
    let (destination_token_record, _bump) = find_token_record_account(mint, &treasury_ata);

    println!("[BoyncTest][boync_initialize_2] auction_state: {:#?} treasury: {:#?} treasury_ata: {:#?} owner_tr: {:#?}, dest_tr {:#?}, creator {:#?} creator_ata {:#?} ",
        *auction,
        *treasury,
        treasury_ata,
        owner_token_record,
        destination_token_record,
        creator.pubkey(),
        creator_ata,
    );

    let accounts = InitializeAuction2Accounts {
        state: *auction,
        treasury: *treasury,
        treasury_ata: *treasury_ata,
        bidders_chest: *bidders_chest,
        signer: creator.pubkey(),
        treasury_mint: *mint,
        metadata: digital_asset.metadata,
        edition,
        owner_token_record,
        destination_token_record,
        auth_rules: mpl_token_auth_rules::id(), // !!!NOT USED
        signer_ata: *creator_ata,
        system_program: system_program::id(),
        token_program: spl_token::id(),
        associated_token_program: spl_associated_token_account::id(),
        auth_rules_token_program:  mpl_token_auth_rules::id(),
        token_metadata_program: mpl_token_metadata::id(),
        rent: sysvar::rent::id(),
        sysvar_instructions: sysvar::instructions::id(),
    };
    let accounts_meta = accounts.to_account_metas(None);

    let data = InitializeAuction2Data {
        app_idx: *timestamp,
        state_bump: auction_bump,
        fp: 3 * ONE_SOL,
        start_at: *timestamp,
        end_at: *timestamp + ((30 as i64) * ONE_MINUTE_IN_MSEC),
    }
    .data();

    let instruction = Instruction {
        program_id: boync_anchor_program::id(),
        data,
        accounts: accounts_meta,
    };

    (
        accounts,
        Transaction::new_signed_with_payer(
            &[instruction],
            Some(&creator.pubkey()),
            &[creator],
            context.last_blockhash,
        ),
    )
}
