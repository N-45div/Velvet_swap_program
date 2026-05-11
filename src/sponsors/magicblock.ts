import { Transaction, VersionedTransaction } from "@solana/web3.js";
import { SponsorHandoffPayload, sha256Bytes, textHash } from "./handoff";

export type MagicBlockTransferRequest = {
  from: string;
  to: string;
  mint: string;
  amount: number;
  visibility: "public" | "private";
  fromBalance: "base" | "ephemeral";
  toBalance: "base" | "ephemeral";
  cluster?: "mainnet" | "devnet" | string;
  initIfMissing?: boolean;
  initAtasIfMissing?: boolean;
  initVaultIfMissing?: boolean;
  memo?: string;
  minDelayMs?: string;
  maxDelayMs?: string;
  clientRefId?: string;
  split?: number;
  gasless?: boolean;
  legacy?: boolean;
};

export type MagicBlockTransferResponse = {
  kind: "transfer";
  version: "legacy" | "v0";
  transactionBase64: string;
  sendTo: "base" | "ephemeral";
  recentBlockhash: string;
  lastValidBlockHeight: number;
  instructionCount: number;
  requiredSigners: string[];
  validator?: string;
};

export class MagicBlockPrivatePaymentsAdapter {
  constructor(
    private readonly baseUrl = "https://payments.magicblock.app",
    private readonly bearerToken?: string
  ) {}

  async buildPrivateTransfer(
    request: MagicBlockTransferRequest
  ): Promise<SponsorHandoffPayload & { response: MagicBlockTransferResponse }> {
    if (request.visibility !== "private") {
      throw new Error("MagicBlock sponsor path must use private visibility");
    }

    const response = await this.postJsonWithRetry(`${this.baseUrl}/v1/spl/transfer`, {
      method: "POST",
      headers: {
        "content-type": "application/json",
        ...(this.bearerToken ? { authorization: `Bearer ${this.bearerToken}` } : {}),
      },
      body: JSON.stringify(request),
    });

    const body = await response.json();
    if (!response.ok) {
      throw new Error(`MagicBlock transfer build failed: ${JSON.stringify(body)}`);
    }

    const transfer = body as MagicBlockTransferResponse;
    validateMagicBlockTransferResponse(transfer);

    return {
      provider: "magicBlockPrivatePayment",
      payloadHash: sha256Bytes(transfer),
      referenceHash: textHash(`${transfer.kind}:${transfer.recentBlockhash}:${transfer.lastValidBlockHeight}`),
      rawPayload: transfer,
      response: transfer,
    };
  }

  private async postJsonWithRetry(url: string, init: RequestInit): Promise<Response> {
    let lastError: unknown;
    for (let attempt = 0; attempt < 3; attempt += 1) {
      try {
        return await fetch(url, init);
      } catch (error) {
        lastError = error;
        await new Promise((resolve) => setTimeout(resolve, 250 * (attempt + 1)));
      }
    }

    throw lastError;
  }
}

export function validateMagicBlockTransferResponse(response: MagicBlockTransferResponse): void {
  if (response.kind !== "transfer") {
    throw new Error(`Unexpected MagicBlock response kind: ${response.kind}`);
  }
  if (response.version !== "legacy" && response.version !== "v0") {
    throw new Error(`Unexpected MagicBlock transaction version: ${response.version}`);
  }
  if (!response.transactionBase64) {
    throw new Error("MagicBlock response missing transactionBase64");
  }
  if (!Array.isArray(response.requiredSigners) || response.requiredSigners.length === 0) {
    throw new Error("MagicBlock response missing required signers");
  }

  const txBytes = Buffer.from(response.transactionBase64, "base64");
  if (response.version === "legacy") {
    Transaction.from(txBytes);
  } else {
    VersionedTransaction.deserialize(txBytes);
  }
}
