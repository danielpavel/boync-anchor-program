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
use std::result::Result as StdResult;

mod update_auction_claim {

    // use std::println;

    use anchor_lang::prelude::Pubkey;
    use mpl_token_metadata::{instruction::TransferArgs, state::TokenStandard};
    use solana_program_test::ProgramTestContext;
    use spl_associated_token_account::get_associated_token_address;

    use super::*;

    pub async fn setup_transfer_token(
        context: &mut ProgramTestContext,
        token_standard: TokenStandard,
        amount: u64,
    ) -> StdResult<(DigitalAsset, Pubkey, Keypair), BanksClientError> {
        let mut da = DigitalAsset::new();
        da.create_and_mint(context, token_standard, None, None, 1)
            .await
            .unwrap();

        let destination_owner = Keypair::new();
        let destination_token =
            get_associated_token_address(&destination_owner.pubkey(), &da.mint.pubkey());
        airdrop(context, &destination_owner.pubkey(), ONE_SOL)
            .await
            .unwrap();

        let authority = &Keypair::from_bytes(&context.payer.to_bytes()).unwrap();

        let args = TransferArgs::V1 {
            authorization_data: None,
            amount,
        };

        let params = TransferFromParams {
            context,
            authority,
            source_owner: &authority.pubkey(),
            destination_owner: destination_owner.pubkey(),
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

        assert_eq!(token_account.amount, 1);

        Ok((da, destination_token, destination_owner))
    }

    #[tokio::test]
    async fn boync_user_claim() {
        let mut context = program_test().start_with_context().await;

        let token_standard = TokenStandard::ProgrammableNonFungible;
        let (da, destination_token, destination_owner) =
            setup_transfer_token(&mut context, token_standard, 1)
                .await
                .unwrap();

        /*
         * At this point we have the digital asset in the hands of (destination_owner / destination_token)
         * which will act as the auction seller/creator
         */

        let current_timestamp = context
            .banks_client
            .get_sysvar::<Clock>()
            .await
            .unwrap()
            .unix_timestamp
            * MS_IN_SEC;

        let ((auction, auction_bump), treasury, bidders_chest) = find_boync_auction_pdas(
            &destination_owner.pubkey(),
            &da.mint.pubkey(),
            &current_timestamp,
        );

        // let treasury_ata = get_associated_token_address(&treasury, &da.mint.pubkey());

        let auction_duration = 300 * MS_IN_SEC;
        let (_, tx) = boync_initialize_2(
            &mut context,
            &destination_owner,
            &da,
            &auction,
            auction_bump,
            &treasury,
            &bidders_chest,
            &current_timestamp,
            &destination_token,     // creator token
            Some(&auction_duration),
        );

        context.banks_client.process_transaction(tx).await.unwrap();

        let treasury_token_account = Account::unpack_from_slice(
            context
                .banks_client
                .get_account(treasury)
                .await
                .unwrap()
                .unwrap()
                .data
                .as_slice(),
        )
        .unwrap();

        assert_eq!(treasury_token_account.amount, 1);

        let player1 = Keypair::new();
        airdrop(&mut context, &player1.pubkey(), ONE_SOL)
            .await
            .unwrap();

        let player1_token = get_associated_token_address(&player1.pubkey(), &da.mint.pubkey());

        let mut ts = context
            .banks_client
            .get_sysvar::<Clock>()
            .await
            .unwrap()
            .unix_timestamp
            * MS_IN_SEC;
        ts = ts + MS_IN_SEC; // Add a second!

        let (accounts, tx) =
            boync_update_auction_bid(&mut context, &auction, &bidders_chest, &player1, &ts);

        context.banks_client.process_transaction(tx).await.unwrap();

        let bidder = accounts.bidder;
        let bidder_state = accounts.bidder_state;

        /*
         Checks:
           * SOL has been debited from `player1`
           * SOL has been credited to `bidders_chest`
           * Update `auction` account
           * Contents of Player 1's `bidder_state` account
        */

        let balance = context
            .banks_client
            .get_balance(player1.pubkey())
            .await
            .unwrap();
        let expected_balance = ONE_SOL - 150_000_000;
        let fees_spent = expected_balance - balance;

        // When we compare we must take into account the fees + cost of creating the BoyncUserBid which is supported by the bidder
        assert_eq!(balance, (expected_balance - fees_spent));

        let auction_house_data = boync_get_auction_data(&mut context, &auction).await;
        assert_eq!(bidder, auction_house_data.last_bidder);

        let auction_end_time = current_timestamp + auction_duration;
        assert_eq!(
            auction_end_time + (60 * 1000),
            auction_house_data.end_auction_at
        );
        assert_eq!(auction_house_data.next_bid, 157_500_000);
        assert_eq!(auction_house_data.claimed, 0);

        let bidders_chest_balance = context
            .banks_client
            .get_balance(bidders_chest)
            .await
            .unwrap();
        assert_eq!(bidders_chest_balance, 150_000_000);

        let bidder_state_data = boync_get_bidder_state_data(&mut context, &bidder_state).await;
        assert_eq!(bidder, bidder_state_data.bidder);
        assert_eq!(auction, bidder_state_data.auction);
        assert_eq!(150_000_000, bidder_state_data.bid_value);
        assert_eq!(ts, bidder_state_data.ts);

        /* Warp blockchain forward */
        let current_slot = context.banks_client.get_root_slot().await.unwrap();

        context.warp_to_slot(current_slot + 260000).unwrap(); // 260000 slots -> 619 seconds

        let warped_ts = context
            .banks_client
            .get_sysvar::<Clock>()
            .await
            .unwrap()
            .unix_timestamp
            * MS_IN_SEC;

        let auction_house_data = boync_get_auction_data(&mut context, &auction).await;
        assert!(warped_ts > auction_house_data.end_auction_at);

        /* By this point, the auction is ended, we can safely claim */
        let (_claim_accounts, tx) = boync_update_auction_claim(
            &mut context,
            &da,
            &auction,
            &treasury,
            &player1_token,
            &player1,
        );

        context.banks_client.process_transaction(tx).await.unwrap();

        let auction_house_data = boync_get_auction_data(&mut context, &auction).await;
        assert_eq!(auction_house_data.claimed, 1);

        let player1_token_account = Account::unpack_from_slice(
            context
                .banks_client
                .get_account(player1_token)
                .await
                .unwrap()
                .unwrap()
                .data
                .as_slice(),
        )
        .unwrap();

        /* Winner received token */
        assert_eq!(player1_token_account.amount, 1);
    }
}
