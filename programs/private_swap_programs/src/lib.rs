use anchor_lang::prelude::*;
use ephemeral_rollups_sdk::access_control::instructions::CreatePermissionCpiBuilder;
use ephemeral_rollups_sdk::access_control::structs::{Member, MembersArgs};
use ephemeral_rollups_sdk::anchor::{delegate, ephemeral};
use ephemeral_rollups_sdk::consts::PERMISSION_PROGRAM_ID;
use ephemeral_rollups_sdk::cpi::DelegateConfig;
use inco_lightning::cpi::accounts::{Allow, Operation};
use inco_lightning::cpi::{
    allow, as_euint128, e_add, e_ge, e_mul, e_select, e_sub, new_euint128,
};
use inco_lightning::types::{Ebool, Euint128};
use inco_lightning::ID as INCO_LIGHTNING_ID;
use inco_token::cpi::accounts::IncoTransfer;
use inco_token::cpi::transfer as inco_transfer;
use inco_token::IncoAccount;
use inco_token::ID as INCO_TOKEN_ID;

declare_id!("6L8awnTc179Atp7sMharQ8uuBjiKjWxzfEns6qW4fkyF");

const POOL_SEED: &[u8] = b"pool";
const SCALAR_BYTE: u8 = 0;

fn call_allow_from_remaining<'info>(
    inco_program: &AccountInfo<'info>,
    signer: &AccountInfo<'info>,
    system_program: &AccountInfo<'info>,
    remaining_accounts: &[AccountInfo<'info>],
    handle: Euint128,
    allowed_pubkey: Pubkey,
    account_offset: usize,
) -> Result<()> {
    if remaining_accounts.len() < account_offset + 2 {
        return Err(ErrorCode::InvalidAccessAccounts.into());
    }

    let allowance_account = &remaining_accounts[account_offset];
    let allowed_address = &remaining_accounts[account_offset + 1];

    let cpi_ctx = CpiContext::new(
        inco_program.clone(),
        Allow {
            allowance_account: allowance_account.clone(),
            signer: signer.clone(),
            allowed_address: allowed_address.clone(),
            system_program: system_program.clone(),
        },
    );

    allow(cpi_ctx, handle.0, true, allowed_pubkey)?;
    Ok(())
}

#[inline(never)]
fn compute_swap_updates<'info>(
    inco_program: &AccountInfo<'info>,
    signer: &AccountInfo<'info>,
    reserve_in: Euint128,
    reserve_out: Euint128,
    protocol_fee_in: Euint128,
    amount_in_ciphertext: &[u8],
    amount_out_ciphertext: &[u8],
    fee_amount_ciphertext: &[u8],
    input_type: u8,
) -> Result<(Euint128, Euint128, Euint128)> {
    let cpi_ctx = CpiContext::new(
        inco_program.clone(),
        Operation {
            signer: signer.clone(),
        },
    );
    let mut amount_in = new_euint128(cpi_ctx, amount_in_ciphertext.to_vec(), input_type)?;

    let cpi_ctx = CpiContext::new(
        inco_program.clone(),
        Operation {
            signer: signer.clone(),
        },
    );
    let mut amount_out = new_euint128(cpi_ctx, amount_out_ciphertext.to_vec(), input_type)?;

    let cpi_ctx = CpiContext::new(
        inco_program.clone(),
        Operation {
            signer: signer.clone(),
        },
    );
    let mut fee_amount = new_euint128(cpi_ctx, fee_amount_ciphertext.to_vec(), input_type)?;

    let cpi_ctx = CpiContext::new(
        inco_program.clone(),
        Operation {
            signer: signer.clone(),
        },
    );
    let zero = as_euint128(cpi_ctx, 0)?;

    let cpi_ctx = CpiContext::new(
        inco_program.clone(),
        Operation {
            signer: signer.clone(),
        },
    );
    let has_liquidity: Ebool = e_ge(cpi_ctx, reserve_out, amount_out, SCALAR_BYTE)?;

    let cpi_ctx = CpiContext::new(
        inco_program.clone(),
        Operation {
            signer: signer.clone(),
        },
    );
    amount_in = e_select(cpi_ctx, has_liquidity, amount_in, zero, SCALAR_BYTE)?;

    let cpi_ctx = CpiContext::new(
        inco_program.clone(),
        Operation {
            signer: signer.clone(),
        },
    );
    amount_out = e_select(cpi_ctx, has_liquidity, amount_out, zero, SCALAR_BYTE)?;

    let cpi_ctx = CpiContext::new(
        inco_program.clone(),
        Operation {
            signer: signer.clone(),
        },
    );
    fee_amount = e_select(cpi_ctx, has_liquidity, fee_amount, zero, SCALAR_BYTE)?;

    let cpi_ctx = CpiContext::new(
        inco_program.clone(),
        Operation {
            signer: signer.clone(),
        },
    );
    let temp_reserve_in = e_add(cpi_ctx, reserve_in, amount_in, SCALAR_BYTE)?;

    let cpi_ctx = CpiContext::new(
        inco_program.clone(),
        Operation {
            signer: signer.clone(),
        },
    );
    let temp_reserve_out = e_sub(cpi_ctx, reserve_out, amount_out, SCALAR_BYTE)?;

    let cpi_ctx = CpiContext::new(
        inco_program.clone(),
        Operation {
            signer: signer.clone(),
        },
    );
    let old_k = e_mul(cpi_ctx, reserve_in, reserve_out, SCALAR_BYTE)?;

    let cpi_ctx = CpiContext::new(
        inco_program.clone(),
        Operation {
            signer: signer.clone(),
        },
    );
    let new_k = e_mul(cpi_ctx, temp_reserve_in, temp_reserve_out, SCALAR_BYTE)?;

    let cpi_ctx = CpiContext::new(
        inco_program.clone(),
        Operation {
            signer: signer.clone(),
        },
    );
    let k_ok: Ebool = e_ge(cpi_ctx, new_k, old_k, SCALAR_BYTE)?;

    let cpi_ctx = CpiContext::new(
        inco_program.clone(),
        Operation {
            signer: signer.clone(),
        },
    );
    amount_in = e_select(cpi_ctx, k_ok, amount_in, zero, SCALAR_BYTE)?;

    let cpi_ctx = CpiContext::new(
        inco_program.clone(),
        Operation {
            signer: signer.clone(),
        },
    );
    amount_out = e_select(cpi_ctx, k_ok, amount_out, zero, SCALAR_BYTE)?;

    let cpi_ctx = CpiContext::new(
        inco_program.clone(),
        Operation {
            signer: signer.clone(),
        },
    );
    fee_amount = e_select(cpi_ctx, k_ok, fee_amount, zero, SCALAR_BYTE)?;

    let cpi_ctx = CpiContext::new(
        inco_program.clone(),
        Operation {
            signer: signer.clone(),
        },
    );
    let new_reserve_in = e_add(cpi_ctx, reserve_in, amount_in, SCALAR_BYTE)?;

    let cpi_ctx = CpiContext::new(
        inco_program.clone(),
        Operation {
            signer: signer.clone(),
        },
    );
    let new_reserve_out = e_sub(cpi_ctx, reserve_out, amount_out, SCALAR_BYTE)?;

    let cpi_ctx = CpiContext::new(
        inco_program.clone(),
        Operation {
            signer: signer.clone(),
        },
    );
    let new_protocol_fee = e_add(cpi_ctx, protocol_fee_in, fee_amount, SCALAR_BYTE)?;

    Ok((new_reserve_in, new_reserve_out, new_protocol_fee))
}

#[ephemeral]
#[program]
pub mod private_swap_programs {
    use super::*;

    pub fn initialize<'info>(ctx: Context<'_, '_, '_, 'info, Initialize>) -> Result<()> {
        msg!("Greetings from: {:?}", ctx.program_id);
        Ok(())
    }

    pub fn create_permission(
        ctx: Context<CreatePermission>,
        account_type: AccountType,
        members: Option<Vec<Member>>,
    ) -> Result<()> {
        let CreatePermission {
            permissioned_account,
            permission,
            payer,
            permission_program,
            system_program,
        } = ctx.accounts;

        let seed_data = derive_seeds_from_account_type(&account_type);
        let seeds_slices: Vec<&[u8]> = seed_data.iter().map(|s| s.as_slice()).collect();
        let (pda, bump) = Pubkey::find_program_address(&seeds_slices, &crate::ID);
        require_keys_eq!(permissioned_account.key(), pda, ErrorCode::InvalidPermissionAccount);

        let mut seeds = seed_data.clone();
        seeds.push(vec![bump]);
        let seed_refs: Vec<&[u8]> = seeds.iter().map(|s| s.as_slice()).collect();

        CreatePermissionCpiBuilder::new(&permission_program)
            .permissioned_account(&permissioned_account.to_account_info())
            .permission(&permission)
            .payer(&payer)
            .system_program(&system_program)
            .args(MembersArgs { members })
            .invoke_signed(&[seed_refs.as_slice()])?;
        Ok(())
    }

    /// Delegate pool PDA to the MagicBlock validator for PER execution.
    pub fn delegate_pda(ctx: Context<DelegatePda>, account_type: AccountType) -> Result<()> {
        let seed_data = derive_seeds_from_account_type(&account_type);
        let seeds_refs: Vec<&[u8]> = seed_data.iter().map(|s| s.as_slice()).collect();

        let validator = ctx.accounts.validator.as_ref().map(|v| v.key());
        ctx.accounts.delegate_pda(
            &ctx.accounts.payer,
            &seeds_refs,
            DelegateConfig {
                validator,
                ..Default::default()
            },
        )?;
        Ok(())
    }

    pub fn initialize_pool<'info>(
        ctx: Context<'_, '_, '_, 'info, InitializePool<'info>>,
        fee_bps: u16,
    ) -> Result<()> {
        require!(fee_bps <= 10_000, ErrorCode::InvalidFee);

        let pool = &mut ctx.accounts.pool;
        pool.authority = ctx.accounts.authority.key();
        pool.mint_a = ctx.accounts.mint_a.key();
        pool.mint_b = ctx.accounts.mint_b.key();
        pool.fee_bps = fee_bps;
        pool.bump = ctx.bumps.pool;

        let inco = ctx.accounts.inco_lightning_program.to_account_info();
        let signer = ctx.accounts.authority.to_account_info();
        let system_program = ctx.accounts.system_program.to_account_info();

        let cpi_ctx = CpiContext::new(inco.clone(), Operation { signer: signer.clone() });
        pool.reserve_a = as_euint128(cpi_ctx, 0)?;

        let cpi_ctx = CpiContext::new(inco, Operation { signer });
        pool.reserve_b = as_euint128(cpi_ctx, 0)?;

        let inco = ctx.accounts.inco_lightning_program.to_account_info();
        let signer = ctx.accounts.authority.to_account_info();
        let cpi_ctx = CpiContext::new(inco.clone(), Operation { signer: signer.clone() });
        pool.protocol_fee_a = as_euint128(cpi_ctx, 0)?;

        let cpi_ctx = CpiContext::new(inco, Operation { signer });
        pool.protocol_fee_b = as_euint128(cpi_ctx, 0)?;

        pool.is_paused = false;
        pool.last_update_ts = Clock::get()?.unix_timestamp;

        let inco = ctx.accounts.inco_lightning_program.to_account_info();
        let signer = ctx.accounts.authority.to_account_info();
        if ctx.remaining_accounts.len() >= 2 {
            call_allow_from_remaining(
                &inco,
                &signer,
                &system_program,
                ctx.remaining_accounts,
                pool.reserve_a,
                pool.authority,
                0,
            )?;
        }

        if ctx.remaining_accounts.len() >= 4 {
            call_allow_from_remaining(
                &inco,
                &signer,
                &system_program,
                ctx.remaining_accounts,
                pool.reserve_b,
                pool.authority,
                2,
            )?;
        }

        if ctx.remaining_accounts.len() >= 6 {
            call_allow_from_remaining(
                &inco,
                &signer,
                &system_program,
                ctx.remaining_accounts,
                pool.protocol_fee_a,
                pool.authority,
                4,
            )?;
        }

        if ctx.remaining_accounts.len() >= 8 {
            call_allow_from_remaining(
                &inco,
                &signer,
                &system_program,
                ctx.remaining_accounts,
                pool.protocol_fee_b,
                pool.authority,
                6,
            )?;
        }

        Ok(())
    }

    pub fn add_liquidity<'info>(
        ctx: Context<'_, '_, '_, 'info, AddLiquidity<'info>>,
        amount_a_ciphertext: Vec<u8>,
        amount_b_ciphertext: Vec<u8>,
        input_type: u8,
    ) -> Result<()> {
        let pool = &mut ctx.accounts.pool;
        require!(pool.authority == ctx.accounts.authority.key(), ErrorCode::Unauthorized);
        require!(!pool.is_paused, ErrorCode::PoolPaused);

        let amount_a_ciphertext_for_transfer = amount_a_ciphertext.clone();
        let amount_b_ciphertext_for_transfer = amount_b_ciphertext.clone();

        let inco = ctx.accounts.inco_lightning_program.to_account_info();
        let signer = ctx.accounts.authority.to_account_info();
        let inco_token_program = ctx.accounts.inco_token_program.to_account_info();
        let system_program = ctx.accounts.system_program.to_account_info();

        let cpi_ctx = CpiContext::new(inco.clone(), Operation { signer: signer.clone() });
        let amount_a = new_euint128(cpi_ctx, amount_a_ciphertext, input_type)?;

        let cpi_ctx = CpiContext::new(inco.clone(), Operation { signer: signer.clone() });
        let amount_b = new_euint128(cpi_ctx, amount_b_ciphertext, input_type)?;

        let transfer_a_remaining = if ctx.remaining_accounts.len() >= 4 {
            ctx.remaining_accounts[0..4].to_vec()
        } else {
            Vec::new()
        };
        let transfer_ctx = if transfer_a_remaining.is_empty() {
            CpiContext::new(
                inco_token_program.clone(),
                IncoTransfer {
                    source: ctx.accounts.user_token_a.to_account_info(),
                    destination: ctx.accounts.pool_token_a.to_account_info(),
                    authority: signer.clone(),
                    inco_lightning_program: inco.clone(),
                    system_program: system_program.clone(),
                },
            )
        } else {
            CpiContext::new(
                inco_token_program.clone(),
                IncoTransfer {
                    source: ctx.accounts.user_token_a.to_account_info(),
                    destination: ctx.accounts.pool_token_a.to_account_info(),
                    authority: signer.clone(),
                    inco_lightning_program: inco.clone(),
                    system_program: system_program.clone(),
                },
            )
            .with_remaining_accounts(transfer_a_remaining)
        };
        inco_transfer(transfer_ctx, amount_a_ciphertext_for_transfer, input_type)?;

        let transfer_b_remaining = if ctx.remaining_accounts.len() >= 8 {
            ctx.remaining_accounts[4..8].to_vec()
        } else {
            Vec::new()
        };
        let transfer_ctx = if transfer_b_remaining.is_empty() {
            CpiContext::new(
                inco_token_program.clone(),
                IncoTransfer {
                    source: ctx.accounts.user_token_b.to_account_info(),
                    destination: ctx.accounts.pool_token_b.to_account_info(),
                    authority: signer.clone(),
                    inco_lightning_program: inco.clone(),
                    system_program: system_program.clone(),
                },
            )
        } else {
            CpiContext::new(
                inco_token_program.clone(),
                IncoTransfer {
                    source: ctx.accounts.user_token_b.to_account_info(),
                    destination: ctx.accounts.pool_token_b.to_account_info(),
                    authority: signer.clone(),
                    inco_lightning_program: inco.clone(),
                    system_program: system_program.clone(),
                },
            )
            .with_remaining_accounts(transfer_b_remaining)
        };
        inco_transfer(transfer_ctx, amount_b_ciphertext_for_transfer, input_type)?;

        let cpi_ctx = CpiContext::new(inco.clone(), Operation { signer: signer.clone() });
        pool.reserve_a = e_add(cpi_ctx, pool.reserve_a, amount_a, SCALAR_BYTE)?;

        let cpi_ctx = CpiContext::new(inco.clone(), Operation { signer: signer.clone() });
        pool.reserve_b = e_add(cpi_ctx, pool.reserve_b, amount_b, SCALAR_BYTE)?;

        pool.last_update_ts = Clock::get()?.unix_timestamp;

        let reserve_allowance_offset = 8usize;
        if ctx.remaining_accounts.len() >= reserve_allowance_offset + 2 {
            call_allow_from_remaining(
                &inco,
                &signer,
                &system_program,
                ctx.remaining_accounts,
                pool.reserve_a,
                pool.authority,
                reserve_allowance_offset,
            )?;
        }

        if ctx.remaining_accounts.len() >= reserve_allowance_offset + 4 {
            call_allow_from_remaining(
                &inco,
                &signer,
                &system_program,
                ctx.remaining_accounts,
                pool.reserve_b,
                pool.authority,
                reserve_allowance_offset + 2,
            )?;
        }

        Ok(())
    }

    pub fn remove_liquidity<'info>(
        ctx: Context<'_, '_, '_, 'info, RemoveLiquidity<'info>>,
        amount_a_ciphertext: Vec<u8>,
        amount_b_ciphertext: Vec<u8>,
        input_type: u8,
    ) -> Result<()> {
        let pool = &mut ctx.accounts.pool;
        require!(pool.authority == ctx.accounts.authority.key(), ErrorCode::Unauthorized);
        require!(!pool.is_paused, ErrorCode::PoolPaused);

        let amount_a_ciphertext_for_transfer = amount_a_ciphertext.clone();
        let amount_b_ciphertext_for_transfer = amount_b_ciphertext.clone();

        let inco = ctx.accounts.inco_lightning_program.to_account_info();
        let signer = ctx.accounts.authority.to_account_info();
        let inco_token_program = ctx.accounts.inco_token_program.to_account_info();
        let system_program = ctx.accounts.system_program.to_account_info();
        let reserve_a = pool.reserve_a;
        let reserve_b = pool.reserve_b;

        let cpi_ctx = CpiContext::new(inco.clone(), Operation { signer: signer.clone() });
        let amount_a = new_euint128(cpi_ctx, amount_a_ciphertext, input_type)?;

        let cpi_ctx = CpiContext::new(inco.clone(), Operation { signer: signer.clone() });
        let amount_b = new_euint128(cpi_ctx, amount_b_ciphertext, input_type)?;

        let cpi_ctx = CpiContext::new(inco.clone(), Operation { signer: signer.clone() });
        let zero = as_euint128(cpi_ctx, 0)?;

        let cpi_ctx = CpiContext::new(inco.clone(), Operation { signer: signer.clone() });
        let has_a: Ebool = e_ge(cpi_ctx, reserve_a, amount_a, SCALAR_BYTE)?;

        let cpi_ctx = CpiContext::new(inco.clone(), Operation { signer: signer.clone() });
        let has_b: Ebool = e_ge(cpi_ctx, reserve_b, amount_b, SCALAR_BYTE)?;

        let cpi_ctx = CpiContext::new(inco.clone(), Operation { signer: signer.clone() });
        let safe_a = e_select(cpi_ctx, has_a, amount_a, zero, SCALAR_BYTE)?;

        let cpi_ctx = CpiContext::new(inco.clone(), Operation { signer: signer.clone() });
        let safe_b = e_select(cpi_ctx, has_b, amount_b, zero, SCALAR_BYTE)?;

        let cpi_ctx = CpiContext::new(inco.clone(), Operation { signer: signer.clone() });
        let new_reserve_a = e_sub(cpi_ctx, reserve_a, safe_a, SCALAR_BYTE)?;

        let cpi_ctx = CpiContext::new(inco.clone(), Operation { signer: signer.clone() });
        let new_reserve_b = e_sub(cpi_ctx, reserve_b, safe_b, SCALAR_BYTE)?;

        pool.reserve_a = new_reserve_a;
        pool.reserve_b = new_reserve_b;
        pool.last_update_ts = Clock::get()?.unix_timestamp;

        let pool_signer_seeds = &[
            POOL_SEED,
            pool.mint_a.as_ref(),
            pool.mint_b.as_ref(),
            &[pool.bump],
        ];
        let pool_signer = &[pool_signer_seeds.as_ref()];

        let transfer_a_remaining = if ctx.remaining_accounts.len() >= 4 {
            ctx.remaining_accounts[0..4].to_vec()
        } else {
            Vec::new()
        };
        let transfer_ctx = if transfer_a_remaining.is_empty() {
            CpiContext::new_with_signer(
                inco_token_program.clone(),
                IncoTransfer {
                    source: ctx.accounts.pool_token_a.to_account_info(),
                    destination: ctx.accounts.user_token_a.to_account_info(),
                    authority: pool.to_account_info(),
                    inco_lightning_program: inco.clone(),
                    system_program: system_program.clone(),
                },
                pool_signer,
            )
        } else {
            CpiContext::new_with_signer(
                inco_token_program.clone(),
                IncoTransfer {
                    source: ctx.accounts.pool_token_a.to_account_info(),
                    destination: ctx.accounts.user_token_a.to_account_info(),
                    authority: pool.to_account_info(),
                    inco_lightning_program: inco.clone(),
                    system_program: system_program.clone(),
                },
                pool_signer,
            )
            .with_remaining_accounts(transfer_a_remaining)
        };
        inco_transfer(transfer_ctx, amount_a_ciphertext_for_transfer, input_type)?;

        let transfer_b_remaining = if ctx.remaining_accounts.len() >= 8 {
            ctx.remaining_accounts[4..8].to_vec()
        } else {
            Vec::new()
        };
        let transfer_ctx = if transfer_b_remaining.is_empty() {
            CpiContext::new_with_signer(
                inco_token_program.clone(),
                IncoTransfer {
                    source: ctx.accounts.pool_token_b.to_account_info(),
                    destination: ctx.accounts.user_token_b.to_account_info(),
                    authority: pool.to_account_info(),
                    inco_lightning_program: inco.clone(),
                    system_program: system_program.clone(),
                },
                pool_signer,
            )
        } else {
            CpiContext::new_with_signer(
                inco_token_program.clone(),
                IncoTransfer {
                    source: ctx.accounts.pool_token_b.to_account_info(),
                    destination: ctx.accounts.user_token_b.to_account_info(),
                    authority: pool.to_account_info(),
                    inco_lightning_program: inco.clone(),
                    system_program: system_program.clone(),
                },
                pool_signer,
            )
            .with_remaining_accounts(transfer_b_remaining)
        };
        inco_transfer(transfer_ctx, amount_b_ciphertext_for_transfer, input_type)?;

        let reserve_allowance_offset = 8usize;
        if ctx.remaining_accounts.len() >= reserve_allowance_offset + 2 {
            call_allow_from_remaining(
                &inco,
                &signer,
                &system_program,
                ctx.remaining_accounts,
                pool.reserve_a,
                pool.authority,
                reserve_allowance_offset,
            )?;
        }

        if ctx.remaining_accounts.len() >= reserve_allowance_offset + 4 {
            call_allow_from_remaining(
                &inco,
                &signer,
                &system_program,
                ctx.remaining_accounts,
                pool.reserve_b,
                pool.authority,
                reserve_allowance_offset + 2,
            )?;
        }

        Ok(())
    }

    /// Swap exact input for output. `amount_in_ciphertext` should already be net of fees.
    pub fn swap_exact_in<'info>(
        ctx: Context<'_, '_, '_, 'info, SwapExactIn<'info>>,
        amount_in_ciphertext: Vec<u8>,
        amount_out_ciphertext: Vec<u8>,
        fee_amount_ciphertext: Vec<u8>,
        input_type: u8,
        a_to_b: bool,
    ) -> Result<()> {
        let pool = &mut ctx.accounts.pool;
        require!(!pool.is_paused, ErrorCode::PoolPaused);

        let inco = ctx.accounts.inco_lightning_program.to_account_info();
        let signer = ctx.accounts.authority.to_account_info();
        let inco_token_program = ctx.accounts.inco_token_program.to_account_info();
        let system_program = ctx.accounts.system_program.to_account_info();

        let (reserve_in, reserve_out, protocol_fee_in) = if a_to_b {
            (pool.reserve_a, pool.reserve_b, pool.protocol_fee_a)
        } else {
            (pool.reserve_b, pool.reserve_a, pool.protocol_fee_b)
        };

        let (user_in, user_out, pool_in, pool_out) = if a_to_b {
            (
                &ctx.accounts.user_token_a,
                &ctx.accounts.user_token_b,
                &ctx.accounts.pool_token_a,
                &ctx.accounts.pool_token_b,
            )
        } else {
            (
                &ctx.accounts.user_token_b,
                &ctx.accounts.user_token_a,
                &ctx.accounts.pool_token_b,
                &ctx.accounts.pool_token_a,
            )
        };

        let (new_reserve_in, new_reserve_out, new_protocol_fee) = compute_swap_updates(
            &inco,
            &signer,
            reserve_in,
            reserve_out,
            protocol_fee_in,
            &amount_in_ciphertext,
            &amount_out_ciphertext,
            &fee_amount_ciphertext,
            input_type,
        )?;

        let transfer_ctx = if ctx.remaining_accounts.len() < 4 {
            CpiContext::new(
                inco_token_program.clone(),
                IncoTransfer {
                    source: user_in.to_account_info(),
                    destination: pool_in.to_account_info(),
                    authority: signer.clone(),
                    inco_lightning_program: inco.clone(),
                    system_program: system_program.clone(),
                },
            )
        } else {
            CpiContext::new(
                inco_token_program.clone(),
                IncoTransfer {
                    source: user_in.to_account_info(),
                    destination: pool_in.to_account_info(),
                    authority: signer.clone(),
                    inco_lightning_program: inco.clone(),
                    system_program: system_program.clone(),
                },
            )
            .with_remaining_accounts(ctx.remaining_accounts[0..4].to_vec())
        };
        inco_transfer(transfer_ctx, amount_in_ciphertext, input_type)?;

        let pool_signer_seeds = &[
            POOL_SEED,
            pool.mint_a.as_ref(),
            pool.mint_b.as_ref(),
            &[pool.bump],
        ];
        let pool_signer = &[pool_signer_seeds.as_ref()];
        let transfer_ctx = if ctx.remaining_accounts.len() < 8 {
            CpiContext::new_with_signer(
                inco_token_program.clone(),
                IncoTransfer {
                    source: pool_out.to_account_info(),
                    destination: user_out.to_account_info(),
                    authority: pool.to_account_info(),
                    inco_lightning_program: inco.clone(),
                    system_program: system_program.clone(),
                },
                pool_signer,
            )
        } else {
            CpiContext::new_with_signer(
                inco_token_program.clone(),
                IncoTransfer {
                    source: pool_out.to_account_info(),
                    destination: user_out.to_account_info(),
                    authority: pool.to_account_info(),
                    inco_lightning_program: inco.clone(),
                    system_program: system_program.clone(),
                },
                pool_signer,
            )
            .with_remaining_accounts(ctx.remaining_accounts[4..8].to_vec())
        };
        inco_transfer(transfer_ctx, amount_out_ciphertext, input_type)?;

        if a_to_b {
            pool.reserve_a = new_reserve_in;
            pool.reserve_b = new_reserve_out;
            pool.protocol_fee_a = new_protocol_fee;
        } else {
            pool.reserve_b = new_reserve_in;
            pool.reserve_a = new_reserve_out;
            pool.protocol_fee_b = new_protocol_fee;
        }

        pool.last_update_ts = Clock::get()?.unix_timestamp;

        let allowance_offset = 8usize;
        if ctx.remaining_accounts.len() >= allowance_offset + 2 {
            call_allow_from_remaining(
                &inco,
                &signer,
                &system_program,
                ctx.remaining_accounts,
                pool.reserve_a,
                pool.authority,
                allowance_offset,
            )?;
        }

        if ctx.remaining_accounts.len() >= allowance_offset + 4 {
            call_allow_from_remaining(
                &inco,
                &signer,
                &system_program,
                ctx.remaining_accounts,
                pool.reserve_b,
                pool.authority,
                allowance_offset + 2,
            )?;
        }

        if ctx.remaining_accounts.len() >= allowance_offset + 6 {
            call_allow_from_remaining(
                &inco,
                &signer,
                &system_program,
                ctx.remaining_accounts,
                pool.protocol_fee_a,
                pool.authority,
                allowance_offset + 4,
            )?;
        }

        if ctx.remaining_accounts.len() >= allowance_offset + 8 {
            call_allow_from_remaining(
                &inco,
                &signer,
                &system_program,
                ctx.remaining_accounts,
                pool.protocol_fee_b,
                pool.authority,
                allowance_offset + 6,
            )?;
        }

        Ok(())
    }

    pub fn withdraw_protocol_fees<'info>(
        ctx: Context<'_, '_, '_, 'info, WithdrawProtocolFees<'info>>,
        amount_a_ciphertext: Vec<u8>,
        amount_b_ciphertext: Vec<u8>,
        input_type: u8,
    ) -> Result<()> {
        let pool = &mut ctx.accounts.pool;
        require!(pool.authority == ctx.accounts.authority.key(), ErrorCode::Unauthorized);

        let inco = ctx.accounts.inco_lightning_program.to_account_info();
        let signer = ctx.accounts.authority.to_account_info();
        let fee_a = pool.protocol_fee_a;
        let fee_b = pool.protocol_fee_b;

        let cpi_ctx = CpiContext::new(inco.clone(), Operation { signer: signer.clone() });
        let amount_a = new_euint128(cpi_ctx, amount_a_ciphertext, input_type)?;

        let cpi_ctx = CpiContext::new(inco.clone(), Operation { signer: signer.clone() });
        let amount_b = new_euint128(cpi_ctx, amount_b_ciphertext, input_type)?;

        let cpi_ctx = CpiContext::new(inco.clone(), Operation { signer: signer.clone() });
        let zero = as_euint128(cpi_ctx, 0)?;

        let cpi_ctx = CpiContext::new(inco.clone(), Operation { signer: signer.clone() });
        let has_a: Ebool = e_ge(cpi_ctx, fee_a, amount_a, SCALAR_BYTE)?;

        let cpi_ctx = CpiContext::new(inco.clone(), Operation { signer: signer.clone() });
        let has_b: Ebool = e_ge(cpi_ctx, fee_b, amount_b, SCALAR_BYTE)?;

        let cpi_ctx = CpiContext::new(inco.clone(), Operation { signer: signer.clone() });
        let safe_a = e_select(cpi_ctx, has_a, amount_a, zero, SCALAR_BYTE)?;

        let cpi_ctx = CpiContext::new(inco.clone(), Operation { signer: signer.clone() });
        let safe_b = e_select(cpi_ctx, has_b, amount_b, zero, SCALAR_BYTE)?;

        let cpi_ctx = CpiContext::new(inco.clone(), Operation { signer: signer.clone() });
        pool.protocol_fee_a = e_sub(cpi_ctx, fee_a, safe_a, SCALAR_BYTE)?;

        let cpi_ctx = CpiContext::new(inco.clone(), Operation { signer: signer.clone() });
        pool.protocol_fee_b = e_sub(cpi_ctx, fee_b, safe_b, SCALAR_BYTE)?;

        pool.last_update_ts = Clock::get()?.unix_timestamp;

        let system_program = ctx.accounts.system_program.to_account_info();
        if ctx.remaining_accounts.len() >= 2 {
            call_allow_from_remaining(
                &inco,
                &signer,
                &system_program,
                ctx.remaining_accounts,
                pool.protocol_fee_a,
                pool.authority,
                0,
            )?;
        }

        if ctx.remaining_accounts.len() >= 4 {
            call_allow_from_remaining(
                &inco,
                &signer,
                &system_program,
                ctx.remaining_accounts,
                pool.protocol_fee_b,
                pool.authority,
                2,
            )?;
        }

        Ok(())
    }

    pub fn set_pause<'info>(
        ctx: Context<'_, '_, '_, 'info, SetPause<'info>>,
        is_paused: bool,
    ) -> Result<()> {
        let pool = &mut ctx.accounts.pool;
        require!(pool.authority == ctx.accounts.authority.key(), ErrorCode::Unauthorized);
        pool.is_paused = is_paused;
        pool.last_update_ts = Clock::get()?.unix_timestamp;
        Ok(())
    }

    pub fn set_fee<'info>(
        ctx: Context<'_, '_, '_, 'info, SetFee<'info>>,
        fee_bps: u16,
    ) -> Result<()> {
        require!(fee_bps <= 10_000, ErrorCode::InvalidFee);
        let pool = &mut ctx.accounts.pool;
        require!(pool.authority == ctx.accounts.authority.key(), ErrorCode::Unauthorized);
        pool.fee_bps = fee_bps;
        pool.last_update_ts = Clock::get()?.unix_timestamp;
        Ok(())
    }

    pub fn set_authority<'info>(
        ctx: Context<'_, '_, '_, 'info, SetAuthority<'info>>,
        new_authority: Pubkey,
    ) -> Result<()> {
        let pool = &mut ctx.accounts.pool;
        require!(pool.authority == ctx.accounts.authority.key(), ErrorCode::Unauthorized);
        pool.authority = new_authority;
        pool.last_update_ts = Clock::get()?.unix_timestamp;
        Ok(())
    }
}

#[account]
pub struct Pool {
    pub authority: Pubkey,
    pub mint_a: Pubkey,
    pub mint_b: Pubkey,
    pub reserve_a: Euint128,
    pub reserve_b: Euint128,
    pub protocol_fee_a: Euint128,
    pub protocol_fee_b: Euint128,
    pub fee_bps: u16,
    pub bump: u8,
    pub is_paused: bool,
    pub last_update_ts: i64,
}

impl Pool {
    pub const LEN: usize = 32 + 32 + 32 + 32 + 32 + 32 + 32 + 2 + 1 + 1 + 8;
}

#[derive(Accounts)]
pub struct Initialize {}

#[derive(Accounts)]
pub struct InitializePool<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    /// CHECK: Token mint A
    pub mint_a: UncheckedAccount<'info>,
    /// CHECK: Token mint B
    pub mint_b: UncheckedAccount<'info>,
    #[account(
        init,
        payer = authority,
        space = 8 + Pool::LEN,
        seeds = [POOL_SEED, mint_a.key().as_ref(), mint_b.key().as_ref()],
        bump
    )]
    pub pool: Account<'info, Pool>,
    pub system_program: Program<'info, System>,
    /// CHECK: Inco Lightning program for encrypted operations
    #[account(address = INCO_LIGHTNING_ID)]
    pub inco_lightning_program: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct AddLiquidity<'info> {
    pub authority: Signer<'info>,
    #[account(mut)]
    pub pool: Account<'info, Pool>,
    #[account(mut, constraint = user_token_a.mint == pool.mint_a)]
    pub user_token_a: Account<'info, IncoAccount>,
    #[account(mut, constraint = user_token_b.mint == pool.mint_b)]
    pub user_token_b: Account<'info, IncoAccount>,
    #[account(mut, constraint = pool_token_a.mint == pool.mint_a)]
    pub pool_token_a: Account<'info, IncoAccount>,
    #[account(mut, constraint = pool_token_b.mint == pool.mint_b)]
    pub pool_token_b: Account<'info, IncoAccount>,
    pub system_program: Program<'info, System>,
    /// CHECK: Inco Lightning program for encrypted operations
    #[account(address = INCO_LIGHTNING_ID)]
    pub inco_lightning_program: AccountInfo<'info>,
    /// CHECK: Inco Token program for confidential transfers
    #[account(address = INCO_TOKEN_ID)]
    pub inco_token_program: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct RemoveLiquidity<'info> {
    pub authority: Signer<'info>,
    #[account(mut)]
    pub pool: Account<'info, Pool>,
    #[account(mut, constraint = user_token_a.mint == pool.mint_a)]
    pub user_token_a: Account<'info, IncoAccount>,
    #[account(mut, constraint = user_token_b.mint == pool.mint_b)]
    pub user_token_b: Account<'info, IncoAccount>,
    #[account(mut, constraint = pool_token_a.mint == pool.mint_a)]
    pub pool_token_a: Account<'info, IncoAccount>,
    #[account(mut, constraint = pool_token_b.mint == pool.mint_b)]
    pub pool_token_b: Account<'info, IncoAccount>,
    pub system_program: Program<'info, System>,
    /// CHECK: Inco Lightning program for encrypted operations
    #[account(address = INCO_LIGHTNING_ID)]
    pub inco_lightning_program: AccountInfo<'info>,
    /// CHECK: Inco Token program for confidential transfers
    #[account(address = INCO_TOKEN_ID)]
    pub inco_token_program: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct SwapExactIn<'info> {
    pub authority: Signer<'info>,
    #[account(mut)]
    pub pool: Account<'info, Pool>,
    #[account(mut, constraint = user_token_a.mint == pool.mint_a)]
    pub user_token_a: Account<'info, IncoAccount>,
    #[account(mut, constraint = user_token_b.mint == pool.mint_b)]
    pub user_token_b: Account<'info, IncoAccount>,
    #[account(mut, constraint = pool_token_a.mint == pool.mint_a)]
    pub pool_token_a: Account<'info, IncoAccount>,
    #[account(mut, constraint = pool_token_b.mint == pool.mint_b)]
    pub pool_token_b: Account<'info, IncoAccount>,
    pub system_program: Program<'info, System>,
    /// CHECK: Inco Lightning program for encrypted operations
    #[account(address = INCO_LIGHTNING_ID)]
    pub inco_lightning_program: AccountInfo<'info>,
    /// CHECK: Inco Token program for confidential transfers
    #[account(address = INCO_TOKEN_ID)]
    pub inco_token_program: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct WithdrawProtocolFees<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(mut)]
    pub pool: Account<'info, Pool>,
    pub system_program: Program<'info, System>,
    /// CHECK: Inco Lightning program for encrypted operations
    #[account(address = INCO_LIGHTNING_ID)]
    pub inco_lightning_program: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct SetPause<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(mut)]
    pub pool: Account<'info, Pool>,
}

#[derive(Accounts)]
pub struct SetFee<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(mut)]
    pub pool: Account<'info, Pool>,
}

#[derive(Accounts)]
pub struct SetAuthority<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(mut)]
    pub pool: Account<'info, Pool>,
}

/// Unified delegate PDA context
#[delegate]
#[derive(Accounts)]
pub struct DelegatePda<'info> {
    /// CHECK: The PDA to delegate
    #[account(mut, del)]
    pub pda: AccountInfo<'info>,
    pub payer: Signer<'info>,
    /// CHECK: Checked by the delegate program
    pub validator: Option<AccountInfo<'info>>,
}

#[derive(Accounts)]
pub struct CreatePermission<'info> {
    /// CHECK: Validated via permission program CPI
    pub permissioned_account: UncheckedAccount<'info>,
    /// CHECK: Checked by the permission program
    #[account(mut)]
    pub permission: UncheckedAccount<'info>,
    #[account(mut)]
    pub payer: Signer<'info>,
    /// CHECK: PERMISSION PROGRAM
    #[account(address = PERMISSION_PROGRAM_ID)]
    pub permission_program: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub enum AccountType {
    Pool { mint_a: Pubkey, mint_b: Pubkey },
}

fn derive_seeds_from_account_type(account_type: &AccountType) -> Vec<Vec<u8>> {
    match account_type {
        AccountType::Pool { mint_a, mint_b } => vec![
            POOL_SEED.to_vec(),
            mint_a.to_bytes().to_vec(),
            mint_b.to_bytes().to_vec(),
        ],
    }
}

#[error_code]
pub enum ErrorCode {
    #[msg("Unauthorized pool authority")]
    Unauthorized,
    #[msg("Pool is paused")]
    PoolPaused,
    #[msg("Invalid fee bps")]
    InvalidFee,
    #[msg("Invalid access control accounts")]
    InvalidAccessAccounts,
    #[msg("Invalid permissioned account")]
    InvalidPermissionAccount,
}
