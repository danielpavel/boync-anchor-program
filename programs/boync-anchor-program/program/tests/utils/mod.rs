mod digital_asset;
mod metadata;
mod master_edition_v2;
mod setup_functions;
mod constants;

pub use constants::*;
pub use digital_asset::*;
pub use setup_functions::*;
pub use metadata::{assert_collection_size, Metadata};
pub use master_edition_v2::MasterEditionV2;
pub use mpl_token_metadata::instruction;
use mpl_token_metadata::state::CollectionDetails;
use solana_program_test::*;
use solana_sdk::{
    account::Account, program_pack::Pack, pubkey::Pubkey, signature::Signer,
    signer::keypair::Keypair, system_instruction, transaction::Transaction,

};

use anchor_spl::token::spl_token;

pub const DEFAULT_COLLECTION_DETAILS: Option<CollectionDetails> =
    Some(CollectionDetails::V1 { size: 0 });

pub fn program_test() -> ProgramTest {
    let mut program = ProgramTest::new("boync_anchor_program", boync_anchor_program::id(), None);
    program.set_compute_max_units(1_200_000);

    program.add_program("mpl_token_metadata", mpl_token_metadata::id(), None);

    program
}

pub async fn get_account(context: &mut ProgramTestContext, pubkey: &Pubkey) -> Account {
    context
        .banks_client
        .get_account(*pubkey)
        .await
        .expect("account not found")
        .expect("account empty")
}

// Boync Edit - we don't need this yet.
//
// pub async fn get_mint(context: &mut ProgramTestContext, pubkey: &Pubkey) -> Mint {
//     let account = get_account(context, pubkey).await;
//     Mint::unpack(&account.data).unwrap()
// }

pub async fn airdrop(
    context: &mut ProgramTestContext,
    receiver: &Pubkey,
    amount: u64,
) -> Result<(), BanksClientError> {
    let tx = Transaction::new_signed_with_payer(
        &[system_instruction::transfer(
            &context.payer.pubkey(),
            receiver,
            amount,
        )],
        Some(&context.payer.pubkey()),
        &[&context.payer],
        context.last_blockhash,
    );

    context.banks_client.process_transaction(tx).await.unwrap();
    Ok(())
}

pub async fn burn(
    context: &mut ProgramTestContext,
    metadata: Pubkey,
    owner: &Keypair,
    mint: Pubkey,
    token: Pubkey,
    edition: Pubkey,
    collection_metadata: Option<Pubkey>,
) -> Result<(), BanksClientError> {
    let tx = Transaction::new_signed_with_payer(
        &[instruction::burn_nft(
            mpl_token_metadata::ID,
            metadata,
            owner.pubkey(),
            mint,
            token,
            edition,
            spl_token::ID,
            collection_metadata,
        )],
        Some(&owner.pubkey()),
        &[owner],
        context.last_blockhash,
    );

    context.banks_client.process_transaction(tx).await?;

    Ok(())
}

pub async fn burn_edition(
    context: &mut ProgramTestContext,
    metadata: Pubkey,
    owner: &Keypair,
    print_edition_mint: Pubkey,
    master_edition_mint: Pubkey,
    print_edition_token: Pubkey,
    master_edition_token: Pubkey,
    master_edition: Pubkey,
    print_edition: Pubkey,
    edition_marker: Pubkey,
) -> Result<(), BanksClientError> {
    let tx = Transaction::new_signed_with_payer(
        &[instruction::burn_edition_nft(
            mpl_token_metadata::ID,
            metadata,
            owner.pubkey(),
            print_edition_mint,
            master_edition_mint,
            print_edition_token,
            master_edition_token,
            master_edition,
            print_edition,
            edition_marker,
            spl_token::ID,
        )],
        Some(&owner.pubkey()),
        &[owner],
        context.last_blockhash,
    );

    context.banks_client.process_transaction(tx).await?;

    Ok(())
}

pub async fn mint_tokens(
    context: &mut ProgramTestContext,
    mint: &Pubkey,
    account: &Pubkey,
    amount: u64,
    owner: &Pubkey,
    additional_signer: Option<&Keypair>,
) -> Result<(), BanksClientError> {
    let mut signing_keypairs = vec![&context.payer];
    if let Some(signer) = additional_signer {
        signing_keypairs.push(signer);
    }

    let tx = Transaction::new_signed_with_payer(
        &[
            spl_token::instruction::mint_to(&spl_token::id(), mint, account, owner, &[], amount)
                .unwrap(),
        ],
        Some(&context.payer.pubkey()),
        &signing_keypairs,
        context.last_blockhash,
    );

    context.banks_client.process_transaction(tx).await
}

pub async fn create_token_account(
    context: &mut ProgramTestContext,
    account: &Keypair,
    mint: &Pubkey,
    manager: &Pubkey,
) -> Result<(), BanksClientError> {
    let rent = context.banks_client.get_rent().await.unwrap();

    let tx = Transaction::new_signed_with_payer(
        &[
            system_instruction::create_account(
                &context.payer.pubkey(),
                &account.pubkey(),
                rent.minimum_balance(spl_token::state::Account::LEN),
                spl_token::state::Account::LEN as u64,
                &spl_token::id(),
            ),
            spl_token::instruction::initialize_account(
                &spl_token::id(),
                &account.pubkey(),
                mint,
                manager,
            )
            .unwrap(),
        ],
        Some(&context.payer.pubkey()),
        &[&context.payer, account],
        context.last_blockhash,
    );

    context.banks_client.process_transaction(tx).await
}

pub async fn create_mint(
    context: &mut ProgramTestContext,
    mint: &Keypair,
    manager: &Pubkey,
    freeze_authority: Option<&Pubkey>,
    decimals: u8,
) -> Result<(), BanksClientError> {
    let rent = context.banks_client.get_rent().await.unwrap();

    let tx = Transaction::new_signed_with_payer(
        &[
            system_instruction::create_account(
                &context.payer.pubkey(),
                &mint.pubkey(),
                rent.minimum_balance(spl_token::state::Mint::LEN),
                spl_token::state::Mint::LEN as u64,
                &spl_token::id(),
            ),
            spl_token::instruction::initialize_mint(
                &spl_token::id(),
                &mint.pubkey(),
                manager,
                freeze_authority,
                decimals,
            )
            .unwrap(),
        ],
        Some(&context.payer.pubkey()),
        &[&context.payer, mint],
        context.last_blockhash,
    );

    context.banks_client.process_transaction(tx).await
}

pub trait DirtyClone {
    fn dirty_clone(&self) -> Self;
}

impl DirtyClone for Keypair {
    fn dirty_clone(&self) -> Self {
        Keypair::from_bytes(&self.to_bytes()).unwrap()
    }
}
