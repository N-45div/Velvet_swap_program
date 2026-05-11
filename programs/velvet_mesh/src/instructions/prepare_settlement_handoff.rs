use anchor_lang::prelude::*;

use crate::{
    errors::ErrorCode,
    events::SettlementHandoffPrepared,
    state::{
        AcceptedMatch, PrepareSettlementHandoffParams, SettlementHandoff, SettlementHandoffStatus,
    },
};

#[derive(Accounts)]
pub struct PrepareSettlementHandoff<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,
    #[account(mut, has_one = owner)]
    pub accepted_match: Account<'info, AcceptedMatch>,
    #[account(
        init,
        payer = owner,
        space = SettlementHandoff::SPACE,
        seeds = [b"settlement", accepted_match.key().as_ref()],
        bump
    )]
    pub settlement_handoff: Account<'info, SettlementHandoff>,
    pub system_program: Program<'info, System>,
}

pub fn handler(
    ctx: Context<PrepareSettlementHandoff>,
    params: PrepareSettlementHandoffParams,
) -> Result<()> {
    let now = Clock::get()?.unix_timestamp;
    let accepted_match = &mut ctx.accounts.accepted_match;
    let settlement_handoff = &mut ctx.accounts.settlement_handoff;

    require!(
        !accepted_match.settlement_ready,
        ErrorCode::SettlementAlreadyConfirmed
    );
    require!(
        accepted_match.settlement_prepared_at == 0,
        ErrorCode::SettlementAlreadyPrepared
    );
    require!(
        params.payload_hash != [0; 32],
        ErrorCode::InvalidSettlementPayload
    );

    accepted_match.settlement_provider = params.provider;
    accepted_match.settlement_payload_hash = params.payload_hash;
    accepted_match.settlement_reference_hash = params.reference_hash;
    accepted_match.settlement_prepared_at = now;

    settlement_handoff.accepted_match = accepted_match.key();
    settlement_handoff.owner = accepted_match.owner;
    settlement_handoff.maker = accepted_match.maker;
    settlement_handoff.settlement_verifier = accepted_match.settlement_verifier;
    settlement_handoff.provider = params.provider;
    settlement_handoff.payload_hash = params.payload_hash;
    settlement_handoff.reference_hash = params.reference_hash;
    settlement_handoff.status = SettlementHandoffStatus::Prepared;
    settlement_handoff.prepared_at = now;
    settlement_handoff.confirmed_at = 0;
    settlement_handoff.bump = ctx.bumps.settlement_handoff;

    emit!(SettlementHandoffPrepared {
        accepted_match: accepted_match.key(),
        provider: params.provider,
        payload_hash: params.payload_hash,
        reference_hash: params.reference_hash,
        settlement_verifier: accepted_match.settlement_verifier,
    });

    Ok(())
}
