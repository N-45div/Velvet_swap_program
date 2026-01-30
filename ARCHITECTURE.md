# VelvetSwap Architecture

> Technical deep-dive into the confidential AMM implementation.

---

## System Overview

```mermaid
graph TB
    subgraph "Client Layer"
        UI["VelvetSwap Frontend<br/>(Next.js)"]
        SDK["Swap Client SDK<br/>(TypeScript)"]
    end

    subgraph "Privacy Layer"
        INCO["Inco Lightning<br/>FHE Operations"]
        LIGHT["Light Protocol V2<br/>ZK Compressed Accounts"]
        PER["MagicBlock PER<br/>TEE Execution"]
    end

    subgraph "Solana Runtime"
        PROGRAM["light_swap_psp<br/>Confidential AMM"]
        POOL[("SwapPool<br/>(Compressed)")]
    end

    UI --> SDK
    SDK --> PER
    PER --> PROGRAM
    PROGRAM --> INCO
    PROGRAM --> LIGHT
    PROGRAM --> POOL

    style UI fill:#7C3AED,color:#fff
    style SDK fill:#7C3AED,color:#fff
    style INCO fill:#22C55E,color:#fff
    style LIGHT fill:#3B82F6,color:#fff
    style PER fill:#F59E0B,color:#fff
    style PROGRAM fill:#9945FF,color:#fff
    style POOL fill:#1e1e2e,color:#fff,stroke:#9945FF
```

---

## Core Components

### 1. Pool State (Compressed Account)

The pool state is stored as a **Light Protocol compressed account** with FHE-encrypted fields:

```mermaid
erDiagram
    SwapPool {
        Pubkey authority "Pool admin (can add/remove liquidity)"
        Pubkey pool_authority "PDA for signing transfers"
        Pubkey mint_a "Token A mint address"
        Pubkey mint_b "Token B mint address"
        Euint128 reserve_a "ENCRYPTED: Token A reserves"
        Euint128 reserve_b "ENCRYPTED: Token B reserves"
        Euint128 protocol_fee_a "ENCRYPTED: Accumulated fees (A)"
        Euint128 protocol_fee_b "ENCRYPTED: Accumulated fees (B)"
        u16 fee_bps "Fee in basis points (e.g., 30 = 0.3%)"
        bool is_paused "Emergency pause flag"
        i64 last_update_ts "Last state update timestamp"
    }
```

### 2. Pool Authority PDA

Derived deterministically for each token pair:

```
seeds = ["pool_authority", mint_a, mint_b]
pool_authority_pda = PDA(seeds, program_id)
```

This PDA is **delegated to MagicBlock TEE** for private execution.

### 3. Compressed Account Address

Pool address is derived using Light Protocol V2:

```
seeds = ["pool", mint_a, mint_b]
address_seed = deriveAddressSeedV2(seeds)
pool_address = deriveAddressV2(address_seed, batch_address_tree, program_id)
```

---

## Instruction Flow

### Initialize Pool

```mermaid
sequenceDiagram
    participant Client
    participant LightRPC as Light RPC
    participant Program as VelvetSwap
    participant Inco as Inco Lightning
    participant Light as Light Protocol

    Client->>LightRPC: getValidityProofV0([], [new_address])
    LightRPC-->>Client: validity_proof, root_indices
    
    Client->>Program: initialize_pool(proof, mint_a, mint_b, fee_bps)
    Program->>Inco: as_euint128(0) × 4
    Note over Program,Inco: Initialize encrypted reserves & fees to zero
    
    Program->>Light: Create compressed account
    Light-->>Program: Account created at derived address
    Program-->>Client: Success
```

### Swap Exact In

```mermaid
sequenceDiagram
    participant Client
    participant TEE as MagicBlock TEE
    participant Program as VelvetSwap
    participant Inco as Inco Lightning
    participant Light as Light Protocol

    Client->>Client: Encrypt amounts (FHE)
    Client->>TEE: Submit swap via PER endpoint
    TEE->>Program: swap_exact_in(proof, pool_meta, ciphertexts, a_to_b)
    
    Program->>Program: Load pool state from pool_data
    
    rect rgb(50, 50, 80)
        Note over Program,Inco: FHE Computation (all encrypted)
        Program->>Inco: new_euint128(amount_in_ciphertext)
        Program->>Inco: new_euint128(amount_out_ciphertext)
        Program->>Inco: new_euint128(fee_amount_ciphertext)
        
        Program->>Inco: e_ge(reserve_out, amount_out)
        Note over Inco: Check: has_liquidity?
        
        Program->>Inco: e_mul(reserve_in, reserve_out)
        Note over Inco: old_k = x * y
        
        Program->>Inco: e_add(reserve_in, amount_in)
        Program->>Inco: e_sub(reserve_out, amount_out)
        
        Program->>Inco: e_mul(new_reserve_in, new_reserve_out)
        Note over Inco: new_k = x' * y'
        
        Program->>Inco: e_ge(new_k, old_k)
        Note over Inco: Verify: new_k >= old_k
        
        Program->>Inco: e_select(k_ok, amount, zero)
        Note over Inco: Zero out if invariant violated
    end
    
    Program->>Light: Update compressed pool state
    Light-->>Program: New validity proof
    Program-->>TEE: Swap complete
    TEE-->>Client: Transaction signature
```

### Add/Remove Liquidity

```mermaid
sequenceDiagram
    participant Authority
    participant Program as VelvetSwap
    participant Inco as Inco Lightning
    participant Light as Light Protocol

    Authority->>Program: add_liquidity(proof, pool_meta, amount_a, amount_b)
    
    Program->>Program: Verify authority == pool.authority
    Program->>Program: Verify !pool.is_paused
    
    Program->>Inco: new_euint128(amount_a_ciphertext)
    Program->>Inco: new_euint128(amount_b_ciphertext)
    
    Program->>Inco: e_add(reserve_a, amount_a)
    Program->>Inco: e_add(reserve_b, amount_b)
    
    Program->>Light: Update compressed pool state
    Light-->>Program: Success
    Program-->>Authority: Liquidity added
```

---

## FHE Operations Detail

### Constant Product AMM Math

The swap uses the standard `x * y = k` invariant, but **entirely on encrypted values**:

```mermaid
flowchart LR
    subgraph "Input (Encrypted)"
        AI["amount_in<br/>Euint128"]
        AO["amount_out<br/>Euint128"]
        FEE["fee_amount<br/>Euint128"]
    end

    subgraph "Pool State (Encrypted)"
        RA["reserve_a<br/>Euint128"]
        RB["reserve_b<br/>Euint128"]
    end

    subgraph "FHE Operations"
        CHK1["e_ge(reserve_out, amount_out)<br/>Liquidity check"]
        MUL1["e_mul(reserve_in, reserve_out)<br/>old_k"]
        ADD["e_add(reserve_in, amount_in)<br/>new_reserve_in"]
        SUB["e_sub(reserve_out, amount_out)<br/>new_reserve_out"]
        MUL2["e_mul(new_in, new_out)<br/>new_k"]
        CHK2["e_ge(new_k, old_k)<br/>Invariant check"]
        SEL["e_select(valid, amount, 0)<br/>Zero if invalid"]
    end

    AI --> ADD
    AO --> SUB
    RA --> MUL1
    RB --> MUL1
    RA --> ADD
    RB --> SUB
    MUL1 --> CHK2
    ADD --> MUL2
    SUB --> MUL2
    MUL2 --> CHK2
    CHK1 --> SEL
    CHK2 --> SEL

    style AI fill:#7C3AED,color:#fff
    style AO fill:#7C3AED,color:#fff
    style FEE fill:#7C3AED,color:#fff
    style RA fill:#22C55E,color:#fff
    style RB fill:#22C55E,color:#fff
```

### Operation Complexity

| Operation | Inco CPI Calls | Purpose |
|-----------|----------------|---------|
| `new_euint128` | 3 | Parse input ciphertexts |
| `as_euint128` | 1 | Create zero constant |
| `e_ge` | 2 | Liquidity + invariant checks |
| `e_add` | 2 | Update reserves |
| `e_sub` | 1 | Update output reserve |
| `e_mul` | 2 | Compute k values |
| `e_select` | 3 | Conditional zeroing |
| **Total** | **14** | Per swap |

---

## MagicBlock PER Integration

### Permission Setup

```mermaid
flowchart TB
    subgraph "Setup Phase"
        A[Create Permission] --> B[Delegate PDA to TEE]
        B --> C[Wait for Permission Active]
    end

    subgraph "Execution Phase"
        D[Client submits to TEE endpoint] --> E[TEE validates permission]
        E --> F[Execute in isolated environment]
        F --> G[State updates committed]
    end

    C --> D

    style A fill:#F59E0B,color:#fff
    style B fill:#F59E0B,color:#fff
    style E fill:#F59E0B,color:#fff
    style F fill:#F59E0B,color:#fff
```

### Permission Members

```rust
members: [
    { flags: AUTHORITY | TX_LOGS | TX_BALANCES | TX_MESSAGE | ACCOUNT_SIGNATURES, pubkey: authority },
    { flags: AUTHORITY | TX_LOGS | TX_BALANCES | TX_MESSAGE | ACCOUNT_SIGNATURES, pubkey: pool_authority_pda },
    { flags: AUTHORITY | TX_LOGS | TX_BALANCES | TX_MESSAGE | ACCOUNT_SIGNATURES, pubkey: tee_validator },
    { flags: AUTHORITY | TX_LOGS | TX_BALANCES | TX_MESSAGE | ACCOUNT_SIGNATURES, pubkey: program_id },
]
```

---

## Light Protocol V2 Integration

### Compressed Account Flow

```mermaid
flowchart LR
    subgraph "State Tree"
        ROOT["Merkle Root"]
        LEAF["Pool Account<br/>(Compressed)"]
    end

    subgraph "Address Tree"
        ADDR["Batch Address Tree<br/>amt2kaJA14v3..."]
    end

    subgraph "Output Queue"
        QUEUE["Output Queue<br/>oq1na8gojfd..."]
    end

    ROOT --> LEAF
    ADDR --> LEAF
    LEAF --> QUEUE

    style ROOT fill:#3B82F6,color:#fff
    style LEAF fill:#3B82F6,color:#fff
    style ADDR fill:#3B82F6,color:#fff
    style QUEUE fill:#3B82F6,color:#fff
```

### Key Addresses (Devnet)

| Account | Address |
|---------|---------|
| Batch Address Tree | `amt2kaJA14v3urZbZvnc5v2np8jqvc4Z8zDep5wbtzx` |
| Output Queue | `oq1na8gojfdUhsfCpyjNt6h4JaDWtHf1yQj4koBWfto` |
| Light System Program | `SySTEM1eSU2p4BGQfQpimFEWWSC1XDFeun3Nqzz3rT7` |

---

## Error Handling

| Error | Code | Cause |
|-------|------|-------|
| `PoolPaused` | 6000 | Pool is in emergency pause state |
| `InvalidInputMint` | 6001 | Input token doesn't match pool |
| `InvalidOutputMint` | 6002 | Output token doesn't match pool |
| `InvalidPermissionAccount` | 6003 | PDA doesn't match derived address |
| `Unauthorized` | 6004 | Caller is not pool authority |

---

## Security Model

```mermaid
flowchart TB
    subgraph "Trust Assumptions"
        T1["Inco Lightning FHE<br/>Cryptographic security"]
        T2["Light Protocol ZK<br/>Proof soundness"]
        T3["MagicBlock TEE<br/>Hardware isolation"]
    end

    subgraph "Guarantees"
        G1["Amounts hidden from validators"]
        G2["Reserves hidden from indexers"]
        G3["Execution hidden from observers"]
    end

    T1 --> G1
    T1 --> G2
    T2 --> G2
    T3 --> G3

    style T1 fill:#22C55E,color:#fff
    style T2 fill:#3B82F6,color:#fff
    style T3 fill:#F59E0B,color:#fff
    style G1 fill:#7C3AED,color:#fff
    style G2 fill:#7C3AED,color:#fff
    style G3 fill:#7C3AED,color:#fff
```

---

## File Structure

```
programs/light_swap_psp/src/lib.rs
├── compute_swap_updates()     # FHE swap math (lines 29-106)
├── create_permission()        # PER permission setup (lines 113-143)
├── delegate_pda()             # TEE delegation (lines 146-160)
├── initialize_pool()          # Pool creation (lines 162-222)
├── add_liquidity()            # LP deposit (lines 224-275)
├── remove_liquidity()         # LP withdrawal (lines 277-328)
├── swap_exact_in()            # Core swap (lines 330-407)
├── Account structs            # Anchor contexts (lines 410-475)
├── SwapPool                   # Pool state struct (lines 477-497)
├── ErrorCode                  # Custom errors (lines 499-511)
└── AccountType + helpers      # PDA derivation (lines 513-526)
```

---

## Performance Characteristics

| Metric | Value | Notes |
|--------|-------|-------|
| Compute Units | ~800,000 | Per swap (FHE heavy) |
| Account Size | ~500 bytes | Compressed pool state |
| Validity Proof | ~1-2 seconds | Light RPC latency |
| TEE Overhead | ~500ms | PER execution |

---

## Future Improvements

1. **Multi-hop routing** — Chain multiple pools for better prices
2. **LP tokens** — Fungible representation of liquidity shares
3. **Attested reveals** — Allow users to prove their swap amounts
4. **Fee distribution** — Automated protocol fee collection
