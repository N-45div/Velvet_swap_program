use anchor_lang::prelude::*;

pub mod errors;
pub mod events;
pub mod instructions;
pub mod state;

use instructions::*;
use state::{CreateIntentParams, PrepareSettlementHandoffParams, SubmitQuoteParams};

declare_id!("4GPgiWJN1WRifSvEVs8btvyq7Yinn6DNErnuyXDRHFFo");

#[program]
pub mod velvet_mesh {
    use super::*;

    pub fn create_intent(
        ctx: Context<CreateIntent>,
        intent_nonce: u64,
        params: CreateIntentParams,
    ) -> Result<()> {
        instructions::create_intent::handler(ctx, intent_nonce, params)
    }

    pub fn submit_quote(ctx: Context<SubmitQuote>, params: SubmitQuoteParams) -> Result<()> {
        instructions::submit_quote::handler(ctx, params)
    }

    pub fn request_private_match(
        ctx: Context<RequestPrivateMatch>,
        arcium_computation: [u8; 32],
    ) -> Result<()> {
        instructions::request_private_match::handler(ctx, arcium_computation)
    }

    pub fn record_private_match(
        ctx: Context<RecordPrivateMatch>,
        arcium_computation: [u8; 32],
        selected_quote_commitment: [u8; 32],
        selected_route: state::SettlementRoute,
    ) -> Result<()> {
        instructions::record_private_match::handler(
            ctx,
            arcium_computation,
            selected_quote_commitment,
            selected_route,
        )
    }

    pub fn accept_quote(ctx: Context<AcceptQuote>) -> Result<()> {
        instructions::accept_quote::handler(ctx)
    }

    pub fn prepare_settlement_handoff(
        ctx: Context<PrepareSettlementHandoff>,
        params: PrepareSettlementHandoffParams,
    ) -> Result<()> {
        instructions::prepare_settlement_handoff::handler(ctx, params)
    }

    pub fn mark_settlement_ready(ctx: Context<MarkSettlementReady>) -> Result<()> {
        instructions::mark_settlement_ready::handler(ctx)
    }

    pub fn cancel_intent(ctx: Context<CancelIntent>) -> Result<()> {
        instructions::cancel_intent::handler(ctx)
    }
}
