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

mod standard_transfer {

    use boync_anchor_program::{
        pda::{
            find_boync_auction_address, find_boync_bidders_chest_address,
            find_boync_treasury_address,
        },
        BoyncAuction2,
    };
    use mpl_token_metadata::{instruction::TransferArgs, state::TokenStandard};
    use solana_program::native_token::LAMPORTS_PER_SOL;
    use spl_associated_token_account::get_associated_token_address;

    use super::*;

    #[tokio::test]
    async fn boync_initialize_auction_2() {
        let mut context = program_test().start_with_context().await;

        let mut da = DigitalAsset::new();
        da.create_and_mint(
            &mut context,
            TokenStandard::ProgrammableNonFungible,
            None,
            None,
            1,
        )
        .await
        .unwrap();

        let destination_owner = Keypair::new();
        let destination_token =
            get_associated_token_address(&destination_owner.pubkey(), &da.mint.pubkey());
        airdrop(&mut context, &destination_owner.pubkey(), LAMPORTS_PER_SOL)
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

        // println!("[Boync]");
        // println!("[Boync][auction] {:#?}", auction);
        // println!("[Boync][treasury] {:#?}", treasury);
        // println!("[Boync][bidders_chest] {:#?}", bidders_chest);
        // println!("[Boync][treasury_ata] {:#?}", treasury_ata);
        // println!("[Boync][destination_token] {:#?}", destination_token);
        // println!("[Boync][destination_owner] {:#?}", destination_owner.pubkey());
        // println!("[Boync][mint] {:#?}", da.mint.pubkey());
        // println!("[Boync][metadata] {:#?}", da.metadata);
        // println!("[Boync]");

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

        let auction_house_acc = context
            .banks_client
            .get_account(auction)
            .await
            .expect("account not found")
            .expect("account empty");
        let auction_house_data =
            BoyncAuction2::try_deserialize(&mut auction_house_acc.data.as_ref()).unwrap();

        assert_eq!(
            auction_house_data.starting_price,
            150_000_000
        );
    }
}
