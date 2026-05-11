use anchor_lang::prelude::*;

use crate::{
    errors::ErrorCode,
    events::PrivateMatchRequested,
    state::{ComputeProvider, Intent, IntentStatus},
};

#[derive(Accounts)]
pub struct RequestPrivateMatch<'info> {
    pub owner: Signer<'info>,
    #[account(mut, has_one = owner)]
    pub intent: Account<'info, Intent>,
}

pub fn handler(ctx: Context<RequestPrivateMatch>, arcium_computation: [u8; 32]) -> Result<()> {
    let intent = &mut ctx.accounts.intent;

    require!(
        intent.status == IntentStatus::Open,
        ErrorCode::IntentNotOpen
    );
    require!(
        intent.quote_count >= intent.min_quote_count,
        ErrorCode::NotEnoughQuotes
    );
    require!(
        intent.compute_provider == ComputeProvider::Arcium,
        ErrorCode::InvalidComputeProvider
    );

    intent.status = IntentStatus::ComputationRequested;
    intent.arcium_computation = arcium_computation;

    emit!(PrivateMatchRequested {
        intent: intent.key(),
        arcium_computation,
    });

    Ok(())
}
