/* eslint-disable @typescript-eslint/no-unsafe-assignment */
/* eslint-disable @typescript-eslint/no-unsafe-member-access */
import * as anchor from "@project-serum/anchor";
import { Program } from "@project-serum/anchor";
import { BoyncAnchorProgram } from "../../target/types/boync_anchor_program";
import * as spl from '@solana/spl-token';

import {
  Connection,
  PublicKey,
} from '@solana/web3.js';
import path from 'path';
import fs from 'mz/fs';

import {createKeypairFromFile} from './utils';

/**
 * Boync Program as generated by `anchor build`;
 */
const program = anchor.workspace.BoyncAnchorProgram as Program<BoyncAnchorProgram>;
/**
 * Anchor Provider;
 */
const anchorProvider = anchor.AnchorProvider.env();

/**
 * Connection to the network
 */
let connection: Connection;

/**
 * BoyncUser Bob's PDA
 */
// let bobUserPDA: PublicKey;

/**
 * BoyncAuction Bob's PDA
 */
let bobAuctionPDA: PublicKey;


/**
 * Main BoyncAuction program id
 */
let programId: PublicKey;

/**
 * Path to program files
 */
const PROGRAM_PATH = path.resolve(__dirname, '../../target/deploy');

/**
 * Path to program shared object file which should be deployed on chain.
 * This file is created when running either:
 *   - `npm run build:program-rust`
 */
const PROGRAM_SO_PATH = path.join(PROGRAM_PATH, 'boync_anchor_program.so');

/**
 * Path to the keypair of the deployed program.
 * This file is created when running `solana program deploy dist/program/helloworld.so`
 */
const PROGRAM_KEYPAIR_PATH = path.join(PROGRAM_PATH, 'boync_anchor_program-keypair.json');

/**
 * [From Tests]
 */

interface PDAParams {
  treasuryBump:       number,
  biddersChestBump:   number,
  auctionStateBump:   number,
  idx:                anchor.BN,
  treasuryWalletKey:  anchor.web3.PublicKey,
  biddersChestWalletKey:    anchor.web3.PublicKey,
  auctionStateKey:    anchor.web3.PublicKey,
}

let treasuryMintAddress: anchor.web3.PublicKey;
let boyncTokenMintAddress: anchor.web3.PublicKey;
let bidder1: anchor.web3.Keypair;
let bidder1ATA: anchor.web3.PublicKey;
let auctionCreator: anchor.web3.Keypair;
let auctionCreatorAssocWallet: anchor.web3.PublicKey;
let user2: anchor.web3.Keypair;
let user2AssocWallet: anchor.web3.PublicKey;
let _pda: PDAParams;

const _wallet = anchor.Wallet.local();

const createUserAndAssociatedWallet = async (
  mint?: anchor.web3.PublicKey
): Promise<[anchor.web3.Keypair, anchor.web3.PublicKey | undefined]> => {
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
  const sigTxFund = await anchorProvider.sendAndConfirm(txFund);
  // console.log(`[${user.publicKey.toBase58()}] Funded new account with 5 SOL: ${sigTxFund}`);

  if (mint) {
    // Create a token account for the user and mint some tokens
    userATA = await spl.createAssociatedTokenAccount(
      anchorProvider.connection, // connection
      user, // fee payer
      mint, // mint
      user.publicKey // owner,
    );

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
    await anchorProvider.sendAndConfirm(tx);
  }

  return [user, userATA];
};

const createMint = async (): Promise<anchor.web3.PublicKey> => {
  const tokenMint = new anchor.web3.Keypair();
  const lamportsForMint =
    await anchorProvider.connection.getMinimumBalanceForRentExemption(
      spl.MintLayout.span
    );

  let tx = new anchor.web3.Transaction()
    .add(
      // Create the mint account
      anchor.web3.SystemProgram.createAccount({
        programId: spl.TOKEN_PROGRAM_ID,
        space: spl.MintLayout.span,
        fromPubkey: _wallet.publicKey,
        newAccountPubkey: tokenMint.publicKey,
        lamports: lamportsForMint,
      })
    )
    // Init the mint account
    .add(
      spl.createInitializeMintInstruction(
        tokenMint.publicKey,
        6,
        _wallet.publicKey,
        _wallet.publicKey,
        spl.TOKEN_PROGRAM_ID
      )
    );

  const signature = await anchorProvider.sendAndConfirm(tx, [tokenMint]);

  // console.log(
  //   `[${tokenMint.publicKey}] Created new mint account at ${signature}`
  // );

  return tokenMint.publicKey;
};

const HALF_HOUR = (30 * 60);
const TREASURY_SEED = anchor.utils.bytes.utf8.encode("treasury");
const AUCTION_SEED = anchor.utils.bytes.utf8.encode("auction");
const CHEST_WALLET_SEED = anchor.utils.bytes.utf8.encode("wallet")

const getPDAParams = async (
   auctionCreator: anchor.web3.PublicKey,
   treasuryMint: anchor.web3.PublicKey,
   chestMint: anchor.web3.PublicKey
 ): Promise<PDAParams> => {
   const nowInSec = Math.floor(Date.now() / 1000);
   let uid = new anchor.BN(parseInt((nowInSec + HALF_HOUR).toString()));
   const uidBuffer = uid.toBuffer("le", 8);

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
         chestMint.toBuffer(),
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
/**
 * Establish a connection to the cluster
 */
export async function establishConnection(): Promise<void> {
  anchor.setProvider(anchorProvider);

  connection = anchorProvider.connection;

  // console.log('[Connection]:', connection);
  // console.log('[AnchorProvider]:', anchor.getProvider());
  console.log('[Anchor] Provider set. Connection details:', 
    anchorProvider.connection.rpcEndpoint, 
    anchorProvider.connection.getVersion()
  );

  console.log('[Anchor] establishConnection: Done!');
}

/**
 * Check if the hello world BPF program has been deployed
 */
export async function checkBoyncProgram(): Promise<void> {
  // Read program id from keypair file
  try {
    const programKeypair = await createKeypairFromFile(PROGRAM_KEYPAIR_PATH);
    programId = programKeypair.publicKey;
  } catch (err) {
    const errMsg = (err as Error).message;
    throw new Error(
      `Failed to read program keypair at '${PROGRAM_KEYPAIR_PATH}' due to error: ${errMsg}. Program may need to be deployed with \`solana program deploy dist/program/boyncprogram.so\``
    );
  }

  // Check if the program has been deployed
  const programInfo = await connection.getAccountInfo(programId);
  if (programInfo === null) {
    if (fs.existsSync(PROGRAM_SO_PATH)) {
      throw new Error(
        "Program needs to be deployed with `solana program deploy dist/program/helloworld.so`"
      );
    } else {
      throw new Error("Program needs to be built and deployed");
    }
  } else if (!programInfo.executable) {
    throw new Error(`Program is not executable`);
  }

  console.log(`Using program ${programId.toBase58()}`);

  //Derive the address (public key) of a greeting account from the program so that it's easy to find later.
  // boyncAuctionPubkey = await PublicKey.createWithSeed(
  //   payer.publicKey,
  //   "hello", // seed
  //   programId
  // );
}

// export async function initBoyncUser(): Promise<void> {
//   const [_bobUserPDA, _] = await PublicKey
//   .findProgramAddress(
//     [
//       anchor.utils.bytes.utf8.encode('user'),
//       anchorProvider.wallet.publicKey.toBuffer(),
//     ],
//     program.programId
//   );

//   bobUserPDA = _bobUserPDA;

//   console.log('[Anchor] Initialize Boync User with Bob User PDA:', bobUserPDA.toBase58());
//   console.log('[Anchor] Payer:', anchorProvider.wallet.publicKey.toBase58());

//   const bobData = await program.account.boyncUserData.fetch(bobUserPDA);
//   if (bobData != null || typeof bobData != 'undefined') {
//     console.log(`[Anchor] Boync User with Bob User PDA ${bobUserPDA.toBase58()} already defined:`);
//     console.log(bobData);
//     return;
//   }

//   const tx = await program.methods
//   .initialize('Bob')
//   .accounts({
//     user: anchorProvider.wallet.publicKey,
//     userData: bobUserPDA,
//   })
//   .rpc();

//   console.log('[Anchor] Done, tx:', tx);
// }

export async function preInitBoyncAuction(): Promise<void> {
  // console.log('[preInitBoyncAuction] here 1');
  treasuryMintAddress = await createMint();
  // console.log('[preInitBoyncAuction] here 2');
  boyncTokenMintAddress = await createMint();

  // console.log('[preInitBoyncAuction] here 3');
  [auctionCreator, auctionCreatorAssocWallet] =
    await createUserAndAssociatedWallet(treasuryMintAddress);

  // console.log('[preInitBoyncAuction] here 4');
  [bidder1, bidder1ATA] = await createUserAndAssociatedWallet(
    boyncTokenMintAddress
  );

  // console.log('[preInitBoyncAuction] here 5');
  let _rest;
  [user2, ..._rest] = await createUserAndAssociatedWallet();

  // console.log('[preInitBoyncAuction] here 6');
  _pda = await getPDAParams(
    auctionCreator.publicKey,
    treasuryMintAddress,
    boyncTokenMintAddress
  );
}

export async function initBoyncAuction(): Promise<void> {
  let auctionCreatorTokenAccountBalancePre =
    await anchorProvider.connection.getTokenAccountBalance(auctionCreatorAssocWallet);

  // assert.equal(auctionCreatorTokenAccountBalancePre.value.amount, "100000000");
  console.log('[initBoyncAuction][auctionCreatorTokenAccountBalancePre]:',
    auctionCreatorTokenAccountBalancePre.value.amount);

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

  console.log(
    `Initialized a new Boync Auctions instance. auctionCreator sent 20 tokens to be auctioned.`
  );

  let auctionCreatorTokenAccountBalancePost =
    await anchorProvider.connection.getTokenAccountBalance(auctionCreatorAssocWallet);
  console.log('[initBoyncAuction][auctionCreatorTokenAccountBalancePost]:',
    auctionCreatorTokenAccountBalancePost.value.amount);
  // assert.equal(auctionCreatorTokenAccountBalancePost.value.amount, "80000000");

  let treasuryAccountBalance = await anchorProvider.connection.getTokenAccountBalance(
    _pda.treasuryWalletKey
  );
  console.log('[initBoyncAuction][treasuryAccountBalance]:',
    treasuryAccountBalance.value.amount);
  // assert.equal(treasuryAccountBalance.value.amount, "20000000");

  let chestAccountBalance = await anchorProvider.connection.getTokenAccountBalance(
    _pda.biddersChestWalletKey
  );
  // assert.equal(chestAccountBalance.value.amount, "0");
  console.log('[initBoyncAuction][chestAccountBalance]:',
    chestAccountBalance.value.amount);

  const state = await program.account.boyncAuction.fetch(_pda.auctionStateKey);

  console.log('[initBoyncAuction][state.tokensAmount]:',
    state.tokensAmount.toString());
  console.log('[initBoyncAuction][state.state]:',
    state.state);

  // assert.equal(state.tokensAmount.toString(), "20000000");
  // assert.equal(Object.keys(state.state)[0], "created");

  /*
  const token: PublicKey = new PublicKey("ARbt5mrsfmi32fJ5Uk1Mbo26e5u77CwjJ4Cf9HWXUQe9");
  const [_bobAuctionPDA, _] = await PublicKey
  .findProgramAddress(
    [
      anchor.utils.bytes.utf8.encode('auction'),
      anchorProvider.wallet.publicKey.toBuffer(),
    ],
    program.programId
  );

  bobAuctionPDA = _bobAuctionPDA;

  console.log('[Anchor] Initialize Boync Auction with Bob User PDA:', bobAuctionPDA.toBase58());
  console.log('[Anchor] Payer:', anchorProvider.wallet.publicKey.toBase58());
  console.log('[Anchor] Token Id:', token.toBase58());

  try {
    const _bobAuctionData = await program.account.boyncAuctionData.fetch(bobAuctionPDA);
    if (_bobAuctionData != null || typeof _bobAuctionData != "undefined") {
      console.log(
        `[Anchor] Boync Auction with Bob User PDA ${bobAuctionPDA.toBase58()} already defined:`
      );
      return;
    }
  } catch (error) {
    console.log(`[Anchor] Account ${bobAuctionPDA.toBase58()}, does not exist ... move on to creating it!`);
  }
  
  const tx = await program.methods
  .initialize('Bob', token)
  .accounts({
    authority: anchorProvider.wallet.publicKey,
    userData: _bobAuctionPDA,
  })
  .rpc();

  console.log('[Anchor] Done, tx:', tx);
  */
}

// export async function checkBoyncUser(): Promise<void> {
//   console.log('[Anchor] Fetching Boync User with PDA:', bobUserPDA.toBase58());

//   const bobData = await program.account.boyncUserData.fetch(bobUserPDA);

//   console.log(`[Anchor] Done. Boync User Data: { name: ${bobData.name}, user: ${bobData.user.toBase58()} }`);
// }

export async function checkBoyncAuction(): Promise<void> {
  /*
  console.log('[Anchor] Fetching Boync User with PDA:', bobAuctionPDA.toBase58());

  const bobAuctionData = await program.account.boyncAuctionData.fetch(bobAuctionPDA);

  console.log('[Anchor] Done.');
  console.log(`Boync Auction Data: { name: ${bobAuctionData.name}, authority: ${bobAuctionData.authority.toBase58()}, token: ${bobAuctionData.token.toBase58()}`);
  */
}