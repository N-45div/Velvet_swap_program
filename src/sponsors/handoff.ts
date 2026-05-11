import { createHash } from "crypto";

export type SponsorHandoffProvider = "umbraShieldedPayout" | "magicBlockPrivatePayment";

export type SponsorHandoffPayload = {
  provider: SponsorHandoffProvider;
  payloadHash: number[];
  referenceHash: number[];
  rawPayload: unknown;
};

function stableStringify(value: unknown): string {
  if (value === null || typeof value !== "object") {
    return JSON.stringify(value);
  }

  if (Array.isArray(value)) {
    return `[${value.map((item) => stableStringify(item)).join(",")}]`;
  }

  const entries = Object.entries(value as Record<string, unknown>).sort(([a], [b]) =>
    a.localeCompare(b)
  );
  return `{${entries
    .map(([key, item]) => `${JSON.stringify(key)}:${stableStringify(item)}`)
    .join(",")}}`;
}

export function sha256Bytes(value: unknown): number[] {
  return Array.from(createHash("sha256").update(stableStringify(value)).digest());
}

export function textHash(value: string): number[] {
  return Array.from(createHash("sha256").update(value).digest());
}

