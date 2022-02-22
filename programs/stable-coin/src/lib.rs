use anchor_lang::prelude::*;
use crate::{error::StableCoinError};
use anchor_lang::solana_program::{program::invoke, program::invoke_signed, system_instruction };
use anchor_spl::token::{self, CloseAccount, Mint, SetAuthority, MintTo, TokenAccount, Transfer};
use pyth_client;
use std::mem::size_of;

pub mod error;

declare_id!("FMndmu5WMd562PAFgfJ9XQ5wykEcWraixubnZZ1GWYtZ");

#[program]
pub mod stable_coin {
    use super::*;
    pub fn escrow_sol(
        ctx: Context<EscrowSol>,
        _nonce: u8,
        _stable_nonce: u8,
        _escrow_nonce: u8,
        token_authority_bump: u8,
        sol_amount: u64,
    ) -> ProgramResult {
        if **ctx.accounts.admin_account.lamports.borrow() < sol_amount {
            msg!("No enough SOL");
            return Err(StableCoinError::NoEnough.into());
        }

        // Transfer SOL to the escrow account
        invoke(
            &system_instruction::transfer(
                ctx.accounts.admin_account.key,
                ctx.accounts.escrow_account.key,
                sol_amount,
            ),
            &[
                ctx.accounts.admin_account.to_account_info().clone(),
                ctx.accounts.escrow_account.clone(),
                ctx.accounts.system_program.clone(),
            ],
        )?;

        let pyth_price_info = &ctx.accounts.pyth_account;
        let pyth_price_data = &pyth_price_info.try_borrow_data()?;
        let pyth_price = pyth_client::cast::<pyth_client::Price>(pyth_price_data);

        let sc_usd_price = pyth_price.agg.price as u64; // Get the SOL/USD price from pyth.network

        msg!("The SOL/USD price is {:?}", sc_usd_price);

        if sc_usd_price < 0 {
            return Err(StableCoinError::UsdPriceWrong.into());
        }

        let init_stable_supply = (((sc_usd_price as u128) * (sol_amount as u128) * (100 as u128))
            / ((147 as u128) * (10u32.pow(8) as u128))) as u64;

        msg!("Total Supply:  {:?}", init_stable_supply);

        let seeds = &[&b"mint-authority"[..], &[token_authority_bump]];

        let cpi_accounts = MintTo {
            mint: ctx.accounts.stable_token.to_account_info(),
            to: ctx.accounts.stable_account.to_account_info(),
            authority: ctx.accounts.token_authority.to_account_info(),
        };

        token::mint_to(
            CpiContext::new(ctx.accounts.token_program.to_account_info(), cpi_accounts)
                .with_signer(&[&seeds[..]]),
            init_stable_supply,
        )?;

        ctx.accounts.escrow_info_account.last_sol_price = sc_usd_price;
        ctx.accounts.escrow_info_account.escrow_sol_amount = sol_amount;
        ctx.accounts.escrow_info_account.stable_total_supply = init_stable_supply;
        Ok(())
    }

    pub fn mint_burn_stable_token(
        ctx: Context<MintBurnToken>,
        token_authority_bump: u8,
    ) -> ProgramResult {
        let pyth_price_info = &ctx.accounts.pyth_account;
        let pyth_price_data = &pyth_price_info.try_borrow_data()?;
        let pyth_price = pyth_client::cast::<pyth_client::Price>(pyth_price_data);

        let current_price = pyth_price.agg.price as u64; // Get the SOL/USD price from pyth.network

        let last_sol_price = ctx.accounts.escrow_info_account.last_sol_price;

        let cur_stable_amount = (((current_price as u128) * (ctx.accounts.escrow_info_account.escrow_sol_amount as u128) * (100 as u128))
        / ((147 as u128) * (10u32.pow(8) as u128))) as u64;

        if current_price < last_sol_price {
            let burn_amount = ctx.accounts.escrow_info_account.stable_total_supply - cur_stable_amount;

            let cpi_ctx = CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                token::Burn {
                    mint: ctx.accounts.stable_token.to_account_info(),
                    to: ctx.accounts.stable_account.to_account_info(),
                    authority: ctx.accounts.token_authority.to_account_info(),
                },
            );
            token::burn(cpi_ctx, burn_amount)?;

        } else if current_price > last_sol_price {
            let mint_amount = cur_stable_amount - ctx.accounts.escrow_info_account.stable_total_supply;

            let seeds = &[&b"mint-authority"[..], &[token_authority_bump]];

            let cpi_accounts = MintTo {
                mint: ctx.accounts.stable_token.to_account_info(),
                to: ctx.accounts.stable_account.to_account_info(),
                authority: ctx.accounts.token_authority.to_account_info(),
            };
    
            token::mint_to(
                CpiContext::new(
                    ctx.accounts.token_program.to_account_info(),
                    cpi_accounts,
                ).with_signer(&[&seeds[..]]),
                mint_amount
            )?;
        }

        ctx.accounts.escrow_info_account.last_sol_price = current_price;
        ctx.accounts.escrow_info_account.stable_total_supply = cur_stable_amount;

        Ok(())
    }

    pub fn init_user(
        ctx: Context<InitUser>,
        _nonce: u8
    ) -> ProgramResult {
        if ctx.accounts.user_escrow_info_account.is_init {
            return Err(ProgramError::AccountAlreadyInitialized.into());
        }
        ctx.accounts.user_escrow_info_account.user_account = ctx.accounts.user_account.to_account_info().key();
        ctx.accounts.user_escrow_info_account.escrow_sol_amount = 0;
        ctx.accounts.user_escrow_info_account.is_init = true;
        Ok(())
    }

    pub fn user_escrow_sol(
        ctx: Context<UserEscrowSol>,
        token_authority_bump: u8,
        sol_amount: u64,
    ) -> ProgramResult {
        if !ctx.accounts.user_escrow_info_account.is_init {
            return Err(StableCoinError::NoInit.into());
        }
        if **ctx.accounts.user_account.lamports.borrow() < sol_amount {
            msg!("No enough SOL");
            return Err(StableCoinError::NoEnough.into());
        }

        // Transfer SOL to the escrow account
        invoke(
            &system_instruction::transfer(
                ctx.accounts.user_account.key,
                ctx.accounts.escrow_account.key,
                sol_amount,
            ),
            &[
                ctx.accounts.user_account.to_account_info().clone(),
                ctx.accounts.escrow_account.clone(),
                ctx.accounts.system_program.clone(),
            ],
        )?;

        let pyth_price_info = &ctx.accounts.pyth_account;
        let pyth_price_data = &pyth_price_info.try_borrow_data()?;
        let pyth_price = pyth_client::cast::<pyth_client::Price>(pyth_price_data);

        let sc_usd_price = pyth_price.agg.price as u64; // Get the SOL/USD price from pyth.network

        msg!("The SOL/USD price is {:?}", sc_usd_price);

        if sc_usd_price < 0 {
            return Err(StableCoinError::UsdPriceWrong.into());
        }

        let stable_supply = (((sc_usd_price as u128) * (sol_amount as u128) * (100 as u128))
        / ((147 as u128) * (10u32.pow(8) as u128))) as u64;

        msg!("Supply:  {:?}", stable_supply);

        let seeds = &[&b"mint-authority"[..], &[token_authority_bump]];

        let cpi_accounts = MintTo {
            mint: ctx.accounts.stable_token.to_account_info(),
            to: ctx.accounts.user_stable_account.to_account_info(),
            authority: ctx.accounts.token_authority.to_account_info(),
        };

        token::mint_to(
            CpiContext::new(ctx.accounts.token_program.to_account_info(), cpi_accounts)
                .with_signer(&[&seeds[..]]),
                stable_supply,
        )?;

        ctx.accounts.user_escrow_info_account.escrow_sol_amount += sol_amount;
        Ok(())
    }

    pub fn user_withdraw_sol(
        ctx: Context<UserWithdrawSol>,
    ) -> ProgramResult {
        if !ctx.accounts.user_escrow_info_account.is_init {
            return Err(StableCoinError::NoInit.into());
        }
        if ctx.accounts.user_escrow_info_account.escrow_sol_amount == 0 {
            return Err(StableCoinError::NoDeposit.into());
        }
        if ctx.accounts.user_escrow_info_account.escrow_sol_amount > **ctx.accounts.escrow_account.lamports.borrow() {
            return Err(StableCoinError::NoEnoughSolEscrow.into());
        }

        let pyth_price_info = &ctx.accounts.pyth_account;
        let pyth_price_data = &pyth_price_info.try_borrow_data()?;
        let pyth_price = pyth_client::cast::<pyth_client::Price>(pyth_price_data);

        let current_price = pyth_price.agg.price as u64; // Get the SOL/USD price from pyth.network

        let mut cur_stable_amount = (((current_price as u128) * (ctx.accounts.user_escrow_info_account.escrow_sol_amount as u128) * (100 as u128))
        / ((147 as u128) * (10u32.pow(8) as u128))) as u64;

        
        if cur_stable_amount > ctx.accounts.user_stable_account.amount {
            cur_stable_amount = ctx.accounts.user_stable_account.amount;
        }

        let cpi_ctx = CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            token::Burn {
                mint: ctx.accounts.stable_token.to_account_info(),
                to: ctx.accounts.user_stable_account.to_account_info(),
                authority: ctx.accounts.user_account.to_account_info(),
            },
        );
        token::burn(cpi_ctx, cur_stable_amount)?;

        let (_escrow_pda, _nonce) = Pubkey::find_program_address(&[b"user-escrow"], ctx.program_id);

        invoke_signed(
            &system_instruction::transfer(
                ctx.accounts.escrow_account.key,
                ctx.accounts.user_account.key,
                ctx.accounts.user_escrow_info_account.escrow_sol_amount,
            ),
            &[
                ctx.accounts.escrow_account.clone(),
                ctx.accounts.user_account.to_account_info().clone(),
                ctx.accounts.system_program.clone(),
            ],
            &[&[b"user-escrow", &[_nonce]]]
        )?;

        ctx.accounts.user_escrow_info_account.escrow_sol_amount = 0;

        Ok(())
    }

    pub fn close_user_escrow(
        _ctx: Context<CloseUserEscrowAccount>,
    ) -> ProgramResult {
        Ok(())
    }
}

#[derive(Accounts)]
#[instruction(nonce: u8, stable_nonce: u8, escrow_nonce: u8)]
pub struct EscrowSol<'info> {
    #[account(mut)]
    pub admin_account: Signer<'info>,
    #[account(mut)]
    pub stable_token: Box<Account<'info, Mint>>,
    pub token_authority: AccountInfo<'info>,
    #[account(
        init,
        seeds = [b"escrow".as_ref()],
        bump = nonce,
        payer = admin_account,
        space = 8
    )]
    pub escrow_account: AccountInfo<'info>,
    #[account(
        init,
        seeds = [b"stable-token-account".as_ref()],
        bump = stable_nonce,
        payer = admin_account,
        token::mint = stable_token,
        token::authority = token_authority
    )]
    pub stable_account: Box<Account<'info, TokenAccount>>,
    #[account(
        init,
        seeds = [b"escrow-info".as_ref()],
        bump = escrow_nonce,
        payer = admin_account,
        space = 8 + size_of::<EscrowInfoAccount>()
    )]
    pub escrow_info_account: Box<Account<'info, EscrowInfoAccount>>,
    pub pyth_account: AccountInfo<'info>,
    pub system_program: AccountInfo<'info>,
    pub rent: Sysvar<'info, Rent>,
    pub token_program: AccountInfo<'info>,

}

#[derive(Accounts)]
pub struct MintBurnToken<'info> {
    #[account(mut)]
    pub admin_account: Signer<'info>,
    #[account(mut)]
    pub stable_token: Box<Account<'info, Mint>>,
    pub token_authority: AccountInfo<'info>,
    #[account(mut)]
    pub stable_account: Box<Account<'info, TokenAccount>>,
    #[account(mut)]
    pub escrow_info_account: Box<Account<'info, EscrowInfoAccount>>,
    pub pyth_account: AccountInfo<'info>,
    pub token_program: AccountInfo<'info>,

}

#[derive(Accounts)]
#[instruction(nonce: u8)]
pub struct InitUser<'info> {
    #[account(mut)]
    pub user_account: Signer<'info>,
    #[account(
        init,
        seeds = [
            b"user-escrow-info".as_ref(),
            user_account.key().as_ref()
        ],
        bump = nonce,
        payer = user_account,
        space = 8 + size_of::<UserEscrowInfoAccount>()
    )]
    pub user_escrow_info_account: Box<Account<'info, UserEscrowInfoAccount>>,
    pub system_program: AccountInfo<'info>,
    pub rent: Sysvar<'info, Rent>,

}

#[derive(Accounts)]
pub struct UserEscrowSol<'info> {
    #[account(mut)]
    pub user_account: Signer<'info>,
    #[account(mut)]
    pub stable_token: Box<Account<'info, Mint>>,
    pub token_authority: AccountInfo<'info>,
    #[account(mut)]
    pub escrow_account: AccountInfo<'info>,
    #[account(mut)]
    pub user_stable_account: Box<Account<'info, TokenAccount>>,
    #[account(
        mut,
        constraint = &user_escrow_info_account.user_account == user_account.key
    )]
    pub user_escrow_info_account: Box<Account<'info, UserEscrowInfoAccount>>,
    pub pyth_account: AccountInfo<'info>,
    pub system_program: AccountInfo<'info>,
    pub token_program: AccountInfo<'info>,

}

#[derive(Accounts)]
pub struct UserWithdrawSol<'info> {
    #[account(mut)]
    pub user_account: Signer<'info>,
    #[account(mut)]
    pub stable_token: Box<Account<'info, Mint>>,
    pub token_authority: AccountInfo<'info>,
    #[account(mut)]
    pub user_stable_account: Box<Account<'info, TokenAccount>>,
    #[account(mut)]
    pub escrow_account: AccountInfo<'info>,
    #[account(
        mut,
        constraint = &user_escrow_info_account.user_account == user_account.key
    )]
    pub user_escrow_info_account: Box<Account<'info, UserEscrowInfoAccount>>,
    pub pyth_account: AccountInfo<'info>,
    pub token_program: AccountInfo<'info>,
    pub system_program: AccountInfo<'info>,

}

#[derive(Accounts)]
pub struct CloseUserEscrowAccount<'info> {
    #[account(mut)]
    pub user_account: Signer<'info>,
    #[account(
        mut,
        constraint = &user_escrow_info_account.user_account == user_account.key,
        constraint = user_escrow_info_account.escrow_sol_amount == 0 && user_escrow_info_account.is_init == true,
        close = user_account
    )]
    pub user_escrow_info_account: Box<Account<'info, UserEscrowInfoAccount>>,

}

#[account]
pub struct EscrowInfoAccount {
    pub last_sol_price: u64,
    pub escrow_sol_amount: u64,
    pub stable_total_supply: u64

}

#[account]
pub struct UserEscrowInfoAccount {
    pub user_account: Pubkey,
    pub escrow_sol_amount: u64,
    pub is_init: bool,
}
