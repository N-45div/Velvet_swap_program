use anchor_lang::prelude::*;

use crate::{
    errors::ErrorCode,
    events::PrivateMatchReady,
    state::{route_allowed, ComputeProvider, Intent, IntentStatus, Quote, SettlementRoute},
};

#[derive(Accounts)]
pub struct RecordPrivateMatch<'info> {
    pub match_verifier: Signer<'info>,
    #[account(mut, has_one = match_verifier)]
    pub intent: Account<'info, Intent>,
    #[account(constraint = selected_quote.intent == intent.key() @ ErrorCode::QuoteIntentMismatch)]
    pub selected_quote: Account<'info, Quote>,
}

pub fn handler(
    ctx: Context<RecordPrivateMatch>,
    arcium_computation: [u8; 32],
    selected_quote_commitment: [u8; 32],
    selected_route: SettlementRoute,
) -> Result<()> {
    let now = Clock::get()?.unix_timestamp;
    let intent = &mut ctx.accounts.intent;
    let selected_quote = &ctx.accounts.selected_quote;

    require!(
        intent.compute_provider == ComputeProvider::Arcium,
        ErrorCode::InvalidComputeProvider
    );
    require!(
        intent.status == IntentStatus::ComputationRequested,
        ErrorCode::ComputationNotRequested
    );
    require!(intent.expires_at > now, ErrorCode::IntentExpired);
    require!(selected_quote.expires_at > now, ErrorCode::QuoteExpired);
    require!(
        intent.arcium_computation == arcium_computation,
        ErrorCode::ComputationMismatch
    );
    require!(
        selected_quote.quote_commitment == selected_quote_commitment,
        ErrorCode::QuoteMismatch
    );
    require!(
        selected_quote.route == selected_route,
        ErrorCode::RouteMismatch
    );
    require!(
        route_allowed(intent.allowed_routes, selected_route),
        ErrorCode::RouteNotAllowed
    );

    intent.status = IntentStatus::MatchReady;
    intent.selected_quote = selected_quote.key();

    emit!(PrivateMatchReady {
        intent: intent.key(),
        selected_quote: selected_quote.key(),
        route: selected_route,
        arcium_computation,
        selected_quote_commitment,
    });

    Ok(())
}
