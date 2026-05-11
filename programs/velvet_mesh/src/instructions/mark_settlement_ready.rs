use anchor_lang::prelude::*;

use crate::{
    errors::ErrorCode,
    events::SettlementReady,
    state::{AcceptedMatch, SettlementHandoff, SettlementHandoffStatus},
};

#[derive(Accounts)]
pub struct MarkSettlementReady<'info> {
    pub settlement_verifier: Signer<'info>,
    #[account(mut, has_one = settlement_verifier)]
    pub accepted_match: Account<'info, AcceptedMatch>,
    #[account(
        mut,
        has_one = accepted_match,
        has_one = settlement_verifier,
        seeds = [b"settlement", accepted_match.key().as_ref()],
        bump = settlement_handoff.bump
    )]
    pub settlement_handoff: Account<'info, SettlementHandoff>,
}

pub fn handler(ctx: Context<MarkSettlementReady>) -> Result<()> {
    let accepted_match = &mut ctx.accounts.accepted_match;
    let settlement_handoff = &mut ctx.accounts.settlement_handoff;

    require!(
        accepted_match.settlement_verifier == ctx.accounts.settlement_verifier.key(),
        ErrorCode::Unauthorized
    );
    require!(
        !accepted_match.settlement_ready,
        ErrorCode::SettlementAlreadyConfirmed
    );
    require!(
        settlement_handoff.status == SettlementHandoffStatus::Prepared,
        ErrorCode::SettlementNotPrepared
    );
    require!(
        accepted_match.settlement_payload_hash == settlement_handoff.payload_hash,
        ErrorCode::InvalidSettlementPayload
    );

    let now = Clock::get()?.unix_timestamp;
    accepted_match.settlement_ready = true;
    accepted_match.settlement_confirmed_at = now;
    settlement_handoff.status = SettlementHandoffStatus::Confirmed;
    settlement_handoff.confirmed_at = now;

    emit!(SettlementReady {
        accepted_match: accepted_match.key(),
        provider: accepted_match.settlement_provider,
        settlement_hash: accepted_match.settlement_hash,
        payload_hash: accepted_match.settlement_payload_hash,
        reference_hash: accepted_match.settlement_reference_hash,
    });

    Ok(())
}
