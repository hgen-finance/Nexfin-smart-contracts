use anchor_lang::prelude::*;

pub mod error;
pub mod helpers;
pub mod params;
pub mod state;

use crate::helpers::{get_depositors_fee, get_team_fee, get_trove_debt_amount};

use crate::error::LiquityError;
use anchor_spl::token::{self, Burn, Mint, MintTo, Transfer};

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

    pub fn close_trove(ctx: Context<CloseTrove>) -> ProgramResult {
        let ref mut trove = ctx.accounts.trove;

        if trove.is_liquidated {
            return Err(LiquityError::TroveAlreadyLiquidated.into());
        }

        Ok(())
    }
}
#[event]
pub struct RemoveLiquidity {
    pub out_coin: u64,
    pub out_pc: u64,
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
    pub user_token: Account<'info, Mint>,

    #[account(mut)]
    pub mint_token: Account<'info, Mint>,
}

#[derive(Accounts)]
pub struct Borrow<'info> {
    #[account(signer, mut)]
    pub authority: AccountInfo<'info>,

    #[account(mut)]
    pub trove: ProgramAccount<'info, state::Trove>,

    pub rent: Sysvar<'info, Rent>,
}
