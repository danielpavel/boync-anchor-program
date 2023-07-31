use mpl_token_metadata::{
    id, instruction,
    state::{
        Collection, CollectionDetails, Creator, DataV2, Metadata as TmMetadata,
        TokenMetadataAccount, TokenStandard, Uses, PREFIX,
    },
};

use solana_program::borsh::try_from_slice_unchecked;
use solana_sdk::{
    pubkey::Pubkey, signature::Signer, signer::keypair::Keypair, transaction::Transaction,
};

use std::result::Result as StdResult;
use crate::*;

#[derive(Debug)]
pub struct Metadata {
    pub mint: Keypair,
    pub pubkey: Pubkey,
    pub token: Keypair,
}

impl Metadata {
    pub fn new() -> Self {
        let mint = Keypair::new();
        let mint_pubkey = mint.pubkey();
        let program_id = id();

        let metadata_seeds = &[PREFIX.as_bytes(), program_id.as_ref(), mint_pubkey.as_ref()];
        let (pubkey, _) = Pubkey::find_program_address(metadata_seeds, &id());

        Metadata {
            mint,
            pubkey,
            token: Keypair::new()
        }
    }

    pub async fn into_digital_asset(self, context: &mut ProgramTestContext) -> DigitalAsset {
        let token_record = if self.is_pnft(context).await {
            Some(self.token.pubkey())
        } else {
            None
        };

        DigitalAsset {
            metadata: self.pubkey,
            mint: self.mint,
            token: Some(self.token.pubkey()),
            master_edition: None,
            token_record,
        }
    }

    pub async fn get_data(
        &self,
        context: &mut ProgramTestContext,
    ) -> mpl_token_metadata::state::Metadata {
        let account = get_account(context, &self.pubkey).await;
        try_from_slice_unchecked(&account.data).unwrap()
    }

    pub async fn is_pnft(&self, context: &mut ProgramTestContext) -> bool {
        let md = self.get_metadata(context).await;
        if let Some(standard) = md.token_standard {
            if standard == TokenStandard::ProgrammableNonFungible {
                return true;
            }
        }

        false
    }

    pub async fn get_metadata(&self, context: &mut ProgramTestContext) -> TmMetadata {
        let metadata_account = context
            .banks_client
            .get_account(self.pubkey)
            .await
            .unwrap()
            .unwrap();

        TmMetadata::safe_deserialize(&metadata_account.data).unwrap()
    }

    // Boync Edit - we don't need this yet.
    //              create_metadata_accounts is old and deprecated
    // #[allow(deprecated)]
    // pub async fn create(
    //     &self,
    //     context: &mut ProgramTestContext,
    //     name: String,
    //     symbol: String,
    //     uri: String,
    //     creators: Option<Vec<Creator>>,
    //     seller_fee_basis_points: u16,
    //     is_mutable: bool,
    //     decimals: u8,
    // ) -> Result<(), BanksClientError> {
    //     create_mint(
    //         context,
    //         &self.mint,
    //         &context.payer.pubkey(),
    //         Some(&context.payer.pubkey()),
    //         decimals,
    //     )
    //     .await?;
    //     create_token_account(
    //         context,
    //         &self.token,
    //         &self.mint.pubkey(),
    //         &context.payer.pubkey(),
    //     )
    //     .await?;
    //     mint_tokens(
    //         context,
    //         &self.mint.pubkey(),
    //         &self.token.pubkey(),
    //         1,
    //         &context.payer.pubkey(),
    //         None,
    //     )
    //     .await?;

    //     let tx = Transaction::new_signed_with_payer(
    //         &[instruction::create_metadata_accounts(
    //             id(),
    //             self.pubkey,
    //             self.mint.pubkey(),
    //             context.payer.pubkey(),
    //             context.payer.pubkey(),
    //             context.payer.pubkey(),
    //             name,
    //             symbol,
    //             uri,
    //             creators,
    //             seller_fee_basis_points,
    //             false,
    //             is_mutable,
    //         )],
    //         Some(&context.payer.pubkey()),
    //         &[&context.payer],
    //         context.last_blockhash,
    //     );

    //     context.banks_client.process_transaction(tx).await
    // }

    // Boync Edit - we don't need this yet.
    //              create_metadata_accounts_v2 is old and deprecated
    // #[allow(deprecated)]
    // pub async fn create_v2(
    //     &self,
    //     context: &mut ProgramTestContext,
    //     name: String,
    //     symbol: String,
    //     uri: String,
    //     creators: Option<Vec<Creator>>,
    //     seller_fee_basis_points: u16,
    //     is_mutable: bool,
    //     collection: Option<Collection>,
    //     uses: Option<Uses>,
    // ) -> Result<(), BanksClientError> {
    //     create_mint(
    //         context,
    //         &self.mint,
    //         &context.payer.pubkey(),
    //         Some(&context.payer.pubkey()),
    //         0,
    //     )
    //     .await?;
    //     create_token_account(
    //         context,
    //         &self.token,
    //         &self.mint.pubkey(),
    //         &context.payer.pubkey(),
    //     )
    //     .await?;
    //     mint_tokens(
    //         context,
    //         &self.mint.pubkey(),
    //         &self.token.pubkey(),
    //         1,
    //         &context.payer.pubkey(),
    //         None,
    //     )
    //     .await?;

    //     let tx = Transaction::new_signed_with_payer(
    //         &[old_token_metadata::instruction::create_metadata_accounts_v2(
    //             id(),
    //             self.pubkey,
    //             self.mint.pubkey(),
    //             context.payer.pubkey(),
    //             context.payer.pubkey(),
    //             context.payer.pubkey(),
    //             name,
    //             symbol,
    //             uri,
    //             creators,
    //             seller_fee_basis_points,
    //             false,
    //             is_mutable,
    //             collection,
    //             uses,
    //         )],
    //         Some(&context.payer.pubkey()),
    //         &[&context.payer],
    //         context.last_blockhash,
    //     );

    //     context.banks_client.process_transaction(tx).await
    // }

    // Boync Edit - we don't need this yet!
    //              create_metadata_accounts_v2 is old and deprecated
    // pub async fn create_fungible_v2(
    //     &self,
    //     context: &mut ProgramTestContext,
    //     name: String,
    //     symbol: String,
    //     uri: String,
    //     creators: Option<Vec<Creator>>,
    //     seller_fee_basis_points: u16,
    //     is_mutable: bool,
    //     collection: Option<Collection>,
    //     uses: Option<Uses>,
    // ) -> Result<(), BanksClientError> {
    //     create_mint(
    //         context,
    //         &self.mint,
    //         &context.payer.pubkey(),
    //         Some(&context.payer.pubkey()),
    //         0,
    //     )
    //     .await?;
    //     create_token_account(
    //         context,
    //         &self.token,
    //         &self.mint.pubkey(),
    //         &context.payer.pubkey(),
    //     )
    //     .await?;
    //     mint_tokens(
    //         context,
    //         &self.mint.pubkey(),
    //         &self.token.pubkey(),
    //         10,
    //         &context.payer.pubkey(),
    //         None,
    //     )
    //     .await?;

    //     #[allow(deprecated)]
    //     let tx = Transaction::new_signed_with_payer(
    //         &[instruction::create_metadata_accounts_v2(
    //             id(),
    //             self.pubkey,
    //             self.mint.pubkey(),
    //             context.payer.pubkey(),
    //             context.payer.pubkey(),
    //             context.payer.pubkey(),
    //             name,
    //             symbol,
    //             uri,
    //             creators,
    //             seller_fee_basis_points,
    //             false,
    //             is_mutable,
    //             collection,
    //             uses,
    //         )],
    //         Some(&context.payer.pubkey()),
    //         &[&context.payer],
    //         context.last_blockhash,
    //     );

    //     context.banks_client.process_transaction(tx).await
    // }

    // Boync Edit - we don't need this atm.
    // pub async fn create_v2_default(
    //     &self,
    //     context: &mut ProgramTestContext,
    // ) -> Result<(), BanksClientError> {
    //     self.create_v2(
    //         context,
    //         "name".to_string(),
    //         "symbol".to_string(),
    //         "uri".to_string(),
    //         None,
    //         0,
    //         false,
    //         None,
    //         None,
    //     )
    //     .await
    // }

    pub async fn create_v3(
        &self,
        context: &mut ProgramTestContext,
        name: String,
        symbol: String,
        uri: String,
        creators: Option<Vec<Creator>>,
        seller_fee_basis_points: u16,
        is_mutable: bool,
        collection: Option<Collection>,
        uses: Option<Uses>,
        collection_details: Option<CollectionDetails>,
    ) -> StdResult<(), BanksClientError> {
        create_mint(
            context,
            &self.mint,
            &context.payer.pubkey(),
            Some(&context.payer.pubkey()),
            0,
        )
        .await?;
        create_token_account(
            context,
            &self.token,
            &self.mint.pubkey(),
            &context.payer.pubkey(),
        )
        .await?;
        mint_tokens(
            context,
            &self.mint.pubkey(),
            &self.token.pubkey(),
            1,
            &context.payer.pubkey(),
            None,
        )
        .await?;

        let tx = Transaction::new_signed_with_payer(
            &[instruction::create_metadata_accounts_v3(
                id(),
                self.pubkey,
                self.mint.pubkey(),
                context.payer.pubkey(),
                context.payer.pubkey(),
                context.payer.pubkey(),
                name,
                symbol,
                uri,
                creators,
                seller_fee_basis_points,
                false,
                is_mutable,
                collection,
                uses,
                collection_details,
            )],
            Some(&context.payer.pubkey()),
            &[&context.payer],
            context.last_blockhash,
        );

        context.banks_client.process_transaction(tx).await
    }

    pub async fn create_v3_no_freeze_auth(
        &self,
        context: &mut ProgramTestContext,
    ) -> StdResult<(), BanksClientError> {
        let name = String::from("Test");
        let symbol = String::from("TEST");
        let uri = String::from("https://test.com");
        let creators = vec![Creator {
            address: context.payer.pubkey(),
            verified: true,
            share: 100,
        }];
        let sfbp = 100;
        let is_mutable = true;

        // Mint created with no freeze authority set.
        create_mint(context, &self.mint, &context.payer.pubkey(), None, 0).await?;
        create_token_account(
            context,
            &self.token,
            &self.mint.pubkey(),
            &context.payer.pubkey(),
        )
        .await?;
        mint_tokens(
            context,
            &self.mint.pubkey(),
            &self.token.pubkey(),
            1,
            &context.payer.pubkey(),
            None,
        )
        .await?;

        let tx = Transaction::new_signed_with_payer(
            &[instruction::create_metadata_accounts_v3(
                id(),
                self.pubkey,
                self.mint.pubkey(),
                context.payer.pubkey(),
                context.payer.pubkey(),
                context.payer.pubkey(),
                name,
                symbol,
                uri,
                Some(creators),
                sfbp,
                false,
                is_mutable,
                None,
                None,
                None,
            )],
            Some(&context.payer.pubkey()),
            &[&context.payer],
            context.last_blockhash,
        );

        context.banks_client.process_transaction(tx).await
    }

    pub async fn create_v3_default(
        &self,
        context: &mut ProgramTestContext,
    ) -> StdResult<(), BanksClientError> {
        self.create_v3(
            context,
            "name".to_string(),
            "symbol".to_string(),
            "uri".to_string(),
            None,
            0,
            false,
            None,
            None,
            None,
        )
        .await
    }

    pub async fn create_default_nft(
        context: &mut ProgramTestContext,
    ) -> StdResult<(Metadata, MasterEditionV2), BanksClientError> {
        let nft = Metadata::new();
        nft.create_v3(
            context,
            "Test".to_string(),
            "TST".to_string(),
            "uri".to_string(),
            None,
            10,
            false,
            None,
            None,
            None,
        )
        .await
        .unwrap();

        let master_edition = MasterEditionV2::new(&nft);
        master_edition.create_v3(context, Some(0)).await.unwrap();

        Ok((nft, master_edition))
    }

    pub async fn create_default_sized_parent(
        context: &mut ProgramTestContext,
    ) -> StdResult<(Metadata, MasterEditionV2), BanksClientError> {
        let nft = Metadata::new();
        nft.create_v3(
            context,
            "Test".to_string(),
            "TST".to_string(),
            "uri".to_string(),
            None,
            10,
            false,
            None,
            None,
            Some(CollectionDetails::V1 { size: 0 }),
        )
        .await
        .unwrap();

        let master_edition = MasterEditionV2::new(&nft);
        master_edition.create_v3(context, Some(0)).await.unwrap();

        Ok((nft, master_edition))
    }

    
    // pub async fn create_default_unsized_parent(
    //     context: &mut ProgramTestContext,
    // ) -> Result<(Metadata, MasterEditionV2), BanksClientError> {
    //     Self::create_default_nft(context).await
    // }

    // pub async fn create_nft_with_max_supply(
    //     context: &mut ProgramTestContext,
    //     max_supply: u64,
    // ) -> Result<(Metadata, MasterEditionV2), BanksClientError> {
    //     let nft = Metadata::new();
    //     nft.create_v3(
    //         context,
    //         "Test".to_string(),
    //         "TST".to_string(),
    //         "uri".to_string(),
    //         None,
    //         10,
    //         false,
    //         None,
    //         None,
    //         Some(CollectionDetails::V1 { size: 0 }),
    //     )
    //     .await
    //     .unwrap();

    //     let master_edition = MasterEditionV2::new(&nft);
    //     master_edition
    //         .create_v3(context, Some(max_supply))
    //         .await
    //         .unwrap();

    //     Ok((nft, master_edition))
    // }

    pub async fn update_primary_sale_happened_via_token(
        &self,
        context: &mut ProgramTestContext,
    ) -> StdResult<(), BanksClientError> {
        let tx = Transaction::new_signed_with_payer(
            &[instruction::update_primary_sale_happened_via_token(
                id(),
                self.pubkey,
                context.payer.pubkey(),
                self.token.pubkey(),
            )],
            Some(&context.payer.pubkey()),
            &[&context.payer],
            context.last_blockhash,
        );

        context.banks_client.process_transaction(tx).await
    }

    // Boync Edit - we don't need this yet.
    //              update_metadata_accounts is old and deprecated
    // #[allow(deprecated)]
    // pub async fn update(
    //     &self,
    //     context: &mut ProgramTestContext,
    //     name: String,
    //     symbol: String,
    //     uri: String,
    //     creators: Option<Vec<Creator>>,
    //     seller_fee_basis_points: u16,
    // ) -> Result<(), BanksClientError> {
    //     let tx = Transaction::new_signed_with_payer(
    //         &[instruction::update_metadata_accounts(
    //             id(),
    //             self.pubkey,
    //             context.payer.pubkey(),
    //             None,
    //             Some(Data {
    //                 name,
    //                 symbol,
    //                 uri,
    //                 creators,
    //                 seller_fee_basis_points,
    //             }),
    //             None,
    //         )],
    //         Some(&context.payer.pubkey()),
    //         &[&context.payer],
    //         context.last_blockhash,
    //     );

    //     context.banks_client.process_transaction(tx).await
    // }

    pub async fn update_v2(
        &self,
        context: &mut ProgramTestContext,
        name: String,
        symbol: String,
        uri: String,
        creators: Option<Vec<Creator>>,
        seller_fee_basis_points: u16,
        is_mutable: bool,
        collection: Option<Collection>,
        uses: Option<Uses>,
    ) -> StdResult<(), BanksClientError> {
        let tx = Transaction::new_signed_with_payer(
            &[instruction::update_metadata_accounts_v2(
                id(),
                self.pubkey,
                context.payer.pubkey(),
                None,
                Some(DataV2 {
                    name,
                    symbol,
                    uri,
                    creators,
                    seller_fee_basis_points,
                    collection,
                    uses,
                }),
                None,
                Some(is_mutable),
            )],
            Some(&context.payer.pubkey()),
            &[&context.payer],
            context.last_blockhash,
        );

        context.banks_client.process_transaction(tx).await
    }

    pub async fn verify_collection(
        &self,
        context: &mut ProgramTestContext,
        collection: Pubkey,
        collection_authority: &Keypair,
        collection_mint: Pubkey,
        collection_master_edition_account: Pubkey,
        collection_authority_record: Option<Pubkey>,
    ) -> StdResult<(), BanksClientError> {
        let tx = Transaction::new_signed_with_payer(
            &[instruction::verify_collection(
                id(),
                self.pubkey,
                collection_authority.pubkey(),
                context.payer.pubkey(),
                collection_mint,
                collection,
                collection_master_edition_account,
                collection_authority_record,
            )],
            Some(&context.payer.pubkey()),
            &[&context.payer, collection_authority],
            context.last_blockhash,
        );

        context.banks_client.process_transaction(tx).await
    }

    pub async fn verify_sized_collection_item(
        &self,
        context: &mut ProgramTestContext,
        collection: Pubkey,
        collection_authority: &Keypair,
        collection_mint: Pubkey,
        collection_master_edition_account: Pubkey,
        collection_authority_record: Option<Pubkey>,
    ) -> StdResult<(), BanksClientError> {
        let tx = Transaction::new_signed_with_payer(
            &[instruction::verify_sized_collection_item(
                id(),
                self.pubkey,
                collection_authority.pubkey(),
                context.payer.pubkey(),
                collection_mint,
                collection,
                collection_master_edition_account,
                collection_authority_record,
            )],
            Some(&context.payer.pubkey()),
            &[&context.payer, collection_authority],
            context.last_blockhash,
        );

        context.banks_client.process_transaction(tx).await
    }

    pub async fn set_and_verify_collection(
        &self,
        context: &mut ProgramTestContext,
        collection: Pubkey,
        collection_authority: &Keypair,
        nft_update_authority: Pubkey,
        collection_mint: Pubkey,
        collection_master_edition_account: Pubkey,
        collection_authority_record: Option<Pubkey>,
    ) -> StdResult<(), BanksClientError> {
        let tx = Transaction::new_signed_with_payer(
            &[instruction::set_and_verify_collection(
                id(),
                self.pubkey,
                collection_authority.pubkey(),
                context.payer.pubkey(),
                nft_update_authority,
                collection_mint,
                collection,
                collection_master_edition_account,
                collection_authority_record,
            )],
            Some(&context.payer.pubkey()),
            &[&context.payer, collection_authority],
            context.last_blockhash,
        );
        context.banks_client.process_transaction(tx).await
    }

    pub async fn set_and_verify_sized_collection_item(
        &self,
        context: &mut ProgramTestContext,
        collection: Pubkey,
        collection_authority: &Keypair,
        nft_update_authority: Pubkey,
        collection_mint: Pubkey,
        collection_master_edition_account: Pubkey,
        collection_authority_record: Option<Pubkey>,
    ) -> StdResult<(), BanksClientError> {
        let tx = Transaction::new_signed_with_payer(
            &[instruction::set_and_verify_sized_collection_item(
                id(),
                self.pubkey,
                collection_authority.pubkey(),
                context.payer.pubkey(),
                nft_update_authority,
                collection_mint,
                collection,
                collection_master_edition_account,
                collection_authority_record,
            )],
            Some(&context.payer.pubkey()),
            &[&context.payer, collection_authority],
            context.last_blockhash,
        );
        context.banks_client.process_transaction(tx).await
    }

    pub async fn unverify_collection(
        &self,
        context: &mut ProgramTestContext,
        collection: Pubkey,
        collection_authority: &Keypair,
        collection_mint: Pubkey,
        collection_master_edition_account: Pubkey,
        collection_authority_record: Option<Pubkey>,
    ) -> StdResult<(), BanksClientError> {
        let tx = Transaction::new_signed_with_payer(
            &[instruction::unverify_collection(
                id(),
                self.pubkey,
                collection_authority.pubkey(),
                collection_mint,
                collection,
                collection_master_edition_account,
                collection_authority_record,
            )],
            Some(&context.payer.pubkey()),
            &[&context.payer, collection_authority],
            context.last_blockhash,
        );

        context.banks_client.process_transaction(tx).await
    }

    pub async fn unverify_sized_collection_item(
        &self,
        context: &mut ProgramTestContext,
        collection: Pubkey,
        collection_authority: &Keypair,
        collection_mint: Pubkey,
        collection_master_edition_account: Pubkey,
        collection_authority_record: Option<Pubkey>,
    ) -> StdResult<(), BanksClientError> {
        let tx = Transaction::new_signed_with_payer(
            &[instruction::unverify_sized_collection_item(
                id(),
                self.pubkey,
                collection_authority.pubkey(),
                context.payer.pubkey(),
                collection_mint,
                collection,
                collection_master_edition_account,
                collection_authority_record,
            )],
            Some(&context.payer.pubkey()),
            &[&context.payer, collection_authority],
            context.last_blockhash,
        );

        context.banks_client.process_transaction(tx).await
    }

    pub async fn change_update_authority(
        &self,
        context: &mut ProgramTestContext,
        new_update_authority: Pubkey,
    ) -> StdResult<(), BanksClientError> {
        airdrop(context, &new_update_authority, 1_000_000_000)
            .await
            .unwrap();

        let tx = Transaction::new_signed_with_payer(
            &[instruction::update_metadata_accounts_v2(
                mpl_token_metadata::id(),
                self.pubkey,
                context.payer.pubkey(),
                Some(new_update_authority),
                None,
                None,
                None,
            )],
            Some(&context.payer.pubkey()),
            &[&context.payer],
            context.last_blockhash,
        );

        context.banks_client.process_transaction(tx).await
    }
}

impl Default for Metadata {
    fn default() -> Self {
        Self::new()
    }
}

pub async fn assert_collection_size(
    context: &mut ProgramTestContext,
    collection_metadata: &Metadata,
    size: u64,
) {
    let collection_md = collection_metadata.get_data(context).await;
    let retrieved_size = if let Some(details) = collection_md.collection_details {
        match details {
            CollectionDetails::V1 { size } => size,
        }
    } else {
        panic!("Expected CollectionDetails::V1");
    };
    assert_eq!(retrieved_size, size);
}
