use anchor_lang::prelude::*;

use crate::{
    errors::ErrorCode,
    events::IntentCreated,
    state::{CreateIntentParams, Intent, IntentStatus, MAX_QUOTES_PER_INTENT},
};

#[derive(Accounts)]
#[instruction(intent_nonce: u64)]
pub struct CreateIntent<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,
    #[account(
        init,
        payer = owner,
        space = Intent::SPACE,
        seeds = [b"intent", owner.key().as_ref(), &intent_nonce.to_le_bytes()],
        bump
    )]
    pub intent: Account<'info, Intent>,
    pub system_program: Program<'info, System>,
}

pub fn handler(
    ctx: Context<CreateIntent>,
    intent_nonce: u64,
    params: CreateIntentParams,
) -> Result<()> {
    let now = Clock::get()?.unix_timestamp;

    require!(
        params.input_mint != params.output_mint,
        ErrorCode::InvalidAssetPair
    );
    require!(params.allowed_routes != 0, ErrorCode::NoSettlementRoutes);
    require!(params.expires_at > now, ErrorCode::IntentExpired);
    require!(
        params.compute_provider != crate::state::ComputeProvider::Arcium
            || params.match_verifier != Pubkey::default(),
        ErrorCode::InvalidMatchVerifier
    );
    require!(
        params.settlement_verifier != Pubkey::default(),
        ErrorCode::InvalidSettlementVerifier
    );
    require!(
        params.min_quote_count > 0 && params.min_quote_count <= MAX_QUOTES_PER_INTENT,
        ErrorCode::InvalidQuoteCount
    );

    let intent = &mut ctx.accounts.intent;
    intent.owner = ctx.accounts.owner.key();
    intent.intent_nonce = intent_nonce;
    intent.input_mint = params.input_mint;
    intent.output_mint = params.output_mint;
    intent.encrypted_size = params.encrypted_size;
    intent.encrypted_limit_price = params.encrypted_limit_price;
    intent.encrypted_slippage_bps = params.encrypted_slippage_bps;
    intent.encrypted_risk_preference = params.encrypted_risk_preference;
    intent.allowed_routes = params.allowed_routes;
    intent.compute_provider = params.compute_provider;
    intent.match_verifier = params.match_verifier;
    intent.settlement_verifier = params.settlement_verifier;
    intent.status = IntentStatus::Open;
    intent.min_quote_count = params.min_quote_count;
    intent.quote_count = 0;
    intent.selected_quote = Pubkey::default();
    intent.accepted_match = Pubkey::default();
    intent.arcium_computation = [0; 32];
    intent.metadata_hash = params.metadata_hash;
    intent.created_at = now;
    intent.expires_at = params.expires_at;
    intent.bump = ctx.bumps.intent;

    emit!(IntentCreated {
        intent: intent.key(),
        owner: intent.owner,
        compute_provider: intent.compute_provider,
        allowed_routes: intent.allowed_routes,
        settlement_verifier: intent.settlement_verifier,
    });

    Ok(())
}
