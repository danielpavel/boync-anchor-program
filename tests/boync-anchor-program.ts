import * as anchor from "@project-serum/anchor";
import { Program } from "@project-serum/anchor";
// import { Keypair, PublicKey, LAMPORTS_PER_SOL } from '@solana/web3.js';
import { BoyncAnchorProgram } from "../target/types/boync_anchor_program";
import * as spl from '@solana/spl-token';
import { assert, expect } from 'chai';

const TREASURY_SEED = anchor.utils.bytes.utf8.encode("treasury");
const AUCTION_SEED = anchor.utils.bytes.utf8.encode("auction");
const CHEST_WALLET_SEED = anchor.utils.bytes.utf8.encode("wallet");

const HALF_HOUR = (30 * 60);

/* Scraped from https://stackoverflow.com/questions/45466040/verify-that-an-exception-is-thrown-using-mocha-chai-and-async-await */
const expectThrowsAsync = async (method, errorMessage?) => {
  let error = null
  try {
    await method()
  }
  catch (err) {
    error = err
  }
  expect(error).to.be.an('Error')
  if (errorMessage) {
    expect(error.message).to.equal(errorMessage)
  }
}

interface PDAParams {
  treasuryBump:       number,
  biddersChestBump:   number,
  auctionStateBump:   number,
  idx:                anchor.BN,
  treasuryWalletKey:  anchor.web3.PublicKey,
  biddersChestWalletKey:    anchor.web3.PublicKey,
  auctionStateKey:    anchor.web3.PublicKey,
}

console.log('[new program keypair]:', new anchor.web3.Keypair().publicKey.toBase58())

describe("Boync Auction Tests", () => {
  // Configure the client to use the local cluster.
  anchor.setProvider(anchor.AnchorProvider.env());
  

  const program = anchor.workspace.BoyncAnchorProgram as Program<BoyncAnchorProgram>;
  const _wallet = anchor.Wallet.local()
  const provider = anchor.getProvider()

  let treasuryMintAddress: anchor.web3.PublicKey;
  let boyncTokenMintAddress: anchor.web3.PublicKey;
  let bidder1: anchor.web3.Keypair;
  let bidder1ATA: anchor.web3.PublicKey;
  let auctionCreator: anchor.web3.Keypair;
  let auctionCreatorAssocWallet: anchor.web3.PublicKey;
  let user2: anchor.web3.Keypair;
  let user2AssocWallet: anchor.web3.PublicKey;
  let _pda: PDAParams;


  const createUserAndAssociatedWallet = async (mint?: anchor.web3.PublicKey): 
    Promise<[anchor.web3.Keypair, anchor.web3.PublicKey | undefined]> => {

    const user = new anchor.web3.Keypair();
    let userATA: anchor.web3.PublicKey | undefined = undefined;

    // Fund user with some SOL
    let txFund = new anchor.web3.Transaction().add(
      anchor.web3.SystemProgram.transfer({
        fromPubkey: _wallet.publicKey,
        toPubkey: user.publicKey,
        lamports: 5 * anchor.web3.LAMPORTS_PER_SOL,
      })
    );
    const sigTxFund = await provider.sendAndConfirm(txFund);
    // console.log(`[${user.publicKey.toBase58()}] Funded new account with 5 SOL: ${sigTxFund}`);

    if (mint) {
      // Create a token account for the user and mint some tokens
      userATA = await spl.createAssociatedTokenAccount(
        provider.connection, // connection
        user, // fee payer
        mint, // mint
        user.publicKey // owner,
      );

      // if (typeof userATA === "undefined" || userATA === null) {
      //   console.log("[BoyncDebug] Failed to create userATA");
      // } else {
      //   console.log(
      //     `[${userATA.toBase58()}] New associated account for mint ${mint.toBase58()}: ${userATA}`
      //   );
      // }

      // mint some tokens
      // BROKEN!!! FIX in [BA-Program-vLPY3BPX]
      // let txhash = await spl.mintToChecked(
      //   provider.connection, // connection
      //   user, // fee payer
      //   mint, // mint
      //   userATA, // receiver (sholud be a token account)
      //   provider.wallet.publicKey, // mint authority
      //   100e6, // amount. if your decimals is 6, you mint 10^6 for 1 token.
      //   6 // decimals
      // );

      // if (typeof txhash === 'undefined' || txhash === null) {
      //   console.log('[BoyncDebug] Txhash is undefined | null');
      // } else {
      //   console.log('[BoyncDebug] tx:', txhash);
      // }

      let tx = new anchor.web3.Transaction().add(
        spl.createMintToCheckedInstruction(
          mint, // mint
          userATA, // receiver (sholud be a token account)
          _wallet.publicKey, // mint authority
          100e6, // amount. if your decimals is 6, you mint 10^6 for 1 token.
          6 // decimals
        )
      );

      // TODO: Do we need this?
      // provider.wallet.signTransaction(tx);
      // TODO: Why don't I need singers?
      await provider.sendAndConfirm(tx);
      // console.log("[BoyncDebug] Minted 100 tokens for userATA", userATA.toBase58());
    }

    return [user, userATA];
  }

  const createMint = async (
    connection: anchor.web3.Connection
  ): Promise<anchor.web3.PublicKey> => {
    const tokenMint = new anchor.web3.Keypair();
    const lamportsForMint =
      await provider.connection.getMinimumBalanceForRentExemption(
        spl.MintLayout.span
      );

    let tx = new anchor.web3.Transaction().add(
      // Create the mint account
      anchor.web3.SystemProgram.createAccount({
        programId: spl.TOKEN_PROGRAM_ID,
        space: spl.MintLayout.span,
        fromPubkey: _wallet.publicKey,
        newAccountPubkey: tokenMint.publicKey,
        lamports: lamportsForMint,
      }))
      // Init the mint account
      .add(
        spl.createInitializeMintInstruction(
          tokenMint.publicKey,
          6,
          _wallet.publicKey,
          _wallet.publicKey,
          spl.TOKEN_PROGRAM_ID,
        )
      );

    const signature = await provider.sendAndConfirm(tx, [tokenMint]);

    // console.log(
    //   `[${tokenMint.publicKey}] Created new mint account at ${signature}`
    // );

    return tokenMint.publicKey;
  };

  const getPDAParams = async (
    auctionCreator: anchor.web3.PublicKey,
    treasuryMint: anchor.web3.PublicKey,
    chestMint: anchor.web3.PublicKey
  ): Promise<PDAParams> => {
    const nowInSec = Math.floor(Date.now() / 1000);
    let uid = new anchor.BN(parseInt((nowInSec + HALF_HOUR).toString()));
    // const uidBuffer = uid.toBuffer("le", 8);

    let [auctionStatePubKey, auctionStateBump] =
      await anchor.web3.PublicKey.findProgramAddress(
        [
          // Buffer.from("auction"),
          AUCTION_SEED,
          auctionCreator.toBuffer(),
          treasuryMint.toBuffer(),
          // uidBuffer,
        ],
        program.programId
      );

    let [treasuryWalletPubKey, treasuryWalletBump] =
      await anchor.web3.PublicKey.findProgramAddress(
        [
          // Buffer.from("treasury"),
          TREASURY_SEED,
          auctionCreator.toBuffer(),
          treasuryMint.toBuffer(),
          // uidBuffer,
        ],
        program.programId
      );

    let [biddersChestWalletPubKey, biddersChestWalletBump] =
      await anchor.web3.PublicKey.findProgramAddress(
        [
          // Buffer.from("wallet"),
          CHEST_WALLET_SEED,
          auctionCreator.toBuffer(),
          chestMint.toBuffer(),
          // uidBuffer,
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

  /**
   * OUTDATED!!!
   */
  const readTokenAccount = async (
    accountPublicKey: anchor.web3.PublicKey,
    provider: anchor.Provider
  ): Promise<[spl.RawAccount, string]> => {
    const tokenInfo = await provider.connection.getAccountInfo(
      accountPublicKey
    );
    const data = Buffer.from(tokenInfo.data);
    const accountInfo = spl.AccountLayout.decode(data);

    const amount = (accountInfo.amount as any as Buffer).readBigUInt64LE();
    return [accountInfo, amount.toString()];
  };

  const readMintAccount = async (
    mintPublicKey: anchor.web3.PublicKey,
    provider: anchor.Provider
  ): Promise<spl.RawMint> => {
    const mintInfo = await provider.connection.getAccountInfo(mintPublicKey);
    const data = Buffer.from(mintInfo.data);
    const accountInfo = spl.MintLayout.decode(data);

    // TODO: Not sure if I need this
    // return {
    //   ...accountInfo,
    //   mintAuthority:
    //     accountInfo.mintAuthority == null
    //       ? null
    //       : anchor.web3.PublicKey.decode(Buffer.from(accountInfo.mintAuthority)),
    //   freezeAuthority:
    //     accountInfo.freezeAuthority == null
    //       ? null
    //       : anchor.web3.PublicKey.decode(accountInfo.freezeAuthority),
    // };

    return accountInfo;
  };

  beforeEach(async () => {
    treasuryMintAddress = await createMint(provider.connection);
    boyncTokenMintAddress = await createMint(provider.connection);

    [auctionCreator, auctionCreatorAssocWallet] = await createUserAndAssociatedWallet(
      treasuryMintAddress
    );

    [bidder1, bidder1ATA] = await createUserAndAssociatedWallet(
      boyncTokenMintAddress
    );

    let _rest;
    [user2, ..._rest] = await createUserAndAssociatedWallet();

    _pda = await getPDAParams(auctionCreator.publicKey,
                              treasuryMintAddress,
                              boyncTokenMintAddress);
  });

  /*
  it("Is initialized!", async () => {
    let auctionCreatorTokenAccountBalancePre =
      await provider.connection.getTokenAccountBalance(auctionCreatorAssocWallet);
    assert.equal(auctionCreatorTokenAccountBalancePre.value.amount, "100000000");

    const amount = new anchor.BN(20000000);

    await program.methods
      .initialize(_pda.idx, amount, _pda.auctionStateBump)
      .accounts({
        state: _pda.auctionStateKey,
        treasury: _pda.treasuryWalletKey,
        biddersChest: _pda.biddersChestWalletKey,
        signer: auctionCreator.publicKey,
        treasuryMint: treasuryMintAddress,
        collectorMint: boyncTokenMintAddress,
        signerWithdrawWallet: auctionCreatorAssocWallet,

        systemProgram: anchor.web3.SystemProgram.programId,
        rent: anchor.web3.SYSVAR_RENT_PUBKEY,
        tokenProgram: spl.TOKEN_PROGRAM_ID,
      })
      .signers([auctionCreator])
      .rpc();

    // console.log(
    //   `Initialized a new Boync Auctions instance. auctionCreator sent 20 tokens to be auctioned.`
    // );

    let auctionCreatorTokenAccountBalancePost =
      await provider.connection.getTokenAccountBalance(auctionCreatorAssocWallet);
    assert.equal(auctionCreatorTokenAccountBalancePost.value.amount, "80000000");

    let treasuryAccountBalance =
      await provider.connection.getTokenAccountBalance(_pda.treasuryWalletKey);
    assert.equal(treasuryAccountBalance.value.amount, "20000000");

    let chestAccountBalance =
    await provider.connection.getTokenAccountBalance(_pda.biddersChestWalletKey);
    assert.equal(chestAccountBalance.value.amount, "0");

    const state = await program.account.boyncAuction.fetch(
      _pda.auctionStateKey
    );

    assert.equal(state.tokensAmount.toString(), "20000000");
    assert.equal(Object.keys(state.state)[0], 'created');

  });
  */
  
  it("User bid!", async () => {

    const amountToTreasury = new anchor.BN(20000000);
    const amountToBid = new anchor.BN(10000000);

    let bidderBalancePre =
      await provider.connection.getTokenAccountBalance(bidder1ATA);
    assert.equal(bidderBalancePre.value.amount, "100000000");
 
     await program.methods
      .initialize(_pda.idx, amountToTreasury, _pda.auctionStateBump)
      .accounts({
        state: _pda.auctionStateKey,
        treasury: _pda.treasuryWalletKey,
        biddersChest: _pda.biddersChestWalletKey,
        signer: auctionCreator.publicKey,
        treasuryMint: treasuryMintAddress,
        collectorMint: boyncTokenMintAddress,
        signerWithdrawWallet: auctionCreatorAssocWallet,

        systemProgram: anchor.web3.SystemProgram.programId,
        rent: anchor.web3.SYSVAR_RENT_PUBKEY,
        tokenProgram: spl.TOKEN_PROGRAM_ID,
      })
      .signers([auctionCreator])
      .rpc();

    let state = await program.account.boyncAuction.fetch(
      _pda.auctionStateKey
    );

    assert.equal(state.tokensAmount.toString(), "20000000");
    assert.equal(Object.keys(state.state)[0], 'created');

    await program.methods
      .start()
      .accounts({
        auction: _pda.auctionStateKey,
        authority: auctionCreator.publicKey
      })
      .signers([auctionCreator])
      .rpc();

    state = await program.account.boyncAuction.fetch(
      _pda.auctionStateKey
    );

    assert.equal(Object.keys(state.state)[0], 'started');

    await program.methods
      .bid(amountToBid, _pda.auctionStateBump)
      .accounts({
        state: _pda.auctionStateKey,
        biddersChest: _pda.biddersChestWalletKey,
        bidder: bidder1.publicKey,
        collectorMint: boyncTokenMintAddress,
        bidderWithdrawWallet: bidder1ATA,

        rent: anchor.web3.SYSVAR_RENT_PUBKEY,
        tokenProgram: spl.TOKEN_PROGRAM_ID,
      })
      .signers([bidder1])
      .rpc();

    let bidderBalancePost =
      await provider.connection.getTokenAccountBalance(bidder1ATA);
    assert.equal(bidderBalancePost.value.amount, "90000000");

    let chestBalance =
      await provider.connection.getTokenAccountBalance(_pda.biddersChestWalletKey);
    assert.equal(chestBalance.value.amount, "10000000");

    state = await program.account.boyncAuction.fetch(
      _pda.auctionStateKey
    );

    assert.equal(bidder1.publicKey.toBase58(), state.lastBidder.toBase58());
  });

  it("Claim bid!", async () => {

    /* NOT IMPLEMENTED
    const amountToTreasury = new anchor.BN(20000000);
    const amountToBid = new anchor.BN(10000000);

    let bidderBalancePre =
      await provider.connection.getTokenAccountBalance(bidder1ATA);
    assert.equal(bidderBalancePre.value.amount, "100000000");
 
     await program.methods
      .initialize(_pda.idx, amountToTreasury, _pda.auctionStateBump)
      .accounts({
        state: _pda.auctionStateKey,
        treasury: _pda.treasuryWalletKey,
        biddersChest: _pda.biddersChestWalletKey,
        signer: auctionCreator.publicKey,
        treasuryMint: treasuryMintAddress,
        collectorMint: boyncTokenMintAddress,
        signerWithdrawWallet: auctionCreatorAssocWallet,

        systemProgram: anchor.web3.SystemProgram.programId,
        rent: anchor.web3.SYSVAR_RENT_PUBKEY,
        tokenProgram: spl.TOKEN_PROGRAM_ID,
      })
      .signers([auctionCreator])
      .rpc();

    let state = await program.account.boyncAuction.fetch(
      _pda.auctionStateKey
    );

    assert.equal(state.tokensAmount.toString(), "20000000");
    assert.equal(Object.keys(state.state)[0], 'created');

    await program.methods
      .start()
      .accounts({
        auction: _pda.auctionStateKey,
        authority: auctionCreator.publicKey
      })
      .signers([auctionCreator])
      .rpc();

    state = await program.account.boyncAuction.fetch(
      _pda.auctionStateKey
    );

    assert.equal(Object.keys(state.state)[0], 'started');

    await program.methods
      .bid(amountToBid, _pda.auctionStateBump)
      .accounts({
        state: _pda.auctionStateKey,
        biddersChest: _pda.biddersChestWalletKey,
        bidder: bidder1.publicKey,
        collectorMint: boyncTokenMintAddress,
        bidderWithdrawWallet: bidder1ATA,

        rent: anchor.web3.SYSVAR_RENT_PUBKEY,
        tokenProgram: spl.TOKEN_PROGRAM_ID,
      })
      .signers([bidder1])
      .rpc();

    let bidderBalancePost =
      await provider.connection.getTokenAccountBalance(bidder1ATA);
    assert.equal(bidderBalancePost.value.amount, "90000000");

    let chestBalance =
      await provider.connection.getTokenAccountBalance(_pda.biddersChestWalletKey);
    assert.equal(chestBalance.value.amount, "10000000");

    state = await program.account.boyncAuction.fetch(
      _pda.auctionStateKey
    );

    // console.log('[bidder]:', bidder1.publicKey.toBase58());
    // console.log('[state]:', state.lastBidder.toBase58());

    assert.equal(bidder1.publicKey.toBase58(), state.lastBidder.toBase58());
    */
  })

});
