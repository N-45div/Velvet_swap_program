use anchor_lang::prelude::*;

use crate::{
    errors::ErrorCode,
    events::IntentCancelled,
    state::{Intent, IntentStatus},
};

#[derive(Accounts)]
pub struct CancelIntent<'info> {
    pub owner: Signer<'info>,
    #[account(mut, has_one = owner)]
    pub intent: Account<'info, Intent>,
}

pub fn handler(ctx: Context<CancelIntent>) -> Result<()> {
    let intent = &mut ctx.accounts.intent;

    require!(
        intent.status == IntentStatus::Open
            || intent.status == IntentStatus::ComputationRequested
            || intent.status == IntentStatus::MatchReady,
        ErrorCode::IntentNotCancellable
    );

    intent.status = IntentStatus::Cancelled;
    emit!(IntentCancelled {
        intent: intent.key()
    });

    Ok(())
}
