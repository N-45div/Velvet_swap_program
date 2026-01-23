import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import {
  ComputeBudgetProgram,
  Connection,
  Keypair,
  PublicKey,
  SendTransactionError,
  sendAndConfirmTransaction,
  SystemProgram,
} from "@solana/web3.js";
import * as nacl from "tweetnacl";
import { encryptValue } from "@inco/solana-sdk/encryption";
import { hexToBuffer } from "@inco/solana-sdk/utils";
import {
  AUTHORITY_FLAG,
  TX_LOGS_FLAG,
  TX_BALANCES_FLAG,
  TX_MESSAGE_FLAG,
  ACCOUNT_SIGNATURES_FLAG,
  PERMISSION_PROGRAM_ID,
  Member,
  createDelegatePermissionInstruction,
  getAuthToken,
  getPermissionStatus,
  permissionPdaFromAccount,
  waitUntilPermissionActive,
} from "@magicblock-labs/ephemeral-rollups-sdk";
import { PrivateSwapPrograms } from "../target/types/private_swap_programs";
import { IncoToken } from "../target/types/inco_token";

const INCO_LIGHTNING_PROGRAM_ID = new PublicKey(
  "5sjEbPiqgZrYwR31ahR6Uk9wf5awoX61YGg7jExQSwaj"
);
const ER_VALIDATOR = new PublicKey(
  "FnE6VJT5QNZdedZPnCoLsARgBwoE6DeJNjBs2H1gySXA"
);
const TEE_URL = "https://tee.magicblock.app";
const TEE_WS_URL = "wss://tee.magicblock.app";
const INPUT_TYPE = 0;
const DECIMALS = 9;

describe("private_swap_programs", function () {
  this.timeout(200000);

  const connection = new Connection(
    process.env.ANCHOR_PROVIDER_URL || "https://api.devnet.solana.com",
    "confirmed"
  );
  const provider = new anchor.AnchorProvider(
    connection,
    anchor.AnchorProvider.env().wallet,
    {
      commitment: "confirmed",
      preflightCommitment: "confirmed",
    }
  );
  anchor.setProvider(provider);

  const ephemeralRpcEndpoint = (
    process.env.EPHEMERAL_PROVIDER_ENDPOINT || TEE_URL
  ).replace(/\/$/, "");
  let authToken: { token: string; expiresAt: number } | undefined;
  let providerTee: anchor.AnchorProvider | undefined;
  let validator = ER_VALIDATOR;

  const swapProgram = anchor.workspace
    .privateSwapPrograms as Program<PrivateSwapPrograms>;
  const incoTokenProgram = anchor.workspace.IncoToken as Program<IncoToken>;

  const authority = provider.wallet.publicKey;
  console.log("Authority:", authority.toBase58());
  const authoritySigner = provider.wallet.payer;
  let mintA: Keypair;
  let mintB: Keypair;
  let userTokenA: PublicKey;
  let userTokenB: PublicKey;
  let poolTokenA: PublicKey;
  let poolTokenB: PublicKey;
  let poolPda: PublicKey;
  let permissionForPool: PublicKey;

  const encryptAmount = async (amount: bigint) =>
    hexToBuffer(await encryptValue(amount));

  const deriveIncoTokenPda = (wallet: PublicKey, mint: PublicKey) =>
    PublicKey.findProgramAddressSync(
      [wallet.toBuffer(), incoTokenProgram.programId.toBuffer(), mint.toBuffer()],
      incoTokenProgram.programId
    )[0];

  const computeBudgetIxs = () => [
    ComputeBudgetProgram.setComputeUnitLimit({ units: 400_000 }),
  ];

  const logSendTransactionError = async (
    error: unknown,
    connection: Connection,
    label: string
  ) => {
    if (error instanceof SendTransactionError) {
      try {
        const logs = await error.getLogs(connection);
        console.error(`❌ ${label} logs:`, logs);
      } catch (logError) {
        console.error(`❌ ${label} failed to fetch logs:`, logError);
        console.error(`❌ ${label} error message:`, error.message);
      }
      return;
    }
    console.error(`❌ ${label} error:`, error);
  };

  before(async () => {
    mintA = Keypair.generate();
    mintB = Keypair.generate();
    [poolPda] = PublicKey.findProgramAddressSync(
      [
        Buffer.from("pool"),
        mintA.publicKey.toBuffer(),
        mintB.publicKey.toBuffer(),
      ],
      swapProgram.programId
    );
    permissionForPool = permissionPdaFromAccount(poolPda);

    userTokenA = deriveIncoTokenPda(authority, mintA.publicKey);
    userTokenB = deriveIncoTokenPda(authority, mintB.publicKey);
    poolTokenA = deriveIncoTokenPda(poolPda, mintA.publicKey);
    poolTokenB = deriveIncoTokenPda(poolPda, mintB.publicKey);
  });

  it("initializes inco mints and token accounts", async () => {
    await incoTokenProgram.methods
      .initializeMint(DECIMALS, authority, authority)
      .accounts({
        mint: mintA.publicKey,
        payer: authority,
        systemProgram: SystemProgram.programId,
        incoLightningProgram: INCO_LIGHTNING_PROGRAM_ID,
      })
      .signers([mintA])
      .rpc();

    await incoTokenProgram.methods
      .initializeMint(DECIMALS, authority, authority)
      .accounts({
        mint: mintB.publicKey,
        payer: authority,
        systemProgram: SystemProgram.programId,
        incoLightningProgram: INCO_LIGHTNING_PROGRAM_ID,
      })
      .signers([mintB])
      .rpc();

    const initAccount = async (
      account: PublicKey,
      mint: PublicKey,
      owner: PublicKey
    ) => {
      await incoTokenProgram.methods
        .createIdempotent()
        .accounts({
          payer: authority,
          associatedToken: account,
          mint,
          wallet: owner,
          systemProgram: SystemProgram.programId,
          incoLightningProgram: INCO_LIGHTNING_PROGRAM_ID,
        })
        .rpc();
    };

    await initAccount(userTokenA, mintA.publicKey, authority);
    await initAccount(userTokenB, mintB.publicKey, authority);
    await initAccount(poolTokenA, mintA.publicKey, poolPda);
    await initAccount(poolTokenB, mintB.publicKey, poolPda);
  });

  it("initializes pool", async () => {
    await swapProgram.methods
      .initializePool(30)
      .accounts({
        authority,
        mintA: mintA.publicKey,
        mintB: mintB.publicKey,
        pool: poolPda,
        systemProgram: SystemProgram.programId,
        incoLightningProgram: INCO_LIGHTNING_PROGRAM_ID,
      })
      .rpc();
  });

  it("configures PER permission for pool", async () => {
    const poolInfo = await connection.getAccountInfo(poolPda);
    console.log(
      "Pool PDA:",
      poolPda.toBase58(),
      "owner:",
      poolInfo?.owner.toBase58()
    );
    if (ephemeralRpcEndpoint.includes("tee")) {
      authToken = await getAuthToken(
        ephemeralRpcEndpoint,
        authority,
        (message: Uint8Array) =>
          Promise.resolve(
            nacl.sign.detached(message, provider.wallet.payer.secretKey)
          )
      );
      providerTee = new anchor.AnchorProvider(
        new Connection(`${TEE_URL}?token=${authToken.token}`, {
          wsEndpoint: `${TEE_WS_URL}?token=${authToken.token}`,
        }),
        provider.wallet
      );
    } else {
      providerTee = new anchor.AnchorProvider(
        new Connection(ephemeralRpcEndpoint, {
          wsEndpoint: process.env.EPHEMERAL_WS_ENDPOINT,
        }),
        provider.wallet
      );
    }

    try {
      const identityResponse = await (providerTee.connection as any)._rpcRequest(
        "getIdentity",
        []
      );
      const identity = identityResponse?.result?.identity;
      if (identity) {
        validator = new PublicKey(identity);
        console.log("TEE validator:", validator.toBase58());
      }
    } catch (error) {
      console.warn("⚠️ Failed to fetch TEE validator identity, using default", error);
    }
    console.log("TEE provider wallet:", providerTee.wallet.publicKey.toBase58());

    const mintAmountA = BigInt(1_000_000_000);
    const mintAmountB = BigInt(2_000_000_000);
    await incoTokenProgram.methods
      .mintTo(await encryptAmount(mintAmountA), INPUT_TYPE)
      .accounts({
        mint: mintA.publicKey,
        account: userTokenA,
        mintAuthority: authority,
        incoLightningProgram: INCO_LIGHTNING_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .rpc();

    await incoTokenProgram.methods
      .mintTo(await encryptAmount(mintAmountB), INPUT_TYPE)
      .accounts({
        mint: mintB.publicKey,
        account: userTokenB,
        mintAuthority: authority,
        incoLightningProgram: INCO_LIGHTNING_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .rpc();

    const baseMember: Member = {
      flags:
        AUTHORITY_FLAG |
        TX_LOGS_FLAG |
        TX_BALANCES_FLAG |
        TX_MESSAGE_FLAG |
        ACCOUNT_SIGNATURES_FLAG,
      pubkey: authority,
    };
    const poolSignerMember: Member = {
      flags:
        AUTHORITY_FLAG |
        TX_LOGS_FLAG |
        TX_BALANCES_FLAG |
        TX_MESSAGE_FLAG |
        ACCOUNT_SIGNATURES_FLAG,
      pubkey: poolPda,
    };
    const validatorMember: Member = {
      flags:
        AUTHORITY_FLAG |
        TX_LOGS_FLAG |
        TX_BALANCES_FLAG |
        TX_MESSAGE_FLAG |
        ACCOUNT_SIGNATURES_FLAG,
      pubkey: validator,
    };
    const programMember: Member = {
      flags:
        AUTHORITY_FLAG |
        TX_LOGS_FLAG |
        TX_BALANCES_FLAG |
        TX_MESSAGE_FLAG |
        ACCOUNT_SIGNATURES_FLAG,
      pubkey: swapProgram.programId,
    };
    const members: Member[] = [
      baseMember,
      poolSignerMember,
      validatorMember,
      programMember,
    ];
    const poolTokenMembers: Member[] = [
      baseMember,
      poolSignerMember,
      validatorMember,
      programMember,
    ];

    const createPermissionIx = await swapProgram.methods
      .createPermission(
        { pool: { mintA: mintA.publicKey, mintB: mintB.publicKey } },
        members
      )
      .accountsPartial({
        payer: authority,
        permissionedAccount: poolPda,
        permission: permissionForPool,
        systemProgram: SystemProgram.programId,
      })
      .instruction();

    const delegatePermissionIx = createDelegatePermissionInstruction({
      payer: authority,
      validator,
      permissionedAccount: [poolPda, false],
      authority: [authority, true],
    });

    const delegatePoolIx = await swapProgram.methods
      .delegatePda({ pool: { mintA: mintA.publicKey, mintB: mintB.publicKey } })
      .accounts({
        payer: authority,
        validator,
        pda: poolPda,
      })
      .instruction();

    const tx = new anchor.web3.Transaction().add(
      createPermissionIx,
      delegatePermissionIx,
      delegatePoolIx
    );
    tx.feePayer = authority;
    try {
      await provider.sendAndConfirm(tx, []);
    } catch (error) {
      await logSendTransactionError(error, provider.connection, "pool permission tx");
      throw error;
    }

    const isActive = await waitUntilPermissionActive(
      ephemeralRpcEndpoint,
      poolPda
    );
    if (isActive) {
      console.log("✅ Pool permission active:", poolPda.toBase58());
      const status = await getPermissionStatus(ephemeralRpcEndpoint, poolPda);
      console.log("Pool permission users:", status.authorizedUsers ?? []);
    } else {
      console.log("❌ Pool permission not active:", poolPda.toBase58());
    }

    const ensurePermissionForTokenAccount = async (
      label: string,
      account: PublicKey,
      wallet: PublicKey,
      mint: PublicKey,
      membersOverride: Member[] = members
    ) => {
      const permission = permissionPdaFromAccount(account);
      const createPermissionIx = await incoTokenProgram.methods
        .createPermissionForIncoAccount(membersOverride)
        .accounts({
          permissionedAccount: account,
          permission,
          payer: authority,
          wallet,
          mint,
          permissionProgram: PERMISSION_PROGRAM_ID,
          systemProgram: SystemProgram.programId,
        })
        .instruction();
      const delegatePermissionIx = createDelegatePermissionInstruction({
        payer: authority,
        validator,
        permissionedAccount: [account, false],
        authority: [authority, true],
      });
      const accountInfo = await connection.getAccountInfo(account);
      console.log(
        `${label} account owner:`,
        accountInfo?.owner.toBase58()
      );
      const delegateAccountIx = await incoTokenProgram.methods
        .delegateIncoAccount()
        .accounts({
          payer: authority,
          wallet,
          mint,
          pda: account,
          validator,
        })
        .instruction();

      const tx = new anchor.web3.Transaction().add(
        createPermissionIx,
        delegatePermissionIx,
        delegateAccountIx
      );
      const writableKeys = new Map(
        tx.instructions
          .flatMap((ix) => ix.keys)
          .filter((key) => key.isWritable)
          .map((key) => [key.pubkey.toBase58(), key.pubkey])
      );
      const writableAccounts = await connection.getMultipleAccountsInfo(
        Array.from(writableKeys.values())
      );
      writableAccounts.forEach((info, index) => {
        if (info?.executable) {
          const pubkey = Array.from(writableKeys.values())[index];
          console.warn(
            `⚠️ Writable executable account in ${label} permission tx:`,
            pubkey.toBase58(),
            "owner:",
            info.owner.toBase58()
          );
        }
      });
      tx.feePayer = authority;
      try {
        await provider.sendAndConfirm(tx, []);
      } catch (error) {
        await logSendTransactionError(
          error,
          provider.connection,
          `${label} permission tx`
        );
        throw error;
      }

      const active = await waitUntilPermissionActive(ephemeralRpcEndpoint, account);
      if (active) {
        console.log(`✅ ${label} permission active:`, account.toBase58());
        const status = await getPermissionStatus(
          ephemeralRpcEndpoint,
          account
        );
        console.log(`${label} permission users:`, status.authorizedUsers ?? []);
      } else {
        console.log(`❌ ${label} permission not active:`, account.toBase58());
      }
    };

    await ensurePermissionForTokenAccount(
      "User token A",
      userTokenA,
      authority,
      mintA.publicKey
    );
    await ensurePermissionForTokenAccount(
      "User token B",
      userTokenB,
      authority,
      mintB.publicKey
    );
    await ensurePermissionForTokenAccount(
      "Pool token A",
      poolTokenA,
      poolPda,
      mintA.publicKey,
      poolTokenMembers
    );
    await ensurePermissionForTokenAccount(
      "Pool token B",
      poolTokenB,
      poolPda,
      mintB.publicKey,
      poolTokenMembers
    );
  });

  it("mints user balances and adds liquidity", async () => {
    const amountA = BigInt(1_000_000_000);
    const amountB = BigInt(2_000_000_000);
    const amountACiphertext = await encryptAmount(amountA);
    const amountBCiphertext = await encryptAmount(amountB);

    const addLiquidityTx = await swapProgram.methods
      .addLiquidity(amountACiphertext, amountBCiphertext, INPUT_TYPE)
      .preInstructions(computeBudgetIxs())
      .accounts({
        authority,
        pool: poolPda,
        userTokenA,
        userTokenB,
        poolTokenA,
        poolTokenB,
        systemProgram: SystemProgram.programId,
        incoLightningProgram: INCO_LIGHTNING_PROGRAM_ID,
        incoTokenProgram: incoTokenProgram.programId,
      })
      .transaction();

    const teeProvider = providerTee ?? provider;
    try {
      const teeCheck = await teeProvider.connection.getAccountInfo(
        userTokenA
      );
      console.log("TEE read user token A:", teeCheck !== null);
    } catch (error) {
      console.error("❌ TEE read user token A failed", error);
    }
    addLiquidityTx.feePayer = authority;
    addLiquidityTx.recentBlockhash = (
      await teeProvider.connection.getLatestBlockhash()
    ).blockhash;
    try {
      await sendAndConfirmTransaction(
        teeProvider.connection,
        addLiquidityTx,
        [authoritySigner],
        { skipPreflight: true, commitment: "confirmed" }
      );
    } catch (error) {
      await logSendTransactionError(error, teeProvider.connection, "add liquidity");
      throw error;
    }
  });

  it("swaps A to B", async () => {
    const amountIn = BigInt(100_000_000);
    const amountOut = BigInt(50_000_000);
    const feeAmount = BigInt(0);

    const swapTx = await swapProgram.methods
      .swapExactIn(
        await encryptAmount(amountIn),
        await encryptAmount(amountOut),
        await encryptAmount(feeAmount),
        INPUT_TYPE,
        true
      )
      .preInstructions(computeBudgetIxs())
      .accounts({
        authority,
        pool: poolPda,
        userTokenA,
        userTokenB,
        poolTokenA,
        poolTokenB,
        systemProgram: SystemProgram.programId,
        incoLightningProgram: INCO_LIGHTNING_PROGRAM_ID,
        incoTokenProgram: incoTokenProgram.programId,
      })
      .transaction();

    const teeProvider = providerTee ?? provider;
    swapTx.feePayer = authority;
    swapTx.recentBlockhash = (
      await teeProvider.connection.getLatestBlockhash()
    ).blockhash;
    try {
      await sendAndConfirmTransaction(
        teeProvider.connection,
        swapTx,
        [authoritySigner],
        { skipPreflight: true, commitment: "confirmed" }
      );
    } catch (error) {
      await logSendTransactionError(error, teeProvider.connection, "swap A to B");
      throw error;
    }
  });

  it("removes liquidity", async () => {
    const removeA = BigInt(100_000_000);
    const removeB = BigInt(150_000_000);

    const removeLiquidityTx = await swapProgram.methods
      .removeLiquidity(
        await encryptAmount(removeA),
        await encryptAmount(removeB),
        INPUT_TYPE
      )
      .preInstructions(computeBudgetIxs())
      .accounts({
        authority,
        pool: poolPda,
        userTokenA,
        userTokenB,
        poolTokenA,
        poolTokenB,
        systemProgram: SystemProgram.programId,
        incoLightningProgram: INCO_LIGHTNING_PROGRAM_ID,
        incoTokenProgram: incoTokenProgram.programId,
      })
      .transaction();

    const teeProvider = providerTee ?? provider;
    removeLiquidityTx.feePayer = authority;
    removeLiquidityTx.recentBlockhash = (
      await teeProvider.connection.getLatestBlockhash()
    ).blockhash;
    try {
      await sendAndConfirmTransaction(
        teeProvider.connection,
        removeLiquidityTx,
        [authoritySigner],
        { skipPreflight: true, commitment: "confirmed" }
      );
    } catch (error) {
      await logSendTransactionError(
        error,
        teeProvider.connection,
        "remove liquidity"
      );
      throw error;
    }
  });
});
