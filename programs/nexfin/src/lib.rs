use anchor_lang::prelude::*;
use std::mem::size_of;
pub mod error;
pub mod helpers;
pub mod params;
pub mod state;

use crate::helpers::{get_depositors_fee, get_team_fee, get_trove_debt_amount};
use crate::params::SYSTEM_ACCOUNT_ADDRESS;
use std::ops::{Add, Sub};

use crate::error::LiquityError;
use anchor_lang::AccountsClose;
use anchor_spl::token::{self, Burn, Mint, TokenAccount};
use std::convert::TryInto;

declare_id!("5kLDDxNQzz82UtPA5hJmyKR3nUKBtRTfu4nXaGZmLanS");

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

    pub fn redeem_coin(ctx: Context<RedeemCoin>, amount: u64) -> ProgramResult {
        let ref mut borrower = ctx.accounts.authority;
        let ref mut trove = ctx.accounts.trove;

        if !trove.is_initialized {
            return Err(LiquityError::TroveIsNotInitialized.into());
        }
        if trove.is_liquidated {
            return Err(LiquityError::TroveAlreadyLiquidated.into());
        }

        trove.lamports_amount = trove.lamports_amount.sub(amount);
        Ok(())
    }

    pub fn add_coin(ctx: Context<AddCoin>, amount: u64) -> ProgramResult {
        let ref mut borrower = ctx.accounts.authority;
        let ref mut trove = ctx.accounts.trove;

        if !trove.is_initialized {
            return Err(LiquityError::TroveIsNotInitialized.into());
        }
        if trove.is_liquidated {
            return Err(LiquityError::TroveAlreadyLiquidated.into());
        }

        let ref mut temp_lamport_account = ctx.accounts.temp_lamport_account;

        if temp_lamport_account.lamports() != amount {
            return Err(LiquityError::ExpectedAmountMismatch.into());
        }

        trove.lamports_amount = trove.lamports_amount.add(amount);
        Ok(())
    }

    pub fn add_deposit(ctx: Context<AddDeposit>, amount: u64) -> ProgramResult {
        let ref mut depositor = ctx.accounts.authority;

        let ref mut deposit = ctx.accounts.deposit;
        let deposit_account = &deposit.to_account_info();
        let rent = &Rent::from_account_info(deposit_account)?;

        if !rent.is_exempt(deposit_account.lamports(), deposit_account.data_len()) {
            return Err(LiquityError::NotRentExempt.into());
        }

        let ref mut temp_pda_token = ctx.accounts.user_token;
        let ref mut temp_governance_token = ctx.accounts.user_gov_token;
        let ref mut token_mint = ctx.accounts.token_mint;

        if deposit.is_initialized {
            deposit.token_amount = deposit.token_amount.add(amount);
        } else {
            deposit.is_initialized = true;
            deposit.token_amount = amount;
            deposit.reward_token_amount = 0;
            deposit.reward_governance_token_amount = 0;
            deposit.reward_coin_amount = 0;
            deposit.bank = temp_pda_token.key();
            deposit.governance_bank = temp_governance_token.key();
            deposit.owner = *depositor.key;
        }

        let amount_to_burn = amount * 1_000_000_000;
        let burn_ctx = CpiContext::new(
            ctx.accounts.token_program.clone(),
            Burn {
                authority: depositor.to_account_info(),
                mint: token_mint.to_account_info(),
                to: temp_pda_token.to_account_info(),
            },
        );

        msg!("Calling the token program to transfer tokens to the escrow's initializer...");
        token::burn(burn_ctx, amount_to_burn)?;

        Ok(())
    }

    // TODO: Who's  deposit owner, how to map a depositor to deposit
    pub fn withdraw_deposit(ctx: Context<WithdrawDeposit>, amount: u64) -> ProgramResult {
        let ref mut depositor = ctx.accounts.authority;
        let ref mut deposit = ctx.accounts.deposit;

        if amount > deposit.token_amount {
            return Err(LiquityError::InsufficientLiquidity.into());
        }

        deposit.token_amount = deposit.token_amount.sub(amount);
        msg!("the new deposit token amount is {}", deposit.token_amount);

        Ok(())
    }

    pub fn claim_deposit_reward(ctx: Context<ClaimDepositReward>) -> ProgramResult {
        let ref mut depositor = ctx.accounts.authority;
        let ref mut deposit = ctx.accounts.deposit;

        if *depositor.key != SYSTEM_ACCOUNT_ADDRESS {
            return Err(ProgramError::MissingRequiredSignature);
        }

        deposit.reward_governance_token_amount = 0;
        deposit.reward_token_amount = 0;
        deposit.reward_coin_amount = 0;

        Ok(())
    }

    pub fn receive_trove(ctx: Context<ReceiveTrove>) -> ProgramResult {
        let ref mut sys_acc = ctx.accounts.sys_account;
        let ref mut trove = ctx.accounts.trove;

        if *sys_acc.key != SYSTEM_ACCOUNT_ADDRESS {
            return Err(ProgramError::MissingRequiredSignature);
        }

        if trove.is_liquidated {
            return Err(LiquityError::TroveAlreadyLiquidated.into());
        }
        trove.is_received = true;

        Ok(())
    }

    pub fn add_deposit_reward(
        ctx: Context<AddDepositReward>,
        coin: u64,
        governance: u64,
        token: u64,
    ) -> ProgramResult {
        let ref mut depositor = ctx.accounts.sys_account;
        let ref mut deposit = ctx.accounts.deposit;

        if *depositor.key != SYSTEM_ACCOUNT_ADDRESS {
            return Err(ProgramError::MissingRequiredSignature);
        }

        deposit.reward_coin_amount = deposit.reward_coin_amount.add(coin);
        deposit.reward_governance_token_amount =
            deposit.reward_governance_token_amount.add(governance);
        deposit.reward_token_amount = deposit.reward_token_amount.add(token);

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
}

#[derive(Accounts)]
pub struct AddDepositReward<'info> {
    #[account(signer, mut)]
    pub sys_account: AccountInfo<'info>,

    #[account(mut)]
    pub deposit: ProgramAccount<'info, state::Deposit>,
}
#[derive(Accounts)]
pub struct ReceiveTrove<'info> {
    #[account(signer, mut)]
    pub sys_account: AccountInfo<'info>,

    #[account(mut)]
    pub trove: ProgramAccount<'info, state::Trove>,
}

#[derive(Accounts)]
pub struct ClaimDepositReward<'info> {
    #[account(signer, mut)]
    pub authority: AccountInfo<'info>,

    #[account(mut)]
    pub deposit: ProgramAccount<'info, state::Deposit>,
}

#[derive(Accounts)]
pub struct WithdrawDeposit<'info> {
    #[account(signer, mut)]
    pub authority: AccountInfo<'info>,

    #[account(mut)]
    pub deposit: ProgramAccount<'info, state::Deposit>,
}

#[derive(Accounts)]
pub struct AddDeposit<'info> {
    #[account(signer, mut)]
    pub authority: AccountInfo<'info>,

    #[account(init_if_needed, payer = authority, space = size_of::<state::Deposit>() + 8)]
    pub deposit: ProgramAccount<'info, state::Deposit>,

    pub rent: Sysvar<'info, Rent>,

    #[account(address = spl_token::ID)]
    pub token_program: AccountInfo<'info>,

    #[account(mut)]
    pub user_token: Account<'info, TokenAccount>,

    #[account(mut)]
    pub user_gov_token: Account<'info, TokenAccount>,

    #[account(mut)]
    pub token_mint: Account<'info, Mint>,

    // TODO: need to change FE to add this
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct AddCoin<'info> {
    #[account(signer, mut)]
    pub authority: AccountInfo<'info>,

    #[account(mut)]
    pub trove: ProgramAccount<'info, state::Trove>,

    #[account(signer, mut)]
    pub temp_lamport_account: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct WithdrawCoin<'info> {
    #[account(signer, mut)]
    pub authority: AccountInfo<'info>,

    #[account(mut)]
    pub trove: ProgramAccount<'info, state::Trove>,
}

#[derive(Accounts)]
pub struct RedeemCoin<'info> {
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

    #[account(mut, close = authority)]
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

    #[account(mut, close = authority)]
    pub trove: ProgramAccount<'info, state::Trove>,

    // TODO: ask PS if this one is system_program
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
