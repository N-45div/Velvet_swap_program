# Velvet Swap Program Architecture

## Overview

`private_swap_programs` implements a confidential AMM on Solana using:

- **Inco Lightning** for encrypted math (`Euint128` operations).
- **inco_token** for confidential SPL balances and transfers.
- **MagicBlock PER** to permission pool + token PDA updates.

## System Diagram

```mermaid
flowchart TB
    subgraph Client["Client/Test Harness"]
        TESTS[Anchor tests]
        SDK[Frontend helpers]
    end

    subgraph Programs["On-Chain Programs"]
        AMM[private_swap_programs]
        INCO[inco_lightning]
        TOKEN[inco_token]
        PER[permissioning program]
    end

    TESTS --> AMM
    SDK --> AMM
    AMM --> INCO
    AMM --> TOKEN
    AMM --> PER
```

## Core Accounts

- **Pool PDA**: stores encrypted reserves and fee config.
- **User Token A/B PDA**: confidential balances for swapper.
- **Pool Token A/B PDA**: confidential balances owned by pool PDA.
- **Permission PDAs**: MagicBlock PER permission state per token PDA.

## Instruction Flow

```mermaid
sequenceDiagram
    participant User
    participant Wallet
    participant AMM
    participant INCO
    participant TOKEN
    participant PER

    User->>Wallet: connect + sign
    Wallet->>AMM: initialize_pool
    AMM->>INCO: create encrypted reserves
    AMM->>TOKEN: create token PDAs

    Wallet->>AMM: add_liquidity
    AMM->>TOKEN: inco_transfer (user -> pool)

    Wallet->>AMM: swap_exact_in
    AMM->>INCO: encrypted math
    AMM->>TOKEN: inco_transfer (user -> pool -> user)
```

## Permissioning Rules

- **PER required for devnet** (ephemeral RPC). Pool + token PDAs must be permissioned.
- **Delegate AFTER minting**. Delegating a token PDA before minting can cause ownership errors.

## File Map

```
programs/
├── private_swap_programs/src/   # AMM logic + CPI to inco_token
├── inco-token/src/              # confidential SPL program
└── inco-lightning/              # encrypted math helpers

tests/private_swap_programs.ts  # end-to-end flow
```

## Integration Notes

- Frontend helpers mirror the test sequence for pool init + liquidity.
- `swap_exact_in` expects ciphertext inputs and validates invariant on encrypted values.
