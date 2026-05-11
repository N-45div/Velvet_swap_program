use anchor_lang::prelude::*;

#[error_code]
pub enum ErrorCode {
    #[msg("Input and output assets must differ")]
    InvalidAssetPair,
    #[msg("Intent must allow at least one settlement route")]
    NoSettlementRoutes,
    #[msg("Intent has expired")]
    IntentExpired,
    #[msg("Quote has expired")]
    QuoteExpired,
    #[msg("Invalid minimum quote count")]
    InvalidQuoteCount,
    #[msg("Intent is not open")]
    IntentNotOpen,
    #[msg("Settlement route is not allowed by the intent")]
    RouteNotAllowed,
    #[msg("Intent has too many quotes")]
    TooManyQuotes,
    #[msg("Math overflow")]
    MathOverflow,
    #[msg("Unauthorized")]
    Unauthorized,
    #[msg("Not enough quotes have been submitted")]
    NotEnoughQuotes,
    #[msg("Invalid compute provider for this instruction")]
    InvalidComputeProvider,
    #[msg("Computation has not been requested")]
    ComputationNotRequested,
    #[msg("Quote does not belong to intent")]
    QuoteIntentMismatch,
    #[msg("Private match selected a different quote")]
    QuoteMismatch,
    #[msg("Intent is not acceptable")]
    IntentNotAcceptable,
    #[msg("Intent is not cancellable")]
    IntentNotCancellable,
    #[msg("Arcium-backed intents require a non-default match verifier")]
    InvalidMatchVerifier,
    #[msg("Arcium computation does not match the requested computation")]
    ComputationMismatch,
    #[msg("Selected route does not match the quote route")]
    RouteMismatch,
    #[msg("Sponsor settlement handoff requires a non-default settlement verifier")]
    InvalidSettlementVerifier,
    #[msg("Settlement payload hash must be non-zero")]
    InvalidSettlementPayload,
    #[msg("Settlement handoff has already been prepared")]
    SettlementAlreadyPrepared,
    #[msg("Settlement handoff has already been confirmed")]
    SettlementAlreadyConfirmed,
    #[msg("Settlement handoff is not prepared")]
    SettlementNotPrepared,
}
