import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import {
  AddressLookupTableProgram,
  Keypair,
  PublicKey,
  SystemProgram,
  Transaction,
} from "@solana/web3.js";
import { expect } from "chai";
import { randomBytes } from "crypto";
import fs from "fs";
import path from "path";
import {
  awaitComputationFinalization,
  deserializeLE,
  getArciumProgramId,
  getClockAccAddress,
  getClusterAccAddress,
  getCompDefAccAddress,
  getCompDefAccOffset,
  getComputationAccAddress,
  getExecutingPoolAccAddress,
  getFeePoolAccAddress,
  getLookupTableAddress,
  getMempoolAccAddress,
  getMXEAccAddress,
  getMXEPublicKey,
  RescueCipher,
  uploadCircuit,
  x25519,
} from "@arcium-hq/client";
import { VelvetMeshMatcher } from "../target/types/velvet_mesh_matcher";

const ROOT = path.resolve(__dirname, "../../..");
const VELVET_MESH_PROGRAM_ID = new PublicKey("4GPgiWJN1WRifSvEVs8btvyq7Yinn6DNErnuyXDRHFFo");
const MATCHER_PROGRAM_ID = new PublicKey("CEjM2iFeNzKwDtc8uGLAGVFDoaHvJmy9EunRUwAsJH8e");
const SIGN_PDA_SEED = Buffer.from("ArciumSignerAccount");
const DIRECT_SOLANA_P2P_ROUTE_INDEX = 0;
const DIRECT_SOLANA_P2P_ROUTE_MASK = 1 << DIRECT_SOLANA_P2P_ROUTE_INDEX;

const handle = (seed: number): number[] => Array.from({ length: 32 }, (_, index) => (seed + index) % 256);

function loadJson(relativePath: string): any {
  return JSON.parse(fs.readFileSync(path.join(ROOT, relativePath), "utf8"));
}

describe("velvet_mesh_matcher Arcium callback devnet", function () {
  this.timeout(240000);

  const rpcUrl = process.env.ANCHOR_PROVIDER_URL || "https://api.devnet.solana.com";
  process.env.ANCHOR_PROVIDER_URL = rpcUrl;
  process.env.ANCHOR_WALLET =
    process.env.ANCHOR_WALLET || `${process.env.HOME}/.config/solana/id.json`;

  const connection = new anchor.web3.Connection(rpcUrl, "confirmed");
  const provider = new anchor.AnchorProvider(connection, anchor.AnchorProvider.env().wallet, {
    commitment: "confirmed",
    preflightCommitment: "confirmed",
  });
  anchor.setProvider(provider);

  const matcherIdl = loadJson("arcium/velvet_mesh_matcher/target/idl/velvet_mesh_matcher.json");
  const velvetMeshIdl = loadJson("target/idl/velvet_mesh.json");
  const matcher = new Program<VelvetMeshMatcher>(matcherIdl, provider);
  const velvetMesh = new Program(velvetMeshIdl, provider);
  const owner = provider.wallet.publicKey;

  it("queues a live Arcium private match and lets the callback mark VelvetMesh MatchReady", async function () {
    const mxePublicKey = await getMXEPublicKey(provider, MATCHER_PROGRAM_ID);
    if (!mxePublicKey) {
      this.skip();
      return;
    }

    const mxeAccount = getMXEAccAddress(MATCHER_PROGRAM_ID);
    const mxe = await matcher.account.mxeAccount.fetch(mxeAccount);
    if (mxe.cluster === null || mxe.cluster === undefined) {
      this.skip();
      return;
    }

    const clusterOffset = Number(mxe.cluster);
    const compDefOffset = Buffer.from(getCompDefAccOffset("select_private_quote")).readUInt32LE(0);
    const compDefAccount = getCompDefAccAddress(MATCHER_PROGRAM_ID, compDefOffset);
    const compDefInfo = await connection.getAccountInfo(compDefAccount);
    const addressLookupTable = getLookupTableAddress(MATCHER_PROGRAM_ID, mxe.lutOffsetSlot);

    if (!compDefInfo) {
      await matcher.methods
        .initSelectPrivateQuoteCompDef()
        .accounts({
          payer: owner,
          mxeAccount,
          compDefAccount,
          addressLookupTable,
          lutProgram: AddressLookupTableProgram.programId,
          arciumProgram: getArciumProgramId(),
          systemProgram: SystemProgram.programId,
        })
        .rpc();
    }

    const rawCircuit = fs.readFileSync(path.join(ROOT, "arcium/velvet_mesh_matcher/build/select_private_quote.arcis"));
    await uploadCircuit(provider, "select_private_quote", MATCHER_PROGRAM_ID, rawCircuit, false, 25, {
      commitment: "confirmed",
      skipPreflight: true,
    });

    const [matchVerifier] = PublicKey.findProgramAddressSync([SIGN_PDA_SEED], MATCHER_PROGRAM_ID);
    const maker0 = Keypair.generate();
    const maker1 = Keypair.generate();
    const maker2 = Keypair.generate();

    for (const signer of [maker0, maker1, maker2]) {
      await provider.sendAndConfirm(
        new Transaction().add(
          SystemProgram.transfer({
            fromPubkey: owner,
            toPubkey: signer.publicKey,
            lamports: 0.05 * anchor.web3.LAMPORTS_PER_SOL,
          })
        )
      );
    }

    const nonce = new anchor.BN(Date.now());
    const inputMint = Keypair.generate().publicKey;
    const outputMint = Keypair.generate().publicKey;
    const [intentPda] = PublicKey.findProgramAddressSync(
      [Buffer.from("intent"), owner.toBuffer(), nonce.toArrayLike(Buffer, "le", 8)],
      VELVET_MESH_PROGRAM_ID
    );

    const quotePdas = [maker0, maker1, maker2].map((maker) =>
      PublicKey.findProgramAddressSync(
        [Buffer.from("quote"), intentPda.toBuffer(), maker.publicKey.toBuffer()],
        VELVET_MESH_PROGRAM_ID
      )[0]
    );

    const now = Math.floor(Date.now() / 1000);
    await velvetMesh.methods
      .createIntent(nonce, {
        inputMint,
        outputMint,
        encryptedSize: handle(1),
        encryptedLimitPrice: handle(10),
        encryptedSlippageBps: handle(20),
        encryptedRiskPreference: handle(30),
        allowedRoutes: DIRECT_SOLANA_P2P_ROUTE_MASK,
        computeProvider: { arcium: {} },
        matchVerifier,
        settlementVerifier: owner,
        minQuoteCount: 3,
        metadataHash: handle(40),
        expiresAt: new anchor.BN(now + 3600),
      })
      .accounts({
        owner,
        intent: intentPda,
        systemProgram: SystemProgram.programId,
      })
      .rpc();

    for (const [index, maker] of [maker0, maker1, maker2].entries()) {
      await velvetMesh.methods
        .submitQuote({
          route: { directSolanaP2P: {} },
          encryptedOutputAmount: handle(50 + index * 10),
          encryptedPriceBps: handle(80 + index * 10),
          encryptedMakerRisk: handle(110 + index * 10),
          quoteCommitment: handle(140 + index * 10),
          settlementHash: handle(180 + index * 10),
          expiresAt: new anchor.BN(now + 1800),
        })
        .accounts({
          maker: maker.publicKey,
          intent: intentPda,
          quote: quotePdas[index],
          systemProgram: SystemProgram.programId,
        })
        .signers([maker])
        .rpc();
    }

    const computationOffset = new anchor.BN(randomBytes(8), "le");
    const computationAccount = getComputationAccAddress(clusterOffset, computationOffset);
    await velvetMesh.methods
      .requestPrivateMatch(Array.from(computationAccount.toBytes()))
      .accounts({
        owner,
        intent: intentPda,
      })
      .rpc();

    const privateKey = x25519.utils.randomSecretKey();
    const publicKey = x25519.getPublicKey(privateKey);
    const sharedSecret = x25519.getSharedSecret(privateKey, mxePublicKey);
    const cipher = new RescueCipher(sharedSecret);
    const nonceBytes = randomBytes(16);
    const plaintext = [
      100n,
      100n,
      BigInt(DIRECT_SOLANA_P2P_ROUTE_INDEX),
      110n,
      90n,
      BigInt(DIRECT_SOLANA_P2P_ROUTE_INDEX),
      150n,
      40n,
      BigInt(DIRECT_SOLANA_P2P_ROUTE_INDEX),
      95n,
      20n,
      BigInt(DIRECT_SOLANA_P2P_ROUTE_INDEX),
    ];
    const ciphertexts = cipher.encrypt(plaintext, nonceBytes);

    await matcher.methods
      .requestPrivateMatch(
        computationOffset,
        ciphertexts,
        Array.from(publicKey),
        new anchor.BN(deserializeLE(nonceBytes).toString())
      )
      .accounts({
        payer: owner,
        signPdaAccount: matchVerifier,
        mxeAccount,
        mempoolAccount: getMempoolAccAddress(clusterOffset),
        executingPool: getExecutingPoolAccAddress(clusterOffset),
        computationAccount,
        compDefAccount,
        clusterAccount: getClusterAccAddress(clusterOffset),
        poolAccount: getFeePoolAccAddress(),
        clockAccount: getClockAccAddress(),
        velvetMeshProgram: VELVET_MESH_PROGRAM_ID,
        velvetMeshIntent: intentPda,
        quote0: quotePdas[0],
        quote1: quotePdas[1],
        quote2: quotePdas[2],
        systemProgram: SystemProgram.programId,
        arciumProgram: getArciumProgramId(),
      })
      .rpc({ skipPreflight: true });

    await awaitComputationFinalization(provider, computationOffset, MATCHER_PROGRAM_ID, "confirmed", 180000);

    const intent = await velvetMesh.account.intent.fetch(intentPda);
    expect(intent.status).to.deep.equal({ matchReady: {} });
    expect(intent.selectedQuote.toBase58()).to.equal(quotePdas[1].toBase58());
  });
});
