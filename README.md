# Velvet Swap Program

Confidential AMM on Solana powered by **Inco Lightning** encrypted math and **MagicBlock PER** permissioned execution.

## Overview

This repo contains the on-chain programs and tests for the confidential swap flow used by the VelvetRope frontend.
All pool reserves and swap amounts are stored as `Euint128` ciphertexts and never revealed in plaintext.

## Programs

- **private_swap_programs** — Confidential AMM (swap, add/remove liquidity)
- **inco_token** — Confidential SPL token balances and transfers
- **inco_lightning** — Encrypted math engine

Program IDs (see `Anchor.toml`):

```
private_swap_programs = 6L8awnTc179Atp7sMharQ8uuBjiKjWxzfEns6qW4fkyF
inco_token            = HmBw1FN2fXbgqyGpjB268vggBEEymNx98cuPpZQPYDZc
inco_lightning        = 5sjEbPiqgZrYwR31ahR6Uk9wf5awoX61YGg7jExQSwaj
```

## Repository Layout

```
programs/
├── private_swap_programs/   # Confidential AMM (Anchor)
├── inco-token/              # Confidential SPL program
└── inco-lightning/          # Inco encrypted math helpers

tests/
└── private_swap_programs.ts # End-to-end flow tests
```

## Setup & Test

```bash
# Install deps
npm install

# Run Anchor tests
anchor test
```

## Confidential Swap Flow (High Level)

1. **Create confidential mints** for token A/B.
2. **Create swap accounts** (user + pool token accounts, pool PDA).
3. **Initialize pool** with encrypted reserves.
4. **Mint seed balances** into user accounts (optional / dev only).
5. **Enable MagicBlock PER** for pool + user token PDAs.
6. **Add liquidity** using encrypted amounts.
7. **Swap exact in** with ciphertext inputs and encrypted math.

## Important Notes

- **PER delegation must occur _after_ minting** or the PDA ownership checks will fail.
- All transfers between user and pool use `inco_token::transfer` with encrypted amounts.
- Reference test flow in `tests/private_swap_programs.ts` for the correct ordering.

## Related

- VelvetRope frontend: `/home/divij/vincent/velvet-rope`
- Inco Lightning: https://github.com/Inco-fhevm/inco-solana-programs
- MagicBlock PER: https://docs.magicblock.gg
