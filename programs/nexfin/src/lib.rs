use anchor_lang::prelude::*;
use std::mem::size_of;
pub mod error;
pub mod helpers;
pub mod params;
pub mod state;

use crate::helpers::{get_depositors_fee, get_team_fee, get_trove_debt_amount};
use anchor_lang::solana_program::system_program;

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

    pub fn update_trove(ctx: Context<UpdateTrove>, amount: u64) -> ProgramResult {
        let ref mut token_program = ctx.accounts.token_program;
        let ref mut temp_pda_token = ctx.accounts.user_token;
        let ref mut mint_token = ctx.accounts.token_mint;

        // let transfer_to_initializer_ix = spl_token::instruction::burn(
        //     token_program.key,
        //     temp_pda_token.key, // token account key
        //     token.key,          // token mint address key
        //     borrower.key,       // authority key
        //     &[&borrower.key],   // signer pub key
        //     amount * 1000000000,
        // )?;

        // // update the amount to close price
        // trove.amount_to_close = (trove.amount_to_close).sub(amount);

        // msg!("the amount is {}", amount);
        // msg!("amount to close is {}", trove.amount_to_close);

        // msg!("Calling the token program to transfer tokens to the escrow's initializer...");
        // invoke(
        //     &transfer_to_initializer_ix,
        //     &[
        //         token.clone(),
        //         temp_pda_token.clone(),
        //         borrower.clone(),
        //         token_program.clone(),
        //     ],
        // )?;

        Ok(())
    }

    pub fn close_trove(ctx: Context<CloseTrove>) -> ProgramResult {
        let ref mut trove = ctx.accounts.trove;

        if trove.is_liquidated {
            return Err(LiquityError::TroveAlreadyLiquidated.into());
        }

        Ok(())
    }
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
pub struct Borrow<'info> {
    #[account(signer, mut)]
    pub authority: AccountInfo<'info>,

    #[account(zero)]
    pub trove: ProgramAccount<'info, state::Trove>,

    pub rent: Sysvar<'info, Rent>,
}
