use anchor_lang::prelude::*;

pub const MAX_QUOTES_PER_INTENT: u8 = 16;

#[account]
pub struct Intent {
    pub owner: Pubkey,
    pub intent_nonce: u64,
    pub input_mint: Pubkey,
    pub output_mint: Pubkey,
    pub encrypted_size: [u8; 32],
    pub encrypted_limit_price: [u8; 32],
    pub encrypted_slippage_bps: [u8; 32],
    pub encrypted_risk_preference: [u8; 32],
    pub allowed_routes: u8,
    pub compute_provider: ComputeProvider,
    pub match_verifier: Pubkey,
    pub settlement_verifier: Pubkey,
    pub status: IntentStatus,
    pub min_quote_count: u8,
    pub quote_count: u8,
    pub selected_quote: Pubkey,
    pub accepted_match: Pubkey,
    pub arcium_computation: [u8; 32],
    pub metadata_hash: [u8; 32],
    pub created_at: i64,
    pub expires_at: i64,
    pub bump: u8,
}

impl Intent {
    pub const SPACE: usize = 8
        + 32
        + 8
        + 32
        + 32
        + 32
        + 32
        + 32
        + 32
        + 1
        + 1
        + 32
        + 32
        + 1
        + 1
        + 1
        + 32
        + 32
        + 32
        + 32
        + 8
        + 8
        + 1;
}

#[account]
pub struct Quote {
    pub intent: Pubkey,
    pub maker: Pubkey,
    pub route: SettlementRoute,
    pub encrypted_output_amount: [u8; 32],
    pub encrypted_price_bps: [u8; 32],
    pub encrypted_maker_risk: [u8; 32],
    pub quote_commitment: [u8; 32],
    pub settlement_hash: [u8; 32],
    pub created_at: i64,
    pub expires_at: i64,
    pub accepted: bool,
    pub bump: u8,
}

impl Quote {
    pub const SPACE: usize = 8 + 32 + 32 + 1 + 32 + 32 + 32 + 32 + 32 + 8 + 8 + 1 + 1;
}

#[account]
pub struct AcceptedMatch {
    pub intent: Pubkey,
    pub quote: Pubkey,
    pub owner: Pubkey,
    pub maker: Pubkey,
    pub route: SettlementRoute,
    pub settlement_verifier: Pubkey,
    pub settlement_provider: SettlementProvider,
    pub settlement_hash: [u8; 32],
    pub settlement_payload_hash: [u8; 32],
    pub settlement_reference_hash: [u8; 32],
    pub arcium_computation: [u8; 32],
    pub accepted_at: i64,
    pub settlement_prepared_at: i64,
    pub settlement_confirmed_at: i64,
    pub settlement_ready: bool,
    pub bump: u8,
}

impl AcceptedMatch {
    pub const SPACE: usize =
        8 + 32 + 32 + 32 + 32 + 1 + 32 + 1 + 32 + 32 + 32 + 32 + 8 + 8 + 8 + 1 + 1;
}

#[account]
pub struct SettlementHandoff {
    pub accepted_match: Pubkey,
    pub owner: Pubkey,
    pub maker: Pubkey,
    pub settlement_verifier: Pubkey,
    pub provider: SettlementProvider,
    pub payload_hash: [u8; 32],
    pub reference_hash: [u8; 32],
    pub status: SettlementHandoffStatus,
    pub prepared_at: i64,
    pub confirmed_at: i64,
    pub bump: u8,
}

impl SettlementHandoff {
    pub const SPACE: usize = 8 + 32 + 32 + 32 + 32 + 1 + 32 + 32 + 1 + 8 + 8 + 1;
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq)]
pub enum ComputeProvider {
    None,
    Arcium,
    Encrypt,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq)]
pub enum IntentStatus {
    Open,
    ComputationRequested,
    MatchReady,
    Accepted,
    Cancelled,
    Expired,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq)]
pub enum SettlementRoute {
    DirectSolanaP2p,
    VelvetSwapFallback,
    IkaBridgeless,
    JupiterFallback,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq)]
pub enum SettlementProvider {
    DirectSolana,
    VelvetSwapFallback,
    UmbraShieldedPayout,
    MagicBlockPrivatePayment,
    JupiterFallback,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq)]
pub enum SettlementHandoffStatus {
    Prepared,
    Confirmed,
}

impl SettlementRoute {
    pub fn bit(self) -> u8 {
        match self {
            SettlementRoute::DirectSolanaP2p => 1 << 0,
            SettlementRoute::VelvetSwapFallback => 1 << 1,
            SettlementRoute::IkaBridgeless => 1 << 2,
            SettlementRoute::JupiterFallback => 1 << 3,
        }
    }
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct CreateIntentParams {
    pub input_mint: Pubkey,
    pub output_mint: Pubkey,
    pub encrypted_size: [u8; 32],
    pub encrypted_limit_price: [u8; 32],
    pub encrypted_slippage_bps: [u8; 32],
    pub encrypted_risk_preference: [u8; 32],
    pub allowed_routes: u8,
    pub compute_provider: ComputeProvider,
    pub match_verifier: Pubkey,
    pub settlement_verifier: Pubkey,
    pub min_quote_count: u8,
    pub metadata_hash: [u8; 32],
    pub expires_at: i64,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct SubmitQuoteParams {
    pub route: SettlementRoute,
    pub encrypted_output_amount: [u8; 32],
    pub encrypted_price_bps: [u8; 32],
    pub encrypted_maker_risk: [u8; 32],
    pub quote_commitment: [u8; 32],
    pub settlement_hash: [u8; 32],
    pub expires_at: i64,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct PrepareSettlementHandoffParams {
    pub provider: SettlementProvider,
    pub payload_hash: [u8; 32],
    pub reference_hash: [u8; 32],
}

pub fn route_allowed(allowed_routes: u8, route: SettlementRoute) -> bool {
    allowed_routes & route.bit() != 0
}
