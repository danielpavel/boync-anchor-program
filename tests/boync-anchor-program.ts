import * as anchor from "@project-serum/anchor";
import { Program } from "@project-serum/anchor";
// import { Keypair, PublicKey, LAMPORTS_PER_SOL } from '@solana/web3.js';
import { BoyncAnchorProgram } from "../target/types/boync_anchor_program";
import * as spl from '@solana/spl-token';
import {
  createMintToCheckedInstruction,
} from "@solana/spl-token";
import { assert, expect } from 'chai';
import * as _ from 'lodash'
import { LAMPORTS_PER_SOL, Transaction } from "@solana/web3.js";

const TREASURY_SEED = anchor.utils.bytes.utf8.encode("treasury");
const AUCTION_SEED = anchor.utils.bytes.utf8.encode("auction");
const CHEST_WALLET_SEED = anchor.utils.bytes.utf8.encode("wallet");
const BIDDER_SEED = anchor.utils.bytes.utf8.encode("bidder");

const HALF_HOUR_IN_MS = (30 * 60 * 1000);

interface PDAParams {
  treasuryBump:       number,
  biddersChestBump:   number,
  auctionStateBump:   number,
  idx:                anchor.BN,
  treasuryWalletKey:  anchor.web3.PublicKey,
  biddersChestWalletKey:    anchor.web3.PublicKey,
  auctionStateKey:    anchor.web3.PublicKey,
}

const createMint = async (
  provider: anchor.Provider,
  payer: anchor.web3.Keypair
): Promise<anchor.web3.PublicKey | undefined> => {
  const tokenMint = new anchor.web3.Keypair();
  const lamportsForMint =
    await provider.connection.getMinimumBalanceForRentExemption(
      spl.MintLayout.span
    );

  let tx = new anchor.web3.Transaction()
    .add(
      // Create the mint account
      anchor.web3.SystemProgram.createAccount({
        programId: spl.TOKEN_PROGRAM_ID,
        space: spl.MintLayout.span,
        fromPubkey: payer.publicKey,
        newAccountPubkey: tokenMint.publicKey,
        lamports: lamportsForMint,
      })
    )
    // Init the mint account
    .add(
      spl.createInitializeMintInstruction(
        tokenMint.publicKey,
        0,
        payer.publicKey,
        payer.publicKey,
        spl.TOKEN_PROGRAM_ID
      )
    );

  try {
    await provider.sendAndConfirm(tx, [tokenMint]);

    return tokenMint.publicKey;
  } catch (error) {
    return undefined;
  }
};

const aidrop = async (
  provider: anchor.Provider,
  payer: anchor.web3.Keypair,
  pubkey: anchor.web3.PublicKey,
  amount: number
) => {
  const txFund = new anchor.web3.Transaction().add(
    anchor.web3.SystemProgram.transfer({
      fromPubkey: payer.publicKey,
      toPubkey: pubkey,
      lamports: amount * anchor.web3.LAMPORTS_PER_SOL,
    })
  );

  await provider.sendAndConfirm(txFund);
};

const createAssociatedWallet = async (
  provider: anchor.Provider,
  mintAuth: anchor.web3.PublicKey,
  user: anchor.web3.Keypair,
  mint?: anchor.web3.PublicKey
): Promise<anchor.web3.PublicKey | undefined> => {

  if (mint) {
    // Create a token account for the user and mint some tokens
    const userATA = await spl.getOrCreateAssociatedTokenAccount(
      provider.connection, // connection
      user, // fee payer
      mint, // mint
      user.publicKey, // owner,
      false
    );

    if (_.isEmpty(userATA)) {
      console.log("[BoyncDebug] Failed to create userATA!");
      return undefined;
    } else {
      // console.log(
      //   `[${userATA.address.toString()}] New associated token account for mint ${mint.toBase58()}`
      // );
    }

    // Mint some tokens
    let tx = new Transaction().add(
      createMintToCheckedInstruction(
        mint, // mint
        userATA.address, // receiver (sholud be a token account)
        mintAuth, // mint authority
        1, // amount. if your decimals is 8, you mint 10^8 for 1 token.
        0 // decimals
        // [signer1, signer2 ...], // only multisig account will use
      )
    );

    await provider.sendAndConfirm(tx);

    // console.log(
    //   "[BoyncDebug] Minted 1 token for userATA",
    //   userATA.address.toString()
    // );

    return userATA.address;
  }

  return undefined;
};

const buildRequiredPDAs = async (
  program: Program<BoyncAnchorProgram>,
  auctionCreator: anchor.web3.PublicKey,
  treasuryMint: anchor.web3.PublicKey,
): Promise<PDAParams> => {

  const nowInSec = Math.floor(Date.now());
  let uid = new anchor.BN(parseInt((nowInSec + HALF_HOUR_IN_MS).toString()));
  const uidBuffer = uid.toArrayLike(Buffer, "le", 8);

  let [auctionStatePubKey, auctionStateBump] =
    await anchor.web3.PublicKey.findProgramAddress(
      [
        AUCTION_SEED,
        auctionCreator.toBuffer(),
        treasuryMint.toBuffer(),
        uidBuffer,
      ],
      program.programId
    );

  let [treasuryWalletPubKey, treasuryWalletBump] =
    await anchor.web3.PublicKey.findProgramAddress(
      [
        TREASURY_SEED,
        auctionCreator.toBuffer(),
        treasuryMint.toBuffer(),
        uidBuffer,
      ],
      program.programId
    );

  let [biddersChestWalletPubKey, biddersChestWalletBump] =
    await anchor.web3.PublicKey.findProgramAddress(
      [
        CHEST_WALLET_SEED,
        auctionCreator.toBuffer(),
        // chestMint.toBuffer(),
        uidBuffer,
      ],
      program.programId
    );

  return {
    idx: uid,
    treasuryBump: treasuryWalletBump,
    treasuryWalletKey: treasuryWalletPubKey,
    auctionStateKey: auctionStatePubKey,
    auctionStateBump: auctionStateBump,
    biddersChestWalletKey: biddersChestWalletPubKey,
    biddersChestBump: biddersChestWalletBump,
  };
};


describe("Boync Auction Init Tests", () => {
  anchor.setProvider(anchor.AnchorProvider.env());
  const program = anchor.workspace.BoyncAnchorProgram as Program<BoyncAnchorProgram>;
  const provider = anchor.getProvider()
  const wallet = anchor.Wallet.local()

  let treasuryMint: anchor.web3.PublicKey;
  let auctionCreator = new anchor.web3.Keypair();
  let auctionCreatorAssocWallet: anchor.web3.PublicKey | undefined;

  let pdas: PDAParams;

  beforeEach(async () => {
    /* Create Mint Account for a new SPL token */
    treasuryMint = await createMint(provider, wallet.payer);

    /* Airdrop auctionCreator some SOL to cover future transactions fees */
    await aidrop(provider, wallet.payer, auctionCreator.publicKey, 5);

    /**
     * In order for auctionCreator to create / initialize a Boync Auction we need to have
     * create an ATA (associated token account) for auctionCreator and mint some SPL tokens in that account.
     */
    auctionCreatorAssocWallet = await createAssociatedWallet(
      provider,
      wallet.payer.publicKey,
      auctionCreator,
      treasuryMint
    );

    /* build other needed PDAs (program derrived addresses) */
    pdas = await buildRequiredPDAs(program, auctionCreator.publicKey, treasuryMint)

    /**
     * At this point auctionCreator is ready to create a Boync Auction, phew!
     */
  })

  it("Auction successfully initialized", async () => {
    let auctionCreatorTokenAccountBalancePre =
      await provider.connection.getTokenAccountBalance(auctionCreatorAssocWallet);
    assert.equal(auctionCreatorTokenAccountBalancePre.value.amount, "1");

    await program.methods
      .initialize(pdas.idx, pdas.auctionStateBump)
      .accounts({
        state: pdas.auctionStateKey,
        treasury: pdas.treasuryWalletKey,
        biddersChest: pdas.biddersChestWalletKey,
        signer: auctionCreator.publicKey,
        treasuryMint: treasuryMint,
        signerWithdrawWallet: auctionCreatorAssocWallet,

        systemProgram: anchor.web3.SystemProgram.programId,
        rent: anchor.web3.SYSVAR_RENT_PUBKEY,
        tokenProgram: spl.TOKEN_PROGRAM_ID,
      })
      .signers([auctionCreator])
      .rpc();

    let auctionCreatorTokenAccountBalancePost =
      await provider.connection.getTokenAccountBalance(auctionCreatorAssocWallet);
    assert.equal(auctionCreatorTokenAccountBalancePost.value.amount, "0");

    let treasuryAccountBalance =
      await provider.connection.getTokenAccountBalance(pdas.treasuryWalletKey);
    assert.equal(treasuryAccountBalance.value.amount, "1");

    const state = await program.account.boyncAuction2.fetch(
      pdas.auctionStateKey
    );

    assert.equal(Object.keys(state.state)[0], 'created');
  })
})

describe("Boync Auction Place Bid Tests", () => {
  anchor.setProvider(anchor.AnchorProvider.env());
  const program = anchor.workspace.BoyncAnchorProgram as Program<BoyncAnchorProgram>;
  const provider = anchor.getProvider()
  const wallet = anchor.Wallet.local()

  let treasuryMint: anchor.web3.PublicKey;
  let auctionCreator = new anchor.web3.Keypair();
  let auctionCreatorAssocWallet: anchor.web3.PublicKey | undefined;

  const bidder = new anchor.web3.Keypair();
  const bidderTs: number = Date.now();
  let bidderPDA: anchor.web3.PublicKey;
  let bidderPDABump: number;

  let pdas: PDAParams;

  beforeEach(async () => {
    /* Create Mint Account for a new SPL token */
    treasuryMint = await createMint(provider, wallet.payer);

    /* Airdrop auctionCreator some SOL to cover future transactions fees */
    await aidrop(provider, wallet.payer, auctionCreator.publicKey, 5);
    /* Airdrop bidder some SOL */
    await aidrop(provider, wallet.payer, bidder.publicKey, 4);

    /**
     * In order for auctionCreator to create / initialize a Boync Auction we need to have
     * create an ATA (associated token account) for auctionCreator and mint some SPL tokens in that account.
     */
    auctionCreatorAssocWallet = await createAssociatedWallet(
      provider,
      wallet.payer.publicKey,
      auctionCreator,
      treasuryMint
    );

    /* build other needed PDAs (program derrived addresses) */
    pdas = await buildRequiredPDAs(program, auctionCreator.publicKey, treasuryMint)

    const uidBuffer = new anchor.BN(bidderTs).toArrayLike(Buffer, "le", 8);
    const [pda, bump] =
      await anchor.web3.PublicKey.findProgramAddress(
        [
          BIDDER_SEED,
          pdas.auctionStateKey.toBuffer(),
          bidder.publicKey.toBuffer(),
          uidBuffer,
        ],
        program.programId
      );

    bidderPDA = pda;
    bidderPDABump = bump;

    /**
     * At this point auctionCreator is ready to create a Boync Auction, phew!
     */
    await program.methods
      .initialize(pdas.idx, pdas.auctionStateBump)
      .accounts({
        state: pdas.auctionStateKey,
        treasury: pdas.treasuryWalletKey,
        biddersChest: pdas.biddersChestWalletKey,
        signer: auctionCreator.publicKey,
        treasuryMint: treasuryMint,
        signerWithdrawWallet: auctionCreatorAssocWallet,

        systemProgram: anchor.web3.SystemProgram.programId,
        rent: anchor.web3.SYSVAR_RENT_PUBKEY,
        tokenProgram: spl.TOKEN_PROGRAM_ID,
      })
      .signers([auctionCreator])
      .rpc();

    /* Start the auction manually before anyone can bid */
    await program.methods
      .start()
      .accounts({
        auction: pdas.auctionStateKey,
        authority: auctionCreator.publicKey,
      })
      .signers([auctionCreator])
      .rpc();
  });

  it('Simple bid test', async () => {
    await program.methods
      .bid(new anchor.BN(bidderTs))
      .accounts({
          state: pdas.auctionStateKey,
          biddersChest: pdas.biddersChestWalletKey,
          bidderState: bidderPDA,
          bidder: bidder.publicKey,

          systemProgram: anchor.web3.SystemProgram.programId,
          rent: anchor.web3.SYSVAR_RENT_PUBKEY,
      })
      .signers([bidder])
      .rpc();

    /* Check the bidder chest */
    let bidderBalance =
      await provider.connection.getBalance(bidder.publicKey);
    // assert.equal(, 3.9);
    expect((bidderBalance / LAMPORTS_PER_SOL) > 3.89 && (bidderBalance / LAMPORTS_PER_SOL) < 3.9);

    /* Check the chest that collects bidder fees */
    let biddersChestBalance =
      await provider.connection.getBalance(pdas.biddersChestWalletKey);
    assert.equal((biddersChestBalance / LAMPORTS_PER_SOL), 0.1);

    const bidderPDAs = await program.account.boyncUserBid.all()

    for (const bidderAccount of bidderPDAs) {
      assert.equal(bidderAccount.account.auction.toString(), pdas.auctionStateKey.toString());
      assert.equal(bidderAccount.account.bidder.toString(), bidder.publicKey.toString());
      assert.equal(bidderAccount.account.ts.toNumber(), bidderTs);
    }
  })

});
