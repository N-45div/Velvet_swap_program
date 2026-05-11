/**
 * VelvetMesh devnet state-machine test.
 *
 * This test is intentionally devnet-oriented. It does not mock Solana state.
 * Run after `anchor build` and `anchor deploy --provider.cluster devnet`:
 *
 *   npx ts-mocha -p ./tsconfig.json -t 600000 tests/velvet_mesh_devnet.ts
 *
 * This test intentionally does not fake Arcium cryptography. It verifies that
 * Arcium-backed intents can queue/request private matching, cannot settle before
 * the verifier records a selected quote, and can settle after that handoff.
 */

import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { Keypair, PublicKey, SystemProgram, Transaction } from "@solana/web3.js";
import { expect } from "chai";
import { MagicBlockPrivatePaymentsAdapter } from "../src/sponsors/magicblock";
import { VelvetMesh } from "../target/types/velvet_mesh";

const DIRECT_SOLANA_P2P = 1 << 0;
const VELVETSWAP_FALLBACK = 1 << 1;

const handle = (seed: number): number[] => Array.from({ length: 32 }, (_, index) => (seed + index) % 256);

describe("velvet_mesh devnet", function () {
  this.timeout(600000);

  const apiKey = process.env.HELIUS_DEVNET_API_KEY;
  const rpcUrl = apiKey
    ? `https://devnet.helius-rpc.com/?api-key=${apiKey}`
    : "https://api.devnet.solana.com";

  process.env.ANCHOR_PROVIDER_URL = rpcUrl;
  process.env.ANCHOR_WALLET =
    process.env.ANCHOR_WALLET || `${process.env.HOME}/.config/solana/id.json`;

  const connection = new anchor.web3.Connection(rpcUrl, "confirmed");
  const provider = new anchor.AnchorProvider(connection, anchor.AnchorProvider.env().wallet, {
    commitment: "confirmed",
    preflightCommitment: "confirmed",
  });
  anchor.setProvider(provider);

  const program = anchor.workspace.VelvetMesh as Program<VelvetMesh>;
  const owner = provider.wallet.publicKey;
  const maker = Keypair.generate();
  const matchVerifier = Keypair.generate();
  const settlementVerifier = Keypair.generate();
  const inputMint = Keypair.generate().publicKey;
  const outputMint = Keypair.generate().publicKey;
  const intentNonce = new anchor.BN(Date.now());
  const allowedRoutes = DIRECT_SOLANA_P2P | VELVETSWAP_FALLBACK;
  const arciumComputation = handle(90);
  const quoteCommitment = handle(80);
  const settlementHash = handle(150);
  let settlementPayloadHash = handle(170);
  let settlementReferenceHash = handle(190);

  let intentPda: PublicKey;
  let quotePda: PublicKey;
  let acceptedMatchPda: PublicKey;
  let settlementHandoffPda: PublicKey;

  before(async () => {
    [intentPda] = PublicKey.findProgramAddressSync(
      [
        Buffer.from("intent"),
        owner.toBuffer(),
        intentNonce.toArrayLike(Buffer, "le", 8),
      ],
      program.programId
    );

    [quotePda] = PublicKey.findProgramAddressSync(
      [Buffer.from("quote"), intentPda.toBuffer(), maker.publicKey.toBuffer()],
      program.programId
    );

    [acceptedMatchPda] = PublicKey.findProgramAddressSync(
      [Buffer.from("match"), intentPda.toBuffer()],
      program.programId
    );

    [settlementHandoffPda] = PublicKey.findProgramAddressSync(
      [Buffer.from("settlement"), acceptedMatchPda.toBuffer()],
      program.programId
    );

    for (const signer of [maker, matchVerifier, settlementVerifier]) {
      const balance = await connection.getBalance(signer.publicKey);
      if (balance < 0.2 * anchor.web3.LAMPORTS_PER_SOL) {
        await provider.sendAndConfirm(
          new Transaction().add(
            SystemProgram.transfer({
              fromPubkey: owner,
              toPubkey: signer.publicKey,
              lamports: 0.5 * anchor.web3.LAMPORTS_PER_SOL,
            })
          )
        );
      }
    }
  });

  it("creates an Arcium-backed private intent", async () => {
    const now = Math.floor(Date.now() / 1000);

    await program.methods
      .createIntent(intentNonce, {
        inputMint,
        outputMint,
        encryptedSize: handle(1),
        encryptedLimitPrice: handle(10),
        encryptedSlippageBps: handle(20),
        encryptedRiskPreference: handle(30),
        allowedRoutes,
        computeProvider: { arcium: {} },
        matchVerifier: matchVerifier.publicKey,
        settlementVerifier: settlementVerifier.publicKey,
        minQuoteCount: 1,
        metadataHash: handle(40),
        expiresAt: new anchor.BN(now + 3600),
      })
      .accounts({
        owner,
        intent: intentPda,
        systemProgram: SystemProgram.programId,
      })
      .rpc();

    const intent = await program.account.intent.fetch(intentPda);
    expect(intent.owner.toBase58()).to.equal(owner.toBase58());
    expect(intent.computeProvider).to.deep.equal({ arcium: {} });
    expect(intent.status).to.deep.equal({ open: {} });
    expect(intent.allowedRoutes).to.equal(allowedRoutes);
    expect(intent.matchVerifier.toBase58()).to.equal(matchVerifier.publicKey.toBase58());
    expect(intent.settlementVerifier.toBase58()).to.equal(settlementVerifier.publicKey.toBase58());
  });

  it("submits a maker quote with encrypted handles", async () => {
    const now = Math.floor(Date.now() / 1000);

    await program.methods
      .submitQuote({
        route: { directSolanaP2P: {} },
        encryptedOutputAmount: handle(50),
        encryptedPriceBps: handle(60),
        encryptedMakerRisk: handle(70),
        quoteCommitment,
        settlementHash,
        expiresAt: new anchor.BN(now + 1800),
      })
      .accounts({
        maker: maker.publicKey,
        intent: intentPda,
        quote: quotePda,
        systemProgram: SystemProgram.programId,
      })
      .signers([maker])
      .rpc();

    const quote = await program.account.quote.fetch(quotePda);
    const intent = await program.account.intent.fetch(intentPda);
    expect(quote.intent.toBase58()).to.equal(intentPda.toBase58());
    expect(quote.route).to.deep.equal({ directSolanaP2P: {} });
    expect(intent.quoteCount).to.equal(1);
  });

  it("blocks settlement until the match verifier records the selected quote", async () => {
    await program.methods
      .requestPrivateMatch(arciumComputation)
      .accounts({
        owner,
        intent: intentPda,
      })
      .rpc();

    let intent = await program.account.intent.fetch(intentPda);
    expect(intent.status).to.deep.equal({ computationRequested: {} });
    expect(Array.from(intent.arciumComputation)).to.deep.equal(arciumComputation);

    try {
      await program.methods
        .acceptQuote()
        .accounts({
          owner,
          intent: intentPda,
          quote: quotePda,
          acceptedMatch: acceptedMatchPda,
          systemProgram: SystemProgram.programId,
        })
        .rpc();
      throw new Error("acceptQuote unexpectedly succeeded without Arcium callback");
    } catch (error: any) {
      expect(error.message).to.contain("Intent is not acceptable");
    }

    await program.methods
      .recordPrivateMatch(arciumComputation, quoteCommitment, { directSolanaP2P: {} })
      .accounts({
        matchVerifier: matchVerifier.publicKey,
        intent: intentPda,
        selectedQuote: quotePda,
      })
      .signers([matchVerifier])
      .rpc();

    intent = await program.account.intent.fetch(intentPda);
    expect(intent.status).to.deep.equal({ matchReady: {} });
    expect(intent.selectedQuote.toBase58()).to.equal(quotePda.toBase58());
  });

  it("accepts the verified private match and marks settlement ready", async () => {
    await program.methods
      .acceptQuote()
      .accounts({
        owner,
        intent: intentPda,
        quote: quotePda,
        acceptedMatch: acceptedMatchPda,
        systemProgram: SystemProgram.programId,
      })
      .rpc();

    let intent = await program.account.intent.fetch(intentPda);
    let acceptedMatch = await program.account.acceptedMatch.fetch(acceptedMatchPda);
    expect(intent.status).to.deep.equal({ accepted: {} });
    expect(acceptedMatch.quote.toBase58()).to.equal(quotePda.toBase58());
    expect(acceptedMatch.settlementVerifier.toBase58()).to.equal(settlementVerifier.publicKey.toBase58());
    expect(acceptedMatch.settlementReady).to.equal(false);

    const magicBlockHandoff = await new MagicBlockPrivatePaymentsAdapter().buildPrivateTransfer({
      from: owner.toBase58(),
      to: maker.publicKey.toBase58(),
      mint: "So11111111111111111111111111111111111111112",
      amount: 1,
      visibility: "private",
      fromBalance: "base",
      toBalance: "base",
      cluster: "devnet",
      initIfMissing: true,
      initAtasIfMissing: true,
      initVaultIfMissing: false,
      memo: "VelvetMesh verified match settlement",
      minDelayMs: "0",
      maxDelayMs: "0",
      clientRefId: String(intentNonce.toNumber()),
      split: 1,
      gasless: false,
      legacy: true,
    });
    settlementPayloadHash = magicBlockHandoff.payloadHash;
    settlementReferenceHash = magicBlockHandoff.referenceHash;

    try {
      await program.methods
        .prepareSettlementHandoff({
          provider: { umbraShieldedPayout: {} },
          payloadHash: Array(32).fill(0),
          referenceHash: settlementReferenceHash,
        })
        .accounts({
          owner,
          acceptedMatch: acceptedMatchPda,
          settlementHandoff: settlementHandoffPda,
          systemProgram: SystemProgram.programId,
        })
        .rpc();
      throw new Error("prepareSettlementHandoff unexpectedly accepted a zero payload hash");
    } catch (error: any) {
      expect(error.message).to.contain("Settlement payload hash must be non-zero");
    }

    try {
      await program.methods
        .markSettlementReady()
        .accounts({
          settlementVerifier: settlementVerifier.publicKey,
          acceptedMatch: acceptedMatchPda,
          settlementHandoff: settlementHandoffPda,
        })
        .signers([settlementVerifier])
        .rpc();
      throw new Error("markSettlementReady unexpectedly succeeded without a sponsor handoff");
    } catch (error: any) {
      expect(error.message).to.contain("AccountNotInitialized");
    }

    await program.methods
      .prepareSettlementHandoff({
        provider: { magicBlockPrivatePayment: {} },
        payloadHash: settlementPayloadHash,
        referenceHash: settlementReferenceHash,
      })
      .accounts({
        owner,
        acceptedMatch: acceptedMatchPda,
        settlementHandoff: settlementHandoffPda,
        systemProgram: SystemProgram.programId,
      })
      .rpc();

    acceptedMatch = await program.account.acceptedMatch.fetch(acceptedMatchPda);
    const settlementHandoff = await program.account.settlementHandoff.fetch(settlementHandoffPda);
    expect(acceptedMatch.settlementProvider).to.deep.equal({ magicBlockPrivatePayment: {} });
    expect(Array.from(acceptedMatch.settlementPayloadHash)).to.deep.equal(settlementPayloadHash);
    expect(Array.from(acceptedMatch.settlementReferenceHash)).to.deep.equal(settlementReferenceHash);
    expect(settlementHandoff.status).to.deep.equal({ prepared: {} });

    try {
      await program.methods
        .markSettlementReady()
        .accounts({
          settlementVerifier: matchVerifier.publicKey,
          acceptedMatch: acceptedMatchPda,
          settlementHandoff: settlementHandoffPda,
        })
        .signers([matchVerifier])
        .rpc();
      throw new Error("markSettlementReady unexpectedly accepted the wrong verifier");
    } catch (error: any) {
      expect(error.message).to.contain("A has one constraint was violated");
    }

    await program.methods
      .markSettlementReady()
      .accounts({
        settlementVerifier: settlementVerifier.publicKey,
        acceptedMatch: acceptedMatchPda,
        settlementHandoff: settlementHandoffPda,
      })
      .signers([settlementVerifier])
      .rpc();

    acceptedMatch = await program.account.acceptedMatch.fetch(acceptedMatchPda);
    expect(acceptedMatch.settlementReady).to.equal(true);
    expect(acceptedMatch.settlementConfirmedAt.toNumber()).to.be.greaterThan(0);

    try {
      await program.methods
        .markSettlementReady()
        .accounts({
          settlementVerifier: settlementVerifier.publicKey,
          acceptedMatch: acceptedMatchPda,
          settlementHandoff: settlementHandoffPda,
        })
        .signers([settlementVerifier])
        .rpc();
      throw new Error("markSettlementReady unexpectedly allowed double confirmation");
    } catch (error: any) {
      expect(error.message).to.contain("Settlement handoff has already been confirmed");
    }
  });
});
