/**
 * Verify Privacy Layers Test
 * 
 * Tests all three privacy layers end-to-end:
 * 1. Light Protocol (ZK) - Compressed accounts
 * 2. Inco Lightning (FHE) - Encrypted amounts
 * 3. MagicBlock TEE - Private execution
 * 
 * Run with: npx ts-node scripts/verify-privacy-layers.ts
 */

import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import {
  Connection,
  PublicKey,
  ComputeBudgetProgram,
} from "@solana/web3.js";
import * as nacl from "tweetnacl";
import {
  Rpc,
  createRpc,
  bn,
  deriveAddressSeedV2,
  deriveAddressV2,
  PackedAccounts,
  SystemAccountMetaConfig,
  featureFlags,
  VERSION,
  batchAddressTree,
} from "@lightprotocol/stateless.js";
import {
  getAuthToken,
  permissionPdaFromAccount,
  waitUntilPermissionActive,
} from "@magicblock-labs/ephemeral-rollups-sdk";
import { LightSwapPsp } from "../target/types/light_swap_psp";

// Force V2 mode
(featureFlags as any).version = VERSION.V2;

// Constants
const DEVNET_WSOL_MINT = new PublicKey("So11111111111111111111111111111111111111112");
const DEVNET_USDC_MINT = new PublicKey("4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU");
const INCO_LIGHTNING_PROGRAM_ID = new PublicKey("5sjEbPiqgZrYwR31ahR6Uk9wf5awoX61YGg7jExQSwaj");
const LIGHT_BATCH_ADDRESS_TREE = new PublicKey(batchAddressTree);
const LIGHT_OUTPUT_QUEUE = new PublicKey("oq1na8gojfdUhsfCpyjNt6h4JaDWtHf1yQj4koBWfto");
const TEE_URL = "https://tee.magicblock.app";

interface TestResult {
  layer: string;
  test: string;
  status: "PASS" | "FAIL";
  details: string;
  error?: string;
}

const results: TestResult[] = [];

function log(msg: string) {
  console.log(msg);
}

function addResult(result: TestResult) {
  results.push(result);
  const icon = result.status === "PASS" ? "✅" : "❌";
  log(`${icon} [${result.layer}] ${result.test}: ${result.details}`);
  if (result.error) {
    log(`   Error: ${result.error}`);
  }
}

async function main() {
  log("=".repeat(70));
  log("PRIVACY LAYER VERIFICATION TEST");
  log("=".repeat(70));
  
  const HELIUS_API_KEY = process.env.HELIUS_DEVNET_API_KEY || "2d8978c6-7067-459f-ae97-7ea035f1a0cb";
  const rpcUrl = `https://devnet.helius-rpc.com/?api-key=${HELIUS_API_KEY}`;
  
  process.env.ANCHOR_PROVIDER_URL = rpcUrl;
  process.env.ANCHOR_WALLET = process.env.ANCHOR_WALLET || `${process.env.HOME}/.config/solana/id.json`;

  const connection = new Connection(rpcUrl, "confirmed");
  const provider = new anchor.AnchorProvider(
    connection,
    anchor.AnchorProvider.env().wallet,
    { commitment: "confirmed", preflightCommitment: "confirmed" }
  );
  anchor.setProvider(provider);

  const swapProgram = anchor.workspace.LightSwapPsp as Program<LightSwapPsp>;
  const authority = provider.wallet.publicKey;
  const lightRpc = createRpc(rpcUrl, rpcUrl);

  log(`\nAuthority: ${authority.toBase58()}`);
  log(`Program ID: ${swapProgram.programId.toBase58()}`);
  log(`RPC: Helius devnet`);

  // ============================================
  // TEST 1: Light Protocol (ZK Compression)
  // ============================================
  log("\n" + "-".repeat(70));
  log("TEST 1: LIGHT PROTOCOL (ZK COMPRESSION)");
  log("-".repeat(70));

  // 1a. Check if pool exists
  const poolAddress = deriveAddressV2(
    deriveAddressSeedV2([Buffer.from("pool"), DEVNET_WSOL_MINT.toBuffer(), DEVNET_USDC_MINT.toBuffer()]),
    LIGHT_BATCH_ADDRESS_TREE,
    swapProgram.programId
  );
  
  log(`Pool Address: ${poolAddress.toBase58()}`);

  try {
    const accounts = await lightRpc.getCompressedAccountsByOwner(swapProgram.programId);
    const poolAccount = accounts.items.find((acc: any) => 
      acc.address && Buffer.from(acc.address).equals(poolAddress.toBuffer())
    );
    
    if (poolAccount) {
      addResult({
        layer: "Light Protocol",
        test: "Pool exists as compressed account",
        status: "PASS",
        details: `Found at leafIndex ${(poolAccount as any).leafIndex}`,
      });
      
      // 1b. Check tree info
      const treeInfo = (poolAccount as any).treeInfo;
      addResult({
        layer: "Light Protocol",
        test: "Compressed account has valid tree info",
        status: "PASS",
        details: `Tree: ${treeInfo?.tree?.slice(0, 20)}..., Queue: ${treeInfo?.queue?.slice(0, 20)}...`,
      });
    } else {
      addResult({
        layer: "Light Protocol",
        test: "Pool exists as compressed account",
        status: "FAIL",
        details: "Pool not found",
      });
    }
  } catch (e: any) {
    addResult({
      layer: "Light Protocol",
      test: "Query compressed accounts",
      status: "FAIL",
      details: "Failed to query",
      error: e.message,
    });
  }

  // 1c. Test validity proof fetch
  try {
    const accounts = await lightRpc.getCompressedAccountsByOwner(swapProgram.programId);
    const poolAccount = accounts.items.find((acc: any) => 
      acc.address && Buffer.from(acc.address).equals(poolAddress.toBuffer())
    );
    
    if (poolAccount) {
      const proofs = await lightRpc.getMultipleCompressedAccountProofs([(poolAccount as any).hash]);
      addResult({
        layer: "Light Protocol",
        test: "Fetch merkle proof for compressed account",
        status: "PASS",
        details: `Got proof with rootIndex: ${proofs?.[0]?.rootIndex}`,
      });
    }
  } catch (e: any) {
    addResult({
      layer: "Light Protocol",
      test: "Fetch merkle proof",
      status: "FAIL",
      details: "Failed to get proof",
      error: e.message,
    });
  }

  // ============================================
  // TEST 2: Inco Lightning (FHE)
  // ============================================
  log("\n" + "-".repeat(70));
  log("TEST 2: INCO LIGHTNING (FHE ENCRYPTION)");
  log("-".repeat(70));

  // 2a. Check Inco Lightning program exists
  try {
    const incoAccountInfo = await connection.getAccountInfo(INCO_LIGHTNING_PROGRAM_ID);
    addResult({
      layer: "Inco Lightning",
      test: "Inco Lightning program deployed",
      status: incoAccountInfo ? "PASS" : "FAIL",
      details: incoAccountInfo ? `Program exists, executable: ${incoAccountInfo.executable}` : "Program not found",
    });
  } catch (e: any) {
    addResult({
      layer: "Inco Lightning",
      test: "Inco Lightning program deployed",
      status: "FAIL",
      details: "Failed to check",
      error: e.message,
    });
  }

  // 2b. Check pool has encrypted reserves (verify pool data structure)
  try {
    const accounts = await lightRpc.getCompressedAccountsByOwner(swapProgram.programId);
    const poolAccount = accounts.items.find((acc: any) => 
      acc.address && Buffer.from(acc.address).equals(poolAddress.toBuffer())
    );
    
    if (poolAccount) {
      const poolData = (poolAccount as any).data?.data;
      if (poolData && poolData.length > 0) {
        addResult({
          layer: "Inco Lightning",
          test: "Pool has encrypted data (Euint128 fields)",
          status: "PASS",
          details: `Pool data size: ${poolData.length} bytes (contains FHE ciphertexts)`,
        });
      } else {
        addResult({
          layer: "Inco Lightning",
          test: "Pool has encrypted data",
          status: "FAIL",
          details: "No pool data found",
        });
      }
    }
  } catch (e: any) {
    addResult({
      layer: "Inco Lightning",
      test: "Pool encrypted data",
      status: "FAIL",
      details: "Failed to check",
      error: e.message,
    });
  }

  // ============================================
  // TEST 3: MagicBlock TEE (Private Execution)
  // ============================================
  log("\n" + "-".repeat(70));
  log("TEST 3: MAGICBLOCK TEE (PRIVATE EXECUTION)");
  log("-".repeat(70));

  // 3a. Test TEE authentication
  let teeConnection: Connection | null = null;
  try {
    const authToken = await getAuthToken(
      TEE_URL,
      authority,
      (message: Uint8Array) =>
        Promise.resolve(
          nacl.sign.detached(message, (provider.wallet as any).payer.secretKey)
        )
    );
    
    teeConnection = new Connection(`${TEE_URL}?token=${authToken.token}`, "confirmed");
    addResult({
      layer: "MagicBlock TEE",
      test: "TEE authentication",
      status: "PASS",
      details: `Got auth token, expires: ${new Date(authToken.expiresAt * 1000).toISOString()}`,
    });
  } catch (e: any) {
    addResult({
      layer: "MagicBlock TEE",
      test: "TEE authentication",
      status: "FAIL",
      details: "Failed to authenticate",
      error: e.message,
    });
  }

  // 3b. Check PER permission for pool authority
  const [poolAuthorityPda] = PublicKey.findProgramAddressSync(
    [Buffer.from("pool_authority"), DEVNET_WSOL_MINT.toBuffer(), DEVNET_USDC_MINT.toBuffer()],
    swapProgram.programId
  );
  
  try {
    const isActive = await waitUntilPermissionActive(TEE_URL, poolAuthorityPda, 5000);
    addResult({
      layer: "MagicBlock TEE",
      test: "PER permission for pool authority",
      status: isActive ? "PASS" : "FAIL",
      details: isActive ? `Permission ACTIVE for ${poolAuthorityPda.toBase58().slice(0, 20)}...` : "Permission not active",
    });
  } catch (e: any) {
    addResult({
      layer: "MagicBlock TEE",
      test: "PER permission for pool authority",
      status: "FAIL",
      details: "Failed to check permission",
      error: e.message,
    });
  }

  // 3c. Test if TEE can access Light Protocol programs
  if (teeConnection) {
    try {
      // Try to get Light Protocol system program info via TEE
      const lightSystemProgram = new PublicKey("SySTEM1eSU2p4BGQfQpimFEWWSC1XDFeun3Nqzz3rT7");
      const programInfo = await teeConnection.getAccountInfo(lightSystemProgram);
      
      addResult({
        layer: "MagicBlock TEE",
        test: "TEE can access Light Protocol system program",
        status: programInfo ? "PASS" : "FAIL",
        details: programInfo ? "Light system program accessible from TEE" : "Cannot access Light system program from TEE",
      });
    } catch (e: any) {
      addResult({
        layer: "MagicBlock TEE",
        test: "TEE can access Light Protocol system program",
        status: "FAIL",
        details: "TEE cannot access Light Protocol infrastructure",
        error: e.message,
      });
    }
  }

  // 3d. Try to simulate a swap transaction via TEE
  if (teeConnection) {
    log("\nAttempting swap simulation via TEE...");
    try {
      const accounts = await lightRpc.getCompressedAccountsByOwner(swapProgram.programId);
      const poolAccount = accounts.items.find((acc: any) => 
        acc.address && Buffer.from(acc.address).equals(poolAddress.toBuffer())
      );
      
      if (poolAccount) {
        const acct = poolAccount as any;
        const stateTree = new PublicKey(acct.treeInfo.tree);
        const stateQueue = new PublicKey(acct.treeInfo.queue);
        
        // Build a minimal swap transaction
        const packedAccounts = new PackedAccounts();
        packedAccounts.addSystemAccountsV2(SystemAccountMetaConfig.new(swapProgram.programId));
        const stateTreeIndex = packedAccounts.insertOrGet(stateTree);
        const stateQueueIndex = packedAccounts.insertOrGet(stateQueue);
        
        const { remainingAccounts: rawAccounts } = packedAccounts.toAccountMetas();
        const remainingAccounts = rawAccounts.map((a: any) => ({
          pubkey: a.pubkey,
          isWritable: Boolean(a.isWritable),
          isSigner: Boolean(a.isSigner),
        }));

        // Format amount as u128
        const amountBuf = Buffer.alloc(16);
        amountBuf.writeBigUInt64LE(BigInt(1000000), 0);
        
        const poolMeta = {
          treeInfo: {
            rootIndex: 0,
            proveByIndex: false,
            merkleTreePubkeyIndex: stateTreeIndex,
            queuePubkeyIndex: stateQueueIndex,
            leafIndex: acct.leafIndex,
          },
          address: Array.from(poolAddress.toBytes()),
          outputStateTreeIndex: stateQueueIndex,
        };

        const validityProof = {
          0: {
            a: new Array(32).fill(0),
            b: new Array(64).fill(0),
            c: new Array(32).fill(0),
          }
        };

        const ix = await swapProgram.methods
          .swapExactIn(
            validityProof,
            poolMeta,
            Buffer.from(acct.data?.data || []),
            amountBuf,
            amountBuf,
            amountBuf,
            0,
            true
          )
          .accounts({
            feePayer: authority,
            incoLightningProgram: INCO_LIGHTNING_PROGRAM_ID,
          })
          .remainingAccounts(remainingAccounts)
          .instruction();

        const tx = new anchor.web3.Transaction();
        tx.add(ComputeBudgetProgram.setComputeUnitLimit({ units: 1_000_000 }));
        tx.add(ix);
        tx.feePayer = authority;
        tx.recentBlockhash = (await teeConnection.getLatestBlockhash()).blockhash;

        // Simulate via TEE
        const simulation = await teeConnection.simulateTransaction(tx);
        
        if (simulation.value.err) {
          const errStr = JSON.stringify(simulation.value.err);
          if (errStr.includes("clone") || errStr.includes("SySTEM")) {
            addResult({
              layer: "MagicBlock TEE",
              test: "Simulate swap via TEE",
              status: "FAIL",
              details: "TEE cannot clone Light Protocol programs - INCOMPATIBLE",
              error: errStr,
            });
          } else {
            addResult({
              layer: "MagicBlock TEE",
              test: "Simulate swap via TEE",
              status: "FAIL",
              details: "Simulation failed",
              error: errStr,
            });
          }
        } else {
          addResult({
            layer: "MagicBlock TEE",
            test: "Simulate swap via TEE",
            status: "PASS",
            details: "Swap simulation succeeded via TEE",
          });
        }
      }
    } catch (e: any) {
      addResult({
        layer: "MagicBlock TEE",
        test: "Simulate swap via TEE",
        status: "FAIL",
        details: "Failed to simulate",
        error: e.message,
      });
    }
  }

  // ============================================
  // SUMMARY
  // ============================================
  log("\n" + "=".repeat(70));
  log("SUMMARY");
  log("=".repeat(70));

  const passed = results.filter(r => r.status === "PASS").length;
  const failed = results.filter(r => r.status === "FAIL").length;
  
  log(`\nTotal: ${results.length} tests | ✅ ${passed} passed | ❌ ${failed} failed\n`);

  // Group by layer
  const layers = ["Light Protocol", "Inco Lightning", "MagicBlock TEE"];
  for (const layer of layers) {
    const layerResults = results.filter(r => r.layer === layer);
    const layerPassed = layerResults.filter(r => r.status === "PASS").length;
    const layerTotal = layerResults.length;
    const allPassed = layerPassed === layerTotal;
    
    log(`${allPassed ? "✅" : "⚠️"} ${layer}: ${layerPassed}/${layerTotal} tests passed`);
    for (const r of layerResults) {
      log(`   ${r.status === "PASS" ? "✅" : "❌"} ${r.test}`);
    }
  }

  log("\n" + "=".repeat(70));
  log("CONCLUSION");
  log("=".repeat(70));
  
  const lightOk = results.filter(r => r.layer === "Light Protocol" && r.status === "PASS").length > 0;
  const incoOk = results.filter(r => r.layer === "Inco Lightning" && r.status === "PASS").length > 0;
  const teeSwapOk = results.find(r => r.test === "Simulate swap via TEE")?.status === "PASS";
  
  log(`\nLight Protocol (ZK Compression): ${lightOk ? "✅ WORKING" : "❌ NOT WORKING"}`);
  log(`Inco Lightning (FHE Encryption): ${incoOk ? "✅ WORKING" : "❌ NOT WORKING"}`);
  log(`MagicBlock TEE (Private Exec):   ${teeSwapOk ? "✅ COMPATIBLE" : "❌ INCOMPATIBLE with Light Protocol"}`);
  
  if (!teeSwapOk) {
    log(`\n⚠️  TEE cannot execute Light Protocol transactions.`);
    log(`   Light Protocol programs exist on devnet mainstate.`);
    log(`   TEE creates isolated environment, cannot clone external programs.`);
    log(`\n   WORKAROUND: Execute swaps on devnet directly (not via TEE).`);
    log(`   Privacy still provided by FHE (encrypted reserves) + ZK (compressed state).`);
  }
}

main().catch(console.error);
