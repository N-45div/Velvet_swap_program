use anchor_lang::prelude::*;
#[allow(unused_imports)]
use anchor_lang::solana_program::pubkey;
use anchor_lang::solana_program::{
    instruction::{AccountMeta, Instruction},
    program::invoke_signed,
};
use arcium_anchor::prelude::*;
use arcium_client::idl::arcium::types::CallbackAccount;

const COMP_DEF_OFFSET_SELECT_PRIVATE_QUOTE: u32 = comp_def_offset("select_private_quote");
const NO_VALID_QUOTE: u8 = 255;
// Manual Anchor CPI keeps the Arcium Anchor 0.32 workspace decoupled from
// VelvetMesh's Anchor 0.31 workspace.
const RECORD_PRIVATE_MATCH_DISCRIMINATOR: [u8; 8] = [75, 231, 215, 146, 146, 222, 168, 222];
const VELVET_MESH_PROGRAM_ID: Pubkey = pubkey!("4GPgiWJN1WRifSvEVs8btvyq7Yinn6DNErnuyXDRHFFo");
// Mirror `programs/velvet_mesh/src/state.rs::Quote` field layout.
const QUOTE_ROUTE_OFFSET: usize = 8 + 32 + 32;
const QUOTE_COMMITMENT_OFFSET: usize = QUOTE_ROUTE_OFFSET + 1 + 32 + 32 + 32;
const QUOTE_COMMITMENT_LEN: usize = 32;

declare_id!("CEjM2iFeNzKwDtc8uGLAGVFDoaHvJmy9EunRUwAsJH8e");

#[arcium_program]
pub mod velvet_mesh_matcher {
    use super::*;

    pub fn init_select_private_quote_comp_def(
        ctx: Context<InitSelectPrivateQuoteCompDef>,
    ) -> Result<()> {
        init_comp_def(ctx.accounts, None, None)?;
        Ok(())
    }

    pub fn request_private_match(
        ctx: Context<RequestPrivateMatch>,
        computation_offset: u64,
        ciphertexts: [[u8; 32]; 12],
        pubkey: [u8; 32],
        nonce: u128,
    ) -> Result<()> {
        ctx.accounts.sign_pda_account.bump = ctx.bumps.sign_pda_account;

        let args = ArgBuilder::new()
            .x25519_pubkey(pubkey)
            .plaintext_u128(nonce)
            .encrypted_u64(ciphertexts[0])
            .encrypted_u16(ciphertexts[1])
            .encrypted_u8(ciphertexts[2])
            .encrypted_u64(ciphertexts[3])
            .encrypted_u16(ciphertexts[4])
            .encrypted_u8(ciphertexts[5])
            .encrypted_u64(ciphertexts[6])
            .encrypted_u16(ciphertexts[7])
            .encrypted_u8(ciphertexts[8])
            .encrypted_u64(ciphertexts[9])
            .encrypted_u16(ciphertexts[10])
            .encrypted_u8(ciphertexts[11])
            .build();

        let callback_accounts = vec![
            CallbackAccount {
                pubkey: ctx.accounts.sign_pda_account.key(),
                is_writable: false,
            },
            CallbackAccount {
                pubkey: ctx.accounts.velvet_mesh_program.key(),
                is_writable: false,
            },
            CallbackAccount {
                pubkey: ctx.accounts.velvet_mesh_intent.key(),
                is_writable: true,
            },
            CallbackAccount {
                pubkey: ctx.accounts.quote_0.key(),
                is_writable: false,
            },
            CallbackAccount {
                pubkey: ctx.accounts.quote_1.key(),
                is_writable: false,
            },
            CallbackAccount {
                pubkey: ctx.accounts.quote_2.key(),
                is_writable: false,
            },
        ];

        queue_computation(
            ctx.accounts,
            computation_offset,
            args,
            vec![SelectPrivateQuoteCallback::callback_ix(
                computation_offset,
                &ctx.accounts.mxe_account,
                &callback_accounts,
            )?],
            1,
            0,
        )?;

        Ok(())
    }

    #[arcium_callback(encrypted_ix = "select_private_quote")]
    pub fn select_private_quote_callback(
        ctx: Context<SelectPrivateQuoteCallback>,
        output: SignedComputationOutputs<SelectPrivateQuoteOutput>,
    ) -> Result<()> {
        let selected_quote_index = match output.verify_output(
            &ctx.accounts.cluster_account,
            &ctx.accounts.computation_account,
        ) {
            Ok(SelectPrivateQuoteOutput { field_0 }) => field_0,
            Err(_) => return Err(ErrorCode::AbortedComputation.into()),
        };

        record_velvet_mesh_match(&ctx, selected_quote_index)?;

        emit!(PrivateQuoteSelected {
            selected_quote_index,
        });

        Ok(())
    }
}

#[queue_computation_accounts("select_private_quote", payer)]
#[derive(Accounts)]
#[instruction(computation_offset: u64)]
pub struct RequestPrivateMatch<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(
        init_if_needed,
        space = 9,
        payer = payer,
        seeds = [&SIGN_PDA_SEED],
        bump,
        address = derive_sign_pda!(),
    )]
    pub sign_pda_account: Account<'info, ArciumSignerAccount>,
    #[account(address = derive_mxe_pda!())]
    pub mxe_account: Account<'info, MXEAccount>,
    #[account(mut, address = derive_mempool_pda!(mxe_account, ErrorCode::ClusterNotSet))]
    /// CHECK: checked by the Arcium program.
    pub mempool_account: UncheckedAccount<'info>,
    #[account(mut, address = derive_execpool_pda!(mxe_account, ErrorCode::ClusterNotSet))]
    /// CHECK: checked by the Arcium program.
    pub executing_pool: UncheckedAccount<'info>,
    #[account(mut, address = derive_comp_pda!(computation_offset, mxe_account, ErrorCode::ClusterNotSet))]
    /// CHECK: checked by the Arcium program.
    pub computation_account: UncheckedAccount<'info>,
    #[account(address = derive_comp_def_pda!(COMP_DEF_OFFSET_SELECT_PRIVATE_QUOTE))]
    pub comp_def_account: Account<'info, ComputationDefinitionAccount>,
    #[account(mut, address = derive_cluster_pda!(mxe_account, ErrorCode::ClusterNotSet))]
    pub cluster_account: Account<'info, Cluster>,
    #[account(mut, address = ARCIUM_FEE_POOL_ACCOUNT_ADDRESS)]
    pub pool_account: Account<'info, FeePool>,
    #[account(mut, address = ARCIUM_CLOCK_ACCOUNT_ADDRESS)]
    pub clock_account: Account<'info, ClockAccount>,
    /// CHECK: checked by the VelvetMesh program id constant before callback CPI.
    #[account(address = VELVET_MESH_PROGRAM_ID)]
    pub velvet_mesh_program: UncheckedAccount<'info>,
    /// CHECK: checked by VelvetMesh during callback CPI.
    #[account(mut)]
    pub velvet_mesh_intent: UncheckedAccount<'info>,
    /// CHECK: checked by VelvetMesh if selected by the private computation.
    pub quote_0: UncheckedAccount<'info>,
    /// CHECK: checked by VelvetMesh if selected by the private computation.
    pub quote_1: UncheckedAccount<'info>,
    /// CHECK: checked by VelvetMesh if selected by the private computation.
    pub quote_2: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
    pub arcium_program: Program<'info, Arcium>,
}

#[callback_accounts("select_private_quote")]
#[derive(Accounts)]
pub struct SelectPrivateQuoteCallback<'info> {
    pub arcium_program: Program<'info, Arcium>,
    #[account(address = derive_comp_def_pda!(COMP_DEF_OFFSET_SELECT_PRIVATE_QUOTE))]
    pub comp_def_account: Account<'info, ComputationDefinitionAccount>,
    #[account(address = derive_mxe_pda!())]
    pub mxe_account: Account<'info, MXEAccount>,
    /// CHECK: checked by Arcium callback constraints.
    pub computation_account: UncheckedAccount<'info>,
    #[account(address = derive_cluster_pda!(mxe_account, ErrorCode::ClusterNotSet))]
    pub cluster_account: Account<'info, Cluster>,
    #[account(address = ::anchor_lang::solana_program::sysvar::instructions::ID)]
    /// CHECK: checked by account constraint.
    pub instructions_sysvar: AccountInfo<'info>,
    #[account(
        seeds = [&SIGN_PDA_SEED],
        bump = sign_pda_account.bump,
        address = derive_sign_pda!(),
    )]
    pub sign_pda_account: Account<'info, ArciumSignerAccount>,
    /// CHECK: checked by the VelvetMesh program id constant before CPI.
    #[account(address = VELVET_MESH_PROGRAM_ID)]
    pub velvet_mesh_program: UncheckedAccount<'info>,
    /// CHECK: checked by VelvetMesh during CPI.
    #[account(mut)]
    pub velvet_mesh_intent: UncheckedAccount<'info>,
    /// CHECK: checked by VelvetMesh if selected by the private computation.
    pub quote_0: UncheckedAccount<'info>,
    /// CHECK: checked by VelvetMesh if selected by the private computation.
    pub quote_1: UncheckedAccount<'info>,
    /// CHECK: checked by VelvetMesh if selected by the private computation.
    pub quote_2: UncheckedAccount<'info>,
}

#[init_computation_definition_accounts("select_private_quote", payer)]
#[derive(Accounts)]
pub struct InitSelectPrivateQuoteCompDef<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(mut, address = derive_mxe_pda!())]
    pub mxe_account: Box<Account<'info, MXEAccount>>,
    #[account(mut)]
    /// CHECK: checked by Arcium during initialization.
    pub comp_def_account: UncheckedAccount<'info>,
    #[account(mut, address = derive_mxe_lut_pda!(mxe_account.lut_offset_slot))]
    /// CHECK: checked by Arcium during initialization.
    pub address_lookup_table: UncheckedAccount<'info>,
    #[account(address = LUT_PROGRAM_ID)]
    /// CHECK: Address Lookup Table program.
    pub lut_program: UncheckedAccount<'info>,
    pub arcium_program: Program<'info, Arcium>,
    pub system_program: Program<'info, System>,
}

#[event]
pub struct PrivateQuoteSelected {
    pub selected_quote_index: u8,
}

fn record_velvet_mesh_match<'info>(
    ctx: &Context<SelectPrivateQuoteCallback<'info>>,
    selected_quote_index: u8,
) -> Result<()> {
    let selected_quote = match selected_quote_index {
        0 => ctx.accounts.quote_0.to_account_info(),
        1 => ctx.accounts.quote_1.to_account_info(),
        2 => ctx.accounts.quote_2.to_account_info(),
        NO_VALID_QUOTE => return Err(ErrorCode::NoValidQuote.into()),
        _ => return Err(ErrorCode::InvalidQuoteIndex.into()),
    };

    let quote_data = selected_quote.try_borrow_data()?;
    let selected_route = quote_data
        .get(QUOTE_ROUTE_OFFSET)
        .copied()
        .ok_or(ErrorCode::InvalidQuoteAccount)?;
    let selected_quote_commitment_slice = quote_data
        .get(QUOTE_COMMITMENT_OFFSET..QUOTE_COMMITMENT_OFFSET + QUOTE_COMMITMENT_LEN)
        .ok_or(ErrorCode::InvalidQuoteAccount)?;
    let mut selected_quote_commitment = [0u8; QUOTE_COMMITMENT_LEN];
    selected_quote_commitment.copy_from_slice(selected_quote_commitment_slice);
    drop(quote_data);

    let arcium_computation = ctx.accounts.computation_account.key().to_bytes();
    let mut data = Vec::with_capacity(8 + 32 + 32 + 1);
    data.extend_from_slice(&RECORD_PRIVATE_MATCH_DISCRIMINATOR);
    data.extend_from_slice(&arcium_computation);
    data.extend_from_slice(&selected_quote_commitment);
    data.push(selected_route);

    let ix = Instruction {
        program_id: VELVET_MESH_PROGRAM_ID,
        accounts: vec![
            AccountMeta::new_readonly(ctx.accounts.sign_pda_account.key(), true),
            AccountMeta::new(ctx.accounts.velvet_mesh_intent.key(), false),
            AccountMeta::new_readonly(selected_quote.key(), false),
        ],
        data,
    };

    let signer_bump = [ctx.accounts.sign_pda_account.bump];
    let signer_seeds: &[&[u8]] = &[SIGN_PDA_SEED, &signer_bump];
    invoke_signed(
        &ix,
        &[
            ctx.accounts.sign_pda_account.to_account_info(),
            ctx.accounts.velvet_mesh_intent.to_account_info(),
            selected_quote,
            ctx.accounts.velvet_mesh_program.to_account_info(),
        ],
        &[signer_seeds],
    )?;

    Ok(())
}

#[error_code]
pub enum ErrorCode {
    #[msg("The computation was aborted")]
    AbortedComputation,
    #[msg("Cluster not set")]
    ClusterNotSet,
    #[msg("The private matcher did not find a valid quote")]
    NoValidQuote,
    #[msg("The selected quote index is outside the callback quote set")]
    InvalidQuoteIndex,
    #[msg("The selected quote account does not match the VelvetMesh quote layout")]
    InvalidQuoteAccount,
}
