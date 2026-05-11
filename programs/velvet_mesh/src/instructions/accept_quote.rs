use anchor_lang::prelude::*;

use crate::{
    errors::ErrorCode,
    events::QuoteAccepted,
    state::{
        route_allowed, AcceptedMatch, ComputeProvider, Intent, IntentStatus, Quote,
        SettlementProvider,
    },
};

#[derive(Accounts)]
pub struct AcceptQuote<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,
    #[account(mut, has_one = owner)]
    pub intent: Account<'info, Intent>,
    #[account(mut, constraint = quote.intent == intent.key() @ ErrorCode::QuoteIntentMismatch)]
    pub quote: Account<'info, Quote>,
    #[account(
        init,
        payer = owner,
        space = AcceptedMatch::SPACE,
        seeds = [b"match", intent.key().as_ref()],
        bump
    )]
    pub accepted_match: Account<'info, AcceptedMatch>,
    pub system_program: Program<'info, System>,
}

pub fn handler(ctx: Context<AcceptQuote>) -> Result<()> {
    let now = Clock::get()?.unix_timestamp;
    let intent = &mut ctx.accounts.intent;
    let quote = &mut ctx.accounts.quote;
    let accepted_match = &mut ctx.accounts.accepted_match;

    let can_accept_direct =
        intent.compute_provider != ComputeProvider::Arcium && intent.status == IntentStatus::Open;
    let can_accept_verified_match = intent.status == IntentStatus::MatchReady;
    require!(
        can_accept_direct || can_accept_verified_match,
        ErrorCode::IntentNotAcceptable
    );
    require!(intent.expires_at > now, ErrorCode::IntentExpired);
    require!(quote.expires_at > now, ErrorCode::QuoteExpired);
    require!(quote.intent == intent.key(), ErrorCode::QuoteIntentMismatch);
    require!(
        route_allowed(intent.allowed_routes, quote.route),
        ErrorCode::RouteNotAllowed
    );

    if intent.status == IntentStatus::MatchReady {
        require!(
            intent.selected_quote == quote.key(),
            ErrorCode::QuoteMismatch
        );
    }

    quote.accepted = true;
    intent.status = IntentStatus::Accepted;
    intent.selected_quote = quote.key();
    intent.accepted_match = accepted_match.key();

    accepted_match.intent = intent.key();
    accepted_match.quote = quote.key();
    accepted_match.owner = intent.owner;
    accepted_match.maker = quote.maker;
    accepted_match.route = quote.route;
    accepted_match.settlement_verifier = intent.settlement_verifier;
    accepted_match.settlement_provider = SettlementProvider::DirectSolana;
    accepted_match.settlement_hash = quote.settlement_hash;
    accepted_match.settlement_payload_hash = [0; 32];
    accepted_match.settlement_reference_hash = [0; 32];
    accepted_match.arcium_computation = intent.arcium_computation;
    accepted_match.accepted_at = now;
    accepted_match.settlement_prepared_at = 0;
    accepted_match.settlement_confirmed_at = 0;
    accepted_match.settlement_ready = false;
    accepted_match.bump = ctx.bumps.accepted_match;

    emit!(QuoteAccepted {
        intent: intent.key(),
        quote: quote.key(),
        accepted_match: accepted_match.key(),
        route: accepted_match.route,
        settlement_verifier: accepted_match.settlement_verifier,
    });

    Ok(())
}
