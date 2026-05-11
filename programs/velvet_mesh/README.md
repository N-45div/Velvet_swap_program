# VelvetMesh Program

Rust Anchor program for the private intent/RFQ layer.

This program preserves `programs/light_swap_psp` as the existing VelvetSwap
confidential AMM fallback and adds a separate state machine for:

- private intent creation
- maker quote submission
- confidential-compute match requests
- accepted match routing

The first implementation stores encrypted preference and quote values as opaque
32-byte handles/commitments. The Arcium circuit integration should consume those
handles through the computation request path and write verified match results
back through a real Arcium callback path.

There is intentionally no mock instruction for recording a winning Arcium quote.
Arcium-backed intents cannot be accepted until the callback verifies
`SignedComputationOutputs` from the Arcium program.
