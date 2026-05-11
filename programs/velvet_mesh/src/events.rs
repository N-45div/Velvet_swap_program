use anchor_lang::prelude::*;

use crate::state::{ComputeProvider, SettlementProvider, SettlementRoute};

#[event]
pub struct IntentCreated {
    pub intent: Pubkey,
    pub owner: Pubkey,
    pub compute_provider: ComputeProvider,
    pub allowed_routes: u8,
    pub settlement_verifier: Pubkey,
}

#[event]
pub struct QuoteSubmitted {
    pub intent: Pubkey,
    pub quote: Pubkey,
    pub maker: Pubkey,
    pub route: SettlementRoute,
}

#[event]
pub struct PrivateMatchRequested {
    pub intent: Pubkey,
    pub arcium_computation: [u8; 32],
}

#[event]
pub struct PrivateMatchReady {
    pub intent: Pubkey,
    pub selected_quote: Pubkey,
    pub route: SettlementRoute,
    pub arcium_computation: [u8; 32],
    pub selected_quote_commitment: [u8; 32],
}

#[event]
pub struct QuoteAccepted {
    pub intent: Pubkey,
    pub quote: Pubkey,
    pub accepted_match: Pubkey,
    pub route: SettlementRoute,
    pub settlement_verifier: Pubkey,
}

#[event]
pub struct SettlementHandoffPrepared {
    pub accepted_match: Pubkey,
    pub provider: SettlementProvider,
    pub payload_hash: [u8; 32],
    pub reference_hash: [u8; 32],
    pub settlement_verifier: Pubkey,
}

#[event]
pub struct SettlementReady {
    pub accepted_match: Pubkey,
    pub provider: SettlementProvider,
    pub settlement_hash: [u8; 32],
    pub payload_hash: [u8; 32],
    pub reference_hash: [u8; 32],
}

#[event]
pub struct IntentCancelled {
    pub intent: Pubkey,
}
