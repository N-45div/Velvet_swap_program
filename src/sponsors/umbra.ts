import {
  createInMemorySigner,
  createSignerFromPrivateKeyBytes,
  getEncryptedBalanceQuerierFunction,
  getEncryptedBalanceToPublicBalanceDirectWithdrawerFunction,
  getPublicBalanceToEncryptedBalanceDirectDepositorFunction,
  getUmbraRelayer,
  getUmbraClient,
  getUserRegistrationFunction,
} from "@umbra-privacy/sdk";
import { SponsorHandoffPayload, sha256Bytes, textHash } from "./handoff";

const DEFAULT_DEVNET_RELAYER_ENDPOINT = "https://relayer.api-devnet.umbraprivacy.com";
const DEFAULT_DEVNET_INDEXER_ENDPOINT = "https://utxo-indexer.api-devnet.umbraprivacy.com";

export type UmbraDevnetClientConfig = {
  rpcUrl: string;
  rpcSubscriptionsUrl: string;
  indexerApiEndpoint?: string;
};

export type UmbraFundedDevnetConfig = UmbraDevnetClientConfig & {
  secretKey: Uint8Array;
  relayerApiEndpoint?: string;
};

export class UmbraShieldedPayoutAdapter {
  async createDevnetClient(config: UmbraDevnetClientConfig) {
    const signer = await createInMemorySigner();
    return getUmbraClient({
      signer,
      network: "devnet",
      rpcUrl: config.rpcUrl,
      rpcSubscriptionsUrl: config.rpcSubscriptionsUrl,
      indexerApiEndpoint: config.indexerApiEndpoint,
      deferMasterSeedSignature: true,
    } as any);
  }

  async createFundedDevnetClient(config: UmbraFundedDevnetConfig) {
    const signer = await createSignerFromPrivateKeyBytes(config.secretKey);
    return getUmbraClient({
      signer,
      network: "devnet",
      rpcUrl: config.rpcUrl,
      rpcSubscriptionsUrl: config.rpcSubscriptionsUrl,
      indexerApiEndpoint: config.indexerApiEndpoint ?? DEFAULT_DEVNET_INDEXER_ENDPOINT,
      deferMasterSeedSignature: true,
    } as any);
  }

  async getDevnetRelayerInfo(relayerApiEndpoint = DEFAULT_DEVNET_RELAYER_ENDPOINT) {
    const relayer = getUmbraRelayer({ apiEndpoint: relayerApiEndpoint });
    return relayer.getSupportedMints();
  }

  async assertDevnetMintSupported(mint: string, relayerApiEndpoint?: string) {
    const info = await this.getDevnetRelayerInfo(relayerApiEndpoint);
    if (!info.mints.includes(mint)) {
      throw new Error(`Umbra devnet relayer does not support mint ${mint}`);
    }
    return info;
  }

  async registerDevnetConfidentialUser(config: UmbraFundedDevnetConfig) {
    const client = await this.createFundedDevnetClient(config);
    const register = getUserRegistrationFunction({ client });
    const signatures = await register({
      confidential: true,
      anonymous: false,
      accountInfoCommitment: "confirmed",
    });

    return {
      address: String(client.signer.address),
      signatures: signatures.map(String),
    };
  }

  async depositPublicToEncryptedBalance(config: UmbraFundedDevnetConfig, input: { mint: string; amount: bigint }) {
    await this.assertDevnetMintSupported(input.mint, config.relayerApiEndpoint);
    const client = await this.createFundedDevnetClient(config);
    const deposit = getPublicBalanceToEncryptedBalanceDirectDepositorFunction(
      { client },
      { arcium: { awaitComputationFinalization: { timeoutMs: 240000, pollingIntervalMs: 3000 } } } as any
    );

    return deposit(String(client.signer.address), input.mint, input.amount, {
      accountInfoCommitment: "confirmed",
      epochInfoCommitment: "confirmed",
    } as any);
  }

  async queryEncryptedBalance(config: UmbraFundedDevnetConfig, mint: string) {
    const client = await this.createFundedDevnetClient(config);
    const query = getEncryptedBalanceQuerierFunction({ client });
    const balances = await query([mint], { accountInfoCommitment: "confirmed" } as any);
    return balances.get(mint);
  }

  async withdrawEncryptedToPublicBalance(config: UmbraFundedDevnetConfig, input: { mint: string; amount: bigint }) {
    await this.assertDevnetMintSupported(input.mint, config.relayerApiEndpoint);
    const client = await this.createFundedDevnetClient(config);
    const withdraw = getEncryptedBalanceToPublicBalanceDirectWithdrawerFunction(
      { client },
      { arcium: { awaitComputationFinalization: { timeoutMs: 240000, pollingIntervalMs: 3000 } } } as any
    );

    return withdraw(String(client.signer.address), input.mint, input.amount, {
      accountInfoCommitment: "confirmed",
    } as any);
  }

  buildShieldedPayoutHandoff(input: {
    network: "devnet" | "mainnet";
    recipient: string;
    mint: string;
    amount: string;
    memo: string;
  }): SponsorHandoffPayload {
    if (!input.recipient || !input.mint || !input.amount) {
      throw new Error("Umbra shielded payout handoff requires recipient, mint, and amount");
    }

    return {
      provider: "umbraShieldedPayout",
      payloadHash: sha256Bytes({
        network: input.network,
        recipient: input.recipient,
        mint: input.mint,
        amount: input.amount,
        memo: input.memo,
      }),
      referenceHash: textHash(`umbra:${input.network}:${input.recipient}:${input.mint}`),
      rawPayload: input,
    };
  }
}
