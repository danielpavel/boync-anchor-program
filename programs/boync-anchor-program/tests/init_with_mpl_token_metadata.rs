
#![cfg(feature = "test-bpf")]

pub mod utils;

use solana_program_test::*;
use solana_sdk::signature::{Keypair, Signer};

use utils::*;

use solana_program::program_pack::Pack;
use spl_token::state::Account;

mod standard_transfer {

    use mpl_token_metadata::{
        instruction::TransferArgs,
        state::TokenStandard,
    };
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

}

