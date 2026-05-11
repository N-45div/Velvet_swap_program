import { expect } from "chai";
import fs from "fs";
import { MagicBlockPrivatePaymentsAdapter } from "../src/sponsors/magicblock";
import { UmbraShieldedPayoutAdapter } from "../src/sponsors/umbra";

describe("live sponsor adapters", function () {
  this.timeout(300000);

  const rpcUrl =
    process.env.ANCHOR_PROVIDER_URL ||
    "https://devnet.helius-rpc.com/?api-key=2d8978c6-7067-459f-ae97-7ea035f1a0cb";
  const wsolMint = "So11111111111111111111111111111111111111112";

  function fundedWalletSecretKey() {
    return Uint8Array.from(
      JSON.parse(fs.readFileSync(`${process.env.HOME}/.config/solana/id.json`, "utf8"))
    );
  }

  it("builds a real MagicBlock private payment transaction payload", async () => {
    const adapter = new MagicBlockPrivatePaymentsAdapter();
    const handoff = await adapter.buildPrivateTransfer({
      from: "3rXKwQ1kpjBd5tdcco32qsvqUh1BnZjcYnS5kYrP7AYE",
      to: "Bt9oNR5cCtnfuMmXgWELd6q5i974PdEMQDUE55nBC57L",
      mint: "So11111111111111111111111111111111111111112",
      amount: 1,
      visibility: "private",
      fromBalance: "base",
      toBalance: "base",
      cluster: "devnet",
      initIfMissing: true,
      initAtasIfMissing: true,
      initVaultIfMissing: false,
      memo: "VelvetMesh live sponsor probe",
      minDelayMs: "0",
      maxDelayMs: "0",
      clientRefId: "42",
      split: 1,
      gasless: false,
      legacy: true,
    });

    expect(handoff.provider).to.equal("magicBlockPrivatePayment");
    expect(handoff.payloadHash).to.have.length(32);
    expect(handoff.referenceHash).to.have.length(32);
    expect(handoff.response.kind).to.equal("transfer");
    expect(handoff.response.transactionBase64).to.be.a("string").and.have.length.greaterThan(100);
    expect(handoff.response.requiredSigners).to.include("3rXKwQ1kpjBd5tdcco32qsvqUh1BnZjcYnS5kYrP7AYE");
  });

  it("constructs a real Umbra devnet SDK client and sponsor handoff payload", async () => {
    const adapter = new UmbraShieldedPayoutAdapter();
    const client = await adapter.createDevnetClient({
      rpcUrl,
      rpcSubscriptionsUrl: "wss://api.devnet.solana.com",
    });

    const handoff = adapter.buildShieldedPayoutHandoff({
      network: "devnet",
      recipient: String(client.signer.address),
      mint: wsolMint,
      amount: "1",
      memo: "VelvetMesh shielded payout handoff",
    });

    expect(client.network).to.equal("devnet");
    expect(String(client.signer.address)).to.have.length.greaterThan(32);
    expect(handoff.provider).to.equal("umbraShieldedPayout");
    expect(handoff.payloadHash).to.have.length(32);
    expect(handoff.referenceHash).to.have.length(32);
  });

  it("checks Umbra devnet relayer-supported mints before settlement", async () => {
    const adapter = new UmbraShieldedPayoutAdapter();
    const relayerInfo = await adapter.assertDevnetMintSupported(wsolMint);

    expect(relayerInfo.mints).to.include(wsolMint);
    expect(Number(relayerInfo.count)).to.be.greaterThan(0);
  });

  it("registers or verifies the funded wallet as a real Umbra confidential devnet user", async () => {
    const adapter = new UmbraShieldedPayoutAdapter();
    const result = await adapter.registerDevnetConfidentialUser({
      rpcUrl,
      rpcSubscriptionsUrl: "wss://api.devnet.solana.com",
      secretKey: fundedWalletSecretKey(),
    });

    expect(result.address).to.equal("4daUQ43GtRM4tkptESK8SdSrRWRg6Szh77DACXeVrGRF");
    expect(result.signatures).to.be.an("array");
  });

  it("runs real Umbra devnet deposit and withdrawal when explicitly enabled", async function () {
    if (process.env.RUN_UMBRA_E2E !== "true") {
      this.skip();
    }

    const adapter = new UmbraShieldedPayoutAdapter();
    const config = {
      rpcUrl,
      rpcSubscriptionsUrl: "wss://api.devnet.solana.com",
      secretKey: fundedWalletSecretKey(),
    };

    const depositResult = await adapter.depositPublicToEncryptedBalance(config, {
      mint: wsolMint,
      amount: 1_000_000n,
    });
    const balance = await adapter.queryEncryptedBalance(config, wsolMint);
    const withdrawResult = await adapter.withdrawEncryptedToPublicBalance(config, {
      mint: wsolMint,
      amount: 500_000n,
    });

    expect(depositResult.callbackStatus).to.equal("finalized");
    expect(balance).to.deep.include({ state: "shared" });
    expect(withdrawResult.callbackStatus).to.equal("finalized");
  });
});
