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

mod standard_transfer {

    use anchor_lang::prelude::Pubkey;
    use boync_anchor_program::{
        pda::{
            find_boync_auction_address, find_boync_bidders_chest_address,
            find_boync_treasury_address,
        },
        BoyncAuction2,
    };
    use mpl_token_metadata::{instruction::TransferArgs, state::TokenStandard};
    use solana_program::native_token::LAMPORTS_PER_SOL;
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
        airdrop(context, &destination_owner.pubkey(), LAMPORTS_PER_SOL)
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

        Ok((da, destination_token, destination_owner))
    }

    #[tokio::test]
    async fn boync_initialize_auction_2_programmable_non_fungible() {
        let mut context = program_test().start_with_context().await;

        let token_standard = TokenStandard::NonFungible;
        let (da, destination_token, destination_owner) =
            setup_transfer_token(&mut context, token_standard, 1)
                .await
                .unwrap();

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

        /*
         * At this point we have the digital asset in the hands of (destination_owner / destination_token)
         * which will act as the auction seller/creator
         */

        let current_timestamp = context
            .banks_client
            .get_sysvar::<Clock>()
            .await
            .unwrap()
            .unix_timestamp;

        let (auction, auction_bump) = find_boync_auction_address(
            &destination_owner.pubkey(),
            &da.mint.pubkey(),
            &current_timestamp,
        );
        let (treasury, _) = find_boync_treasury_address(
            &destination_owner.pubkey(),
            &da.mint.pubkey(),
            &current_timestamp,
        );
        let (bidders_chest, _) =
            find_boync_bidders_chest_address(&destination_owner.pubkey(), &current_timestamp);
        let treasury_ata = get_associated_token_address(&treasury, &da.mint.pubkey());

        let (_, tx) = boync_initialize_2(
            &mut context,
            &destination_owner,
            &da,
            &auction,
            auction_bump,
            &treasury,
            &bidders_chest,
            &current_timestamp,
            &destination_token,
            &treasury_ata,
            token_standard
        );

        context.banks_client.process_transaction(tx).await.unwrap();

        let treasury_token_account = Account::unpack_from_slice(
            context
                .banks_client
                .get_account(treasury_ata)
                .await
                .unwrap()
                .unwrap()
                .data
                .as_slice(),
        )
        .unwrap();

        assert_eq!(treasury_token_account.amount, 1);

        /*
         * Check contents of the auction house
         */
        let auction_house_acc = context
            .banks_client
            .get_account(auction)
            .await
            .expect("account not found")
            .expect("account empty");
        let auction_house_data =
            BoyncAuction2::try_deserialize(&mut auction_house_acc.data.as_ref()).unwrap();

        assert_eq!(auction_house_data.starting_price, 150_000_000);
        assert_eq!(auction_house_data.start_auction_at, current_timestamp);
        assert_eq!(
            auction_house_data.end_auction_at,
            current_timestamp + ((30 as i64) * (60 * 60 * 1000 as i64))
        );
    }
}
