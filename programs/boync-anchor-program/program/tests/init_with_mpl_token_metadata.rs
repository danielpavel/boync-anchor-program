#![cfg(feature = "test-bpf")]

pub mod utils;

use solana_program_test::*;
use solana_sdk::{
    signature::{Keypair, Signer},
    sysvar::clock::Clock,
};

use utils::*;

use anchor_lang::*;
use solana_program::program_pack::Pack;
use spl_token::state::Account;

use boync_anchor_program::pda::{
    find_boync_auction_address, find_boync_bidders_chest_address, find_boync_treasury_address,
};

mod standard_transfer {

    use mpl_token_metadata::{instruction::TransferArgs, state::TokenStandard};
    use solana_program::native_token::LAMPORTS_PER_SOL;
    use spl_associated_token_account::get_associated_token_address;

    use super::*;

    #[tokio::test]
    async fn transfer_nonfungible() {
        let mut context = program_test().start_with_context().await;

        let mut da = DigitalAsset::new();
        da.create_and_mint(&mut context, TokenStandard::NonFungible, None, None, 1)
            .await
            .unwrap();

        let destination_owner = Keypair::new().pubkey();
        let destination_token = get_associated_token_address(&destination_owner, &da.mint.pubkey());
        airdrop(&mut context, &destination_owner, LAMPORTS_PER_SOL)
            .await
            .unwrap();

        let authority = &Keypair::from_bytes(&context.payer.to_bytes()).unwrap();

        let args = TransferArgs::V1 {
            authorization_data: None,
            amount: 1,
        };

        let params = TransferFromParams {
            context: &mut context,
            authority,
            source_owner: &authority.pubkey(),
            destination_owner,
            destination_token: None,
            authorization_rules: None,
            payer: authority,
            args,
        };

        da.transfer_from(params).await.unwrap();

        let token_account = Account::unpack_from_slice(
            context
                .banks_client
                .get_account(destination_token)
                .await
                .unwrap()
                .unwrap()
                .data
                .as_slice(),
        )
        .unwrap();

        // let token_account = Account::unpack(
        //     &context
        //         .banks_client
        //         .get_account(destination_token)
        //         .await
        //         .unwrap()
        //         .unwrap()
        //         .data,
        // )
        // .unwrap();

        assert_eq!(token_account.amount, 1);
    }

    #[tokio::test]
    async fn test_boync_initialize_2() {
        let mut context = program_test().start_with_context().await;

        // create and mint the pNFT
        let mut da = DigitalAsset::new();
        da.create_and_mint(&mut context, TokenStandard::NonFungible, None, None, 1)
            .await
            .unwrap();

        let current_timestamp = context
            .banks_client
            .get_sysvar::<Clock>()
            .await
            .unwrap()
            .unix_timestamp;

        let creator = Keypair::new();
        let (auction, auction_bump) =
            find_boync_auction_address(&creator.pubkey(), &da.mint.pubkey(), &current_timestamp);
        let (treasury, _) =
            find_boync_treasury_address(&creator.pubkey(), &da.mint.pubkey(), &current_timestamp);
        let (bidders_chest, _) =
            find_boync_bidders_chest_address(&creator.pubkey(), &current_timestamp);
        let creator_token_account =
            get_associated_token_address(&creator.pubkey(), &da.mint.pubkey());
        let treasury_token_account = get_associated_token_address(&treasury, &da.mint.pubkey());

        let (_, tx) = boync_initialize_2(
            &mut context,
            &creator,
            &da,
            &auction,
            auction_bump,
            &treasury,
            &bidders_chest,
            &current_timestamp,
        );

        context.banks_client.process_transaction(tx).await.unwrap();

        let token_account = Account::unpack_from_slice(
            context
                .banks_client
                .get_account(treasury_token_account)
                .await
                .unwrap()
                .unwrap()
                .data
                .as_slice(),
        )
        .unwrap();

        assert_eq!(token_account.amount, 1);
    }
}
