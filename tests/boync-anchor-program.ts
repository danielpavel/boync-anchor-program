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
import { Keypair, LAMPORTS_PER_SOL, PublicKey, SystemProgram, Transaction } from "@solana/web3.js";

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
  const localFp = new anchor.BN(3 * LAMPORTS_PER_SOL);

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
      .initialize(pdas.idx, pdas.auctionStateBump, localFp)
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
    assert.equal(state.startingPrice.toNumber(), 0.05 * localFp.toNumber());
    assert.equal(state.nextBid.toNumber(), 0.05 * localFp.toNumber());
    assert.equal(state.lastBidder.toString(), SystemProgram.programId.toString());
    assert.equal(state.id.toNumber(), pdas.idx.toNumber());
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

  const localFp = new anchor.BN(3 * LAMPORTS_PER_SOL);

  const bidder = new anchor.web3.Keypair();
  const bidderTs: number = Date.now();
  let bidderPDA: anchor.web3.PublicKey;
  let bidderPDABump: number;
  const bidder2 = new anchor.web3.Keypair();
  const bidderTs2: number = Date.now() + (1000);
  let bidder2PDA: anchor.web3.PublicKey;
  let bidder2PDABump: number;

  let pdas: PDAParams;
  const treasury = new anchor.web3.Keypair();

  const placeBid = async (
    ts: anchor.BN,
    bidder: Keypair,
    bidderPDA: PublicKey
  ) => {
    return program.methods
      .bid(ts)
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
  };

  beforeEach(async () => {
    /* Create Mint Account for a new SPL token */
    treasuryMint = await createMint(provider, wallet.payer);

    /* Airdrop auctionCreator some SOL to cover future transactions fees */
    await aidrop(provider, wallet.payer, auctionCreator.publicKey, 5);
    /* Airdrop bidders some SOL */
    await aidrop(provider, wallet.payer, bidder.publicKey, 4);
    await aidrop(provider, wallet.payer, bidder2.publicKey, 5);

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

    /* build PDA for first bidder */
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

    /* build PDA for second bidder */
    const uidBuffer2 = new anchor.BN(bidderTs2).toArrayLike(Buffer, "le", 8);
    const [pda2, bump2] =
      await anchor.web3.PublicKey.findProgramAddress(
        [
          BIDDER_SEED,
          pdas.auctionStateKey.toBuffer(),
          bidder2.publicKey.toBuffer(),
          uidBuffer2,
        ],
        program.programId
      );

    bidder2PDA = pda2;
    bidder2PDABump = bump2;

    /**
     * At this point auctionCreator is ready to create a Boync Auction, phew!
     */
    await program.methods
      .initialize(pdas.idx, pdas.auctionStateBump, localFp)
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

  it('Simple bid test with 2 bidders', async () => {
    /* Place first bid */
    await placeBid(new anchor.BN(bidderTs), bidder, bidderPDA);

    /* Check the bidder chest */
    let bidderBalance =
      await provider.connection.getBalance(bidder.publicKey);
    expect((bidderBalance / LAMPORTS_PER_SOL) > 3.83 && (bidderBalance / LAMPORTS_PER_SOL) < 3.84);

    /* Check that chest collects bidder fees */
    let biddersChestBalance =
      await provider.connection.getBalance(pdas.biddersChestWalletKey);
    assert.equal((biddersChestBalance), (0.05 * localFp.toNumber()));

    const bidderPDAState = await program.account.boyncUserBid.fetch(bidderPDA)
    assert.equal(bidderPDAState.auction.toString(), pdas.auctionStateKey.toString());
    assert.equal(bidderPDAState.bidder.toString(), bidder.publicKey.toString());
    assert.equal(bidderPDAState.bidValue.toNumber(), 0.15 * LAMPORTS_PER_SOL);
    assert.equal(bidderPDAState.ts.toNumber(), bidderTs);

    let state = await program.account.boyncAuction2.fetch(
      pdas.auctionStateKey
    );
    assert.equal(state.nextBid.toNumber(), 1.05 * (0.05 * localFp.toNumber()));
    assert.equal(state.lastBidder.toString(), bidder.publicKey.toString());

    /* Place second bid */
    await placeBid(new anchor.BN(bidderTs2), bidder2, bidder2PDA);
    biddersChestBalance = await provider.connection.getBalance(pdas.biddersChestWalletKey);
    assert.equal((biddersChestBalance), (0.05 * localFp.toNumber()) + (1.05 * (0.05 * localFp.toNumber())));

    /* End the auction */
    await program.methods
      .end(pdas.biddersChestBump)
      .accounts({
        state: pdas.auctionStateKey,
        biddersChest: pdas.biddersChestWalletKey,
        authority: auctionCreator.publicKey,
        treasury: treasury.publicKey
      })
      .signers([auctionCreator])
      .rpc();

    state = await program.account.boyncAuction2.fetch(
      pdas.auctionStateKey
    );
    assert.equal(Object.keys(state.state)[0], 'ended');
    assert.equal(state.lastBidder.toString(), bidder2.publicKey.toString());

    /* Check that collector chest is empty */
    biddersChestBalance =
      await provider.connection.getBalance(pdas.biddersChestWalletKey);
    // console.log('[bidders Chest] balance', biddersChestBalance / LAMPORTS_PER_SOL);
    assert.equal((biddersChestBalance / LAMPORTS_PER_SOL), 0);

    /* Check auctionCreator balance was added fees */
    let auctionCreatorBalance =
      await provider.connection.getBalance(auctionCreator.publicKey);
    // console.log('[auction creator] balance', auctionCreatorBalance / LAMPORTS_PER_SOL);
    // !!! This assert amounts o a little over what we've calculated due to fees deducted when creating auction.
    // !!! We can't guess exactly that number but from logging it, it looks OK!
    // assert.equal((auctionCreatorBalance), (5 + 0.230625) * LAMPORTS_PER_SOL); //75% of total fees which was calculated 0.75 * ((0.05 * fp) + (1.05 * (0.05 * fp)))

    /* Check treasury balance was added fees */
    let treasuryBalance =
      await provider.connection.getBalance(treasury.publicKey);
    // console.log('[treasury] balance', treasuryBalance / LAMPORTS_PER_SOL);
    assert.equal((treasuryBalance), 0.076875 * LAMPORTS_PER_SOL); //25% of total fees which was calculated 0.25 * ((0.05 * fp) + (1.05 * (0.05 * fp)))
  })

});
