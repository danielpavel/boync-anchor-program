import * as anchor from "@project-serum/anchor";
import { Program } from "@project-serum/anchor";
import { BoyncAnchorProgram } from "../target/types/boync_anchor_program";

import { PublicKey } from '@solana/web3.js';
import { expect } from "chai";

describe("boync-anchor-program", () => {
  // Configure the client to use the local cluster.
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.BoyncAnchorProgram as Program<BoyncAnchorProgram>;

  it("Creating user Bob", async () => {
    // Add your test here.
    const [bobUserPDA, _] = await PublicKey
      .findProgramAddress(
        [
          anchor.utils.bytes.utf8.encode('user'),
          provider.wallet.publicKey.toBuffer(),
        ],
        program.programId
      );

    const tx = await program.methods
      .initialize('Bob')
      .accounts({
        user: provider.wallet.publicKey,
        userData: bobUserPDA,
      })
      .rpc();

    console.log("[Success] Your transaction signature", tx);

    const bobData = await program.account.boyncUserData.fetch(bobUserPDA);
    expect(bobData.name).to.be.eql('Bob');
  });
});
