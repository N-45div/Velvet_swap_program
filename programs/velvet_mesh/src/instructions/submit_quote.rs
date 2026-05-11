use anchor_lang::prelude::*;

use crate::{
    errors::ErrorCode,
    events::QuoteSubmitted,
    state::{route_allowed, Intent, IntentStatus, Quote, SubmitQuoteParams, MAX_QUOTES_PER_INTENT},
};

#[derive(Accounts)]
pub struct SubmitQuote<'info> {
    #[account(mut)]
    pub maker: Signer<'info>,
    #[account(mut)]
    pub intent: Account<'info, Intent>,
    #[account(
        init,
        payer = maker,
        space = Quote::SPACE,
        seeds = [b"quote", intent.key().as_ref(), maker.key().as_ref()],
        bump
    )]
    pub quote: Account<'info, Quote>,
    pub system_program: Program<'info, System>,
}

pub fn handler(ctx: Context<SubmitQuote>, params: SubmitQuoteParams) -> Result<()> {
    let now = Clock::get()?.unix_timestamp;
    let intent = &mut ctx.accounts.intent;

    require!(
        intent.status == IntentStatus::Open,
        ErrorCode::IntentNotOpen
    );
    require!(intent.expires_at > now, ErrorCode::IntentExpired);
    require!(params.expires_at > now, ErrorCode::QuoteExpired);
    require!(
        route_allowed(intent.allowed_routes, params.route),
        ErrorCode::RouteNotAllowed
    );
    require!(
        intent.quote_count < MAX_QUOTES_PER_INTENT,
        ErrorCode::TooManyQuotes
    );

    let quote = &mut ctx.accounts.quote;
    quote.intent = intent.key();
    quote.maker = ctx.accounts.maker.key();
    quote.route = params.route;
    quote.encrypted_output_amount = params.encrypted_output_amount;
    quote.encrypted_price_bps = params.encrypted_price_bps;
    quote.encrypted_maker_risk = params.encrypted_maker_risk;
    quote.quote_commitment = params.quote_commitment;
    quote.settlement_hash = params.settlement_hash;
    quote.created_at = now;
    quote.expires_at = params.expires_at;
    quote.accepted = false;
    quote.bump = ctx.bumps.quote;

    intent.quote_count = intent
        .quote_count
        .checked_add(1)
        .ok_or(ErrorCode::MathOverflow)?;

    emit!(QuoteSubmitted {
        intent: intent.key(),
        quote: quote.key(),
        maker: quote.maker,
        route: quote.route,
    });

    Ok(())
}
