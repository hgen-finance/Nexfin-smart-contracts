use anchor_lang::prelude::*;
use std::mem::size_of;
pub mod error;
pub mod helpers;
pub mod params;
pub mod state;

use crate::helpers::{get_depositors_fee, get_team_fee, get_trove_debt_amount};
use crate::params::SYSTEM_ACCOUNT_ADDRESS;
use anchor_lang::solana_program::system_program;
use std::ops::{Add, Sub};

use crate::error::LiquityError;
use anchor_spl::token::{self, Burn, Mint, MintTo, TokenAccount, Transfer};
use std::convert::TryInto;

declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");

#[program]
pub mod nexfin {
    use super::*;
    // Consider put this trove to a PDA
    pub fn borrow(ctx: Context<Borrow>, borrow_amount: u64, lamports: u64) -> ProgramResult {
        msg!("Instruction Borrow");
        let ref mut trove = ctx.accounts.trove;

        let ref borrower = ctx.accounts.authority;

        if trove.is_initialized {
            return Err(ProgramError::AccountAlreadyInitialized);
        }

        trove.is_initialized = true;
        trove.is_liquidated = false;
        trove.is_received = false;
        trove.borrow_amount = borrow_amount;
        trove.lamports_amount = lamports;
        trove.depositor_fee = get_depositors_fee(borrow_amount);
        trove.team_fee = get_team_fee(borrow_amount);
        trove.amount_to_close = get_trove_debt_amount(borrow_amount);
        trove.owner = *borrower.key;

        msg!("trove owner is {}", trove.owner);
        msg!("the borrow amount is {}", trove.borrow_amount);

        Ok(())
    }

    /// Burn amount  * 1_000_000_000 from user_token
    pub fn update_trove(ctx: Context<UpdateTrove>, amount: u64) -> ProgramResult {
        let ref borrower = ctx.accounts.authority;
        let ref mut user_token = ctx.accounts.user_token;
        let ref mut mint_token = ctx.accounts.token_mint;
        let ref mut trove = ctx.accounts.trove;

        // update the amount to close price
        trove.amount_to_close = (trove.amount_to_close).sub(amount);

        msg!("the amount is {}", amount);
        msg!("amount to close is {}", trove.amount_to_close);
        msg!("Calling the token program to transfer tokens to the escrow's initializer...");

        let amount_to_burn = amount * 1_000_000_000;
        let burn_ctx = CpiContext::new(
            ctx.accounts.token_program.clone(),
            Burn {
                authority: borrower.to_account_info(),
                mint: mint_token.to_account_info(),
                to: user_token.to_account_info(),
            },
        );
        token::burn(burn_ctx, amount_to_burn)?;

        Ok(())
    }

    pub fn close_trove(ctx: Context<CloseTrove>) -> ProgramResult {
        let ref mut trove = ctx.accounts.trove;
        if trove.is_liquidated {
            return Err(LiquityError::TroveAlreadyLiquidated.into());
        }

        let ref borrower = ctx.accounts.authority;
        let ref mut user_token = ctx.accounts.user_token;
        let ref mut mint_token = ctx.accounts.token_mint;

        let amount_to_burn = trove.amount_to_close * 1_000_000_000;
        msg!("the borrow key is {}", borrower.key);
        msg!("the token key is {}", mint_token.key());
        msg!("the token temp key is {}", user_token.key());
        msg!("the amount to be closed is  {}", trove.amount_to_close);
        msg!("the amount to be bured is  {}", amount_to_burn);

        let burn_ctx = CpiContext::new(
            ctx.accounts.token_program.clone(),
            Burn {
                authority: borrower.to_account_info(),
                mint: mint_token.to_account_info(),
                to: user_token.to_account_info(),
            },
        );

        msg!("Calling the token program to transfer tokens to the escrow's initializer...");
        token::burn(burn_ctx, amount_to_burn)?;

        msg!("Send back the lamports!");
        let trove_account = ctx.accounts.trove.to_account_info();
        **borrower.lamports.borrow_mut() = borrower
            .lamports()
            .checked_add(trove_account.lamports())
            .ok_or(LiquityError::AmountOverflow)?;

        **trove_account.lamports.borrow_mut() = 0;

        *trove_account.data.borrow_mut() = &mut [];

        Ok(())
    }

    pub fn liquidate_trove(ctx: Context<LiquidateTrove>) -> ProgramResult {
        let ref mut trove = ctx.accounts.trove;
        let ref mut sys_account = ctx.accounts.trove_owner;

        if *sys_account.key != SYSTEM_ACCOUNT_ADDRESS {
            msg!("Invalid d");
            return Err(ProgramError::MissingRequiredSignature);
        }

        if trove.is_liquidated {
            return Err(LiquityError::TroveAlreadyLiquidated.into());
        }

        if !trove.is_received {
            return Err(LiquityError::TroveIsNotReceived.into());
        }

        msg!("Send lamports to the sys acc");

        let trove_account = ctx.accounts.trove.to_account_info();
        **sys_account.lamports.borrow_mut() = sys_account
            .lamports()
            .checked_add(trove_account.lamports())
            .ok_or(LiquityError::AmountOverflow)?;

        **trove_account.lamports.borrow_mut() = 0;
        *trove_account.data.borrow_mut() = &mut [];
        Ok(())
    }

    pub fn withdraw_coin(ctx: Context<WithdrawCoin>, amount: u64) -> ProgramResult {
        let ref mut borrower = ctx.accounts.authority;
        let ref mut trove = ctx.accounts.trove;

        if !trove.is_initialized {
            return Err(LiquityError::TroveIsNotInitialized.into());
        }
        if trove.is_liquidated {
            return Err(LiquityError::TroveAlreadyLiquidated.into());
        }
        if *borrower.key != trove.owner {
            return Err(LiquityError::OnlyForTroveOwner.into());
        }

        trove.lamports_amount = trove.lamports_amount.sub(amount);

        if !helpers::check_min_collateral_include_gas_fee(
            trove.borrow_amount,
            trove.lamports_amount,
        ) {
            return Err(LiquityError::InvalidCollateral.into());
        }
        Ok(())
    }
}

#[derive(Accounts)]
pub struct WithdrawCoin<'info> {
    #[account(signer, mut)]
    pub authority: AccountInfo<'info>,

    #[account(mut)]
    pub trove: ProgramAccount<'info, state::Trove>,
}

#[derive(Accounts)]
pub struct UpdateTrove<'info> {
    #[account(signer, mut)]
    pub authority: AccountInfo<'info>,

    #[account(mut)]
    pub trove: ProgramAccount<'info, state::Trove>,

    #[account(address = spl_token::ID)]
    pub token_program: AccountInfo<'info>,

    #[account(mut)]
    pub user_token: Account<'info, TokenAccount>,

    #[account(mut)]
    pub token_mint: Account<'info, Mint>,
}

#[derive(Accounts)]
pub struct CloseTrove<'info> {
    #[account(signer, mut)]
    pub authority: AccountInfo<'info>,

    #[account(mut)]
    pub trove: ProgramAccount<'info, state::Trove>,

    #[account(address = spl_token::ID)]
    pub token_program: AccountInfo<'info>,

    #[account(mut)]
    pub user_token: Account<'info, TokenAccount>,

    #[account(mut)]
    pub token_mint: Account<'info, Mint>,
}

#[derive(Accounts)]
pub struct LiquidateTrove<'info> {
    #[account(signer, mut)]
    pub authority: AccountInfo<'info>,

    #[account(mut)]
    pub trove: ProgramAccount<'info, state::Trove>,

    #[account(mut)]
    pub trove_owner: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct Borrow<'info> {
    #[account(signer, mut)]
    pub authority: AccountInfo<'info>,

    #[account(zero)]
    pub trove: ProgramAccount<'info, state::Trove>,

    pub rent: Sysvar<'info, Rent>,
}
