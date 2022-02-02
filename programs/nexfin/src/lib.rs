use anchor_lang::prelude::*;
use std::mem::size_of;
pub mod error;
pub mod helpers;
pub mod params;
pub mod pc;
pub mod state;
use pc::Price;

use crate::helpers::{get_depositors_fee, get_team_fee, get_trove_debt_amount};
// use crate::params::SYSTEM_ACCOUNT_ADDRESS;
// use std::ops::{Add, Sub};

use crate::error::NexfinError;
// use anchor_lang::AccountsClose;
use anchor_spl::token::{self, Burn, Mint, TokenAccount};
// use std::convert::TryInto;

declare_id!("5kLDDxNQzz82UtPA5hJmyKR3nUKBtRTfu4nXaGZmLanS");

#[program]
pub mod nexfin {
    use super::*;
    // TODO: Consider put this trove to a PDA (Hung) 
    // TODO: Add the info on authroity and tokens in the different pda
    /// Borrow money
    /// Accounts expected:
    /// 0. `[signer]` The account of the person taking the trade
    /// 1. `[writable]` The account to store trove
    /// 2. `[]` The rent sysvar
    pub fn borrow(ctx: Context<Borrow>, borrow_amount: u64, lamports: u64) -> ProgramResult {
        msg!("Instruction Borrow");
        let trove = &mut ctx.accounts.trove;

        // TODO: Add authority later 
        // TODO: Check if it matches with the borrow authority in the borrow info pda
        let borrower = &ctx.accounts.authority;

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

    /// Add Borrow amount
    ///
    ///
    /// Accounts expected:
    /// 0. `[signer]` The account of the person taking the trade
    /// 1. `[writable]` The account to store trove
    /// 2. `[]` The rent sysvar
    // TODO: Check if the borrow authority is the signer
    // TODO: Check if the admin is the signer
    // TODO: Check if the admin matches in the config pda info
    pub fn addBorrow(ctx: Context<AddBorrow>, borrow_amount: u64, lamports: u64) -> ProgramResult {
        msg!("Instruction Borrow");
        let trove = &mut ctx.accounts.trove;

        // TODO: Check the borrow authority is in the borrow info pda
        // TODO: Check if the borrow authority info pda is owned by the system program
        // TODO: Check if the borrow authoirty is the signer
        let _borrower = &ctx.accounts.authority;

        if trove.is_initialized {
            return Err(ProgramError::AccountAlreadyInitialized);
        }

        trove.lamports_amount = trove.lamports_amount.checked_add(lamports).ok_or(NexfinError::MathOverflow)?;
        trove.amount_to_close = trove.amount_to_close.checked_add(borrow_amount).ok_or(NexfinError::MathOverflow)?;
        trove.borrow_amount = trove.borrow_amount.checked_add(borrow_amount).ok_or(NexfinError::MathOverflow)?;
        trove.depositor_fee = trove.depositor_fee.checked_add(get_depositors_fee(borrow_amount)).ok_or(NexfinError::MathOverflow)?;
        trove.team_fee = trove.team_fee.checked_add(get_team_fee(borrow_amount)).ok_or(NexfinError::MathOverflow)?;

        Ok(())
    }

    /// Close Trove
    ///
    ///
    /// Accounts expected:
    ///
    /// 0. `[signer]` The account of the person taking the trade
    /// 1. `[writable]` The Trove account
    /// 2. `[]` Token program
    /// 3. `[]` User token acc
    /// 4. `[]` Mint Token key
    // TODO: Check for burn authority matches with the authority in the close info pda
    // TODO: Check if the close info pda owner matches with the 
    // TODO: Check if the amount matches with the remaining amount in the bororw trove to close this troke
    // TODO: Check if the user has sufficient amount of GENS token in his/her account
    pub fn close_trove(ctx: Context<CloseTrove>) -> ProgramResult {
        let trove = &mut ctx.accounts.trove;
        if trove.is_liquidated {
            return Err(NexfinError::TroveAlreadyLiquidated.into());
        }

        let borrower = &ctx.accounts.authority;
        let user_token = &mut ctx.accounts.user_token;
        let mint_token = &mut ctx.accounts.token_mint;

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

    /// Liquidate Trove
    ///
    /// Accounts expected:
    ///
    /// 0. `[signer]` The account of the person taking the trade
    /// 1. `[writable]` The Trove account
    /// 2. `[writable]` The Trove owner
    // TODO: Transfer the solana to the treasury account
    pub fn liquidate_trove(ctx: Context<LiquidateTrove>) -> ProgramResult {
        let trove = &mut ctx.accounts.trove;
        // TODO: change the sys account to admin account
        // TODO: Check if the admin account matches the admin in the config pda
        // TODO: Check if config pda is owned by the program
        let _sys_account = &mut ctx.accounts.trove_owner;

        // TODO: Reterive info on the admin authority 
        // if *sys_account.key != SYSTEM_ACCOUNT_ADDRESS {
        //     msg!("Invalid d");
        //     return Err(ProgramError::MissingRequiredSignature);
        // }

        if trove.is_liquidated {
            return Err(NexfinError::TroveAlreadyLiquidated.into());
        }

        if !trove.is_received {
            return Err(NexfinError::TroveIsNotReceived.into());
        }

        Ok(())
    }

    /// Withdraw Coin
    ///
    /// Accounts expected:
    ///
    /// 0. `[signer]` The account of the person taking the trade
    /// 1. `[writable]` The Trove account
    pub fn withdraw_coin(ctx: Context<WithdrawCoin>, amount: u64) -> ProgramResult {
        let borrower = &mut ctx.accounts.authority;
        let trove = &mut ctx.accounts.trove;

        if !trove.is_initialized {
            return Err(NexfinError::TroveIsNotInitialized.into());
        }
        if trove.is_liquidated {
            return Err(NexfinError::TroveAlreadyLiquidated.into());
        }
        if *borrower.key != trove.owner {
            return Err(NexfinError::OnlyForTroveOwner.into());
        }

        trove.lamports_amount = trove.lamports_amount.checked_sub(amount).ok_or(NexfinError::MathOverflow)?;

        if !helpers::check_min_collateral_include_gas_fee(
            trove.borrow_amount,
            trove.lamports_amount,
        ) {
            return Err(NexfinError::InvalidCollateral.into());
        }
        Ok(())
    }

    /// Redeem Coin
    ///
    /// Accounts expected:
    ///
    /// 0. `[signer]` The account of the person taking the trade
    /// 1. `[writable]` The Trove account
    // TODO: Check for the admin authority
    // TODO: Check if the admin matches in the cofig pda account
    // TODO: Check if the config pda owner is the program account
    pub fn redeem_coin(ctx: Context<RedeemCoin>, amount: u64) -> ProgramResult {
        // TODO: Check if the borrower is the signer
        let _borrower = &mut ctx.accounts.authority;
        let trove = &mut ctx.accounts.trove;

        if !trove.is_initialized {
            return Err(NexfinError::TroveIsNotInitialized.into());
        }
        if trove.is_liquidated {
            return Err(NexfinError::TroveAlreadyLiquidated.into());
        }

        trove.lamports_amount = trove.lamports_amount.checked_sub(amount).ok_or(NexfinError::MathOverflow)?;
        Ok(())
    }

    /// Add Coin
    ///
    /// Accounts expected:
    ///
    /// 0. `[signer]` The account of the person taking the trade
    /// 1. `[writable]` The Trove account
    /// 2. `[writable]` The Temp Account to get lamports
    // TODO: Check if we are going to implement it later
    pub fn add_coin(ctx: Context<AddCoin>, amount: u64) -> ProgramResult {
        let _borrower = &mut ctx.accounts.authority;
        let trove = &mut ctx.accounts.trove;

        if !trove.is_initialized {
            return Err(NexfinError::TroveIsNotInitialized.into());
        }
        if trove.is_liquidated {
            return Err(NexfinError::TroveAlreadyLiquidated.into());
        }

        let temp_lamport_account = &mut ctx.accounts.temp_lamport_account;

        if temp_lamport_account.lamports() != amount {
            return Err(NexfinError::ExpectedAmountMismatch.into());
        }

        trove.lamports_amount = trove.lamports_amount.checked_add(amount).ok_or(NexfinError::MathOverflow)?;
        Ok(())
    }

    /// Add deposit
    ///
    /// Accounts expected:
    ///
    /// 0. `[signer]` The account of the person taking the trade
    /// 1. `[writable]` The Deposit account
    /// 2. `[]` The rent sysvar
    /// 3. `[]` Token program
    /// 4. `[]` User token acc
    /// 4. `[]` User governance token acc
    /// 5. `[]` Mint Token key
    // TODO: Add a pda for burning tokens
    // TODO: check for rent expemption on deposit account
    // TODO: Check depositor is signer
    // TODO: Check the authority key is signer (use pda)
    // TODO: Check the owner of the authority key is the program id
    // TODO: Check if the token program passed is valid
    // TODO: Add a config account to check for the authority
    // TODO: Check if the depositor has sufficent amount of token in the wallet (redundant)
    // TODO: check if the deposit account is owned by the solana program
    // TODO: Add admin as a signer
    // TODO: Check admin pubkey with the config account admin field
    // TODO: check if the config account is owned by the solana program
    // TODO: Check if the user has enough amount of gens in wallet
    pub fn add_deposit(ctx: Context<AddDeposit>, amount: u64) -> ProgramResult {
        let depositor = &mut ctx.accounts.authority;

        let deposit = &mut ctx.accounts.deposit;
        let deposit_account = &deposit.to_account_info();
        let rent = &Rent::from_account_info(deposit_account)?;

        if !rent.is_exempt(deposit_account.lamports(), deposit_account.data_len()) {
            return Err(NexfinError::NotRentExempt.into());
        }

        let temp_pda_token = &mut ctx.accounts.user_token;
        let temp_governance_token = &mut ctx.accounts.user_gov_token;
        let token_mint = &mut ctx.accounts.token_mint;

        if deposit.is_initialized {
            deposit.token_amount = deposit.token_amount.checked_add(amount).ok_or(NexfinError::MathOverflow)?;
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

    // TODO: Who's  deposit owner, how to map a depositor to deposit (Hung)
    // TODO: check if the deposit owner matches the depositor
    // TODO: Add a withdraw info account to check for the authroity and accounts
    // TODO: Add a admin as a signer
    ///  Withdraw deposit
    ///
    /// Accounts expected:
    ///
    /// 0. `[signer]` The account of the person taking the trade
    /// 1. `[writable]` The Deposit account
    pub fn withdraw_deposit(ctx: Context<WithdrawDeposit>, amount: u64) -> ProgramResult {
        // TODO: check if the depositor is the signer
        let _depositor = &mut ctx.accounts.authority;
        let deposit = &mut ctx.accounts.deposit;

        if amount > deposit.token_amount {
            return Err(NexfinError::AttemptToWithdrawTooMuch.into());
        }

        deposit.token_amount = deposit.token_amount.checked_sub(amount).ok_or(NexfinError::MathOverflow)?;
        msg!("the new deposit token amount is {}", deposit.token_amount);

        Ok(())
    }

    ///  Claim deposit reward
    ///
    /// Accounts expected:
    ///
    /// 0. `[signer]` The account of the person taking the trade
    /// 1. `[writable]` The Deposit account
    // TODO: check if the deposit owner matches the depositor
    // TODO: Add a withdraw info account to check for the authroity and accounts
    // TODO: Add a admin as a signer
    pub fn claim_deposit_reward(ctx: Context<ClaimDepositReward>) -> ProgramResult {
        // TODO: to check if the depositor is the signer
        let _depositor = &mut ctx.accounts.authority;
        let deposit = &mut ctx.accounts.deposit;

        // TODO: Retrive the authority for the rewards
        // if *depositor.key != SYSTEM_ACCOUNT_ADDRESS {
        //     return Err(ProgramError::MissingRequiredSignature);
        // }

        deposit.reward_governance_token_amount = 0;
        deposit.reward_token_amount = 0;
        deposit.reward_coin_amount = 0;

        Ok(())
    }

    /// Trove received
    ///
    ///
    /// Accounts expected:
    ///
    /// 0. `[signer]` Sys acc
    /// 1. `[writable]` The Trove account
    pub fn receive_trove(ctx: Context<ReceiveTrove>) -> ProgramResult {
        // TODO: change sys account to the admin account
        // TODO: Check if the admin account matches the config info in pda account
        // TODO: Check if the config pda owner is the program
        let _sys_acc =  &mut ctx.accounts.sys_account;
        let trove =  &mut ctx.accounts.trove;

        // TODO: Retrive the authority info
        // if *sys_acc.key != SYSTEM_ACCOUNT_ADDRESS {
        //     return Err(ProgramError::MissingRequiredSignature);
        // }

        if trove.is_liquidated {
            return Err(NexfinError::TroveAlreadyLiquidated.into());
        }
        trove.is_received = true;

        Ok(())
    }

    /// Set Deposit reward
    ///
    ///
    /// Accounts expected:
    ///
    /// 0. `[signer]` Sys acc
    /// 1. `[writable]` The Deposit account
    // TODO: Check if the depositor matches the account in the deposit account
    pub fn add_deposit_reward(
        ctx: Context<AddDepositReward>,
        coin: u64,
        governance: u64,
        token: u64,
    ) -> ProgramResult {
        // TODO: check if the admin account is the signer
        // TODO: check if the deposit account is owned by the program
        // TODO: check if the admin account matches the config info
        // TODO: Check if the owner of the config info pda is the program
        let _admin =  &mut ctx.accounts.sys_account;
        let deposit =  &mut ctx.accounts.deposit;

        // TODO: Retrive the rewards authority info
        // if *depositor.key != SYSTEM_ACCOUNT_ADDRESS {
        //     return Err(ProgramError::MissingRequiredSignature);
        // }

        deposit.reward_coin_amount = deposit.reward_coin_amount.checked_add(coin).ok_or(NexfinError::MathOverflow)?;
        deposit.reward_governance_token_amount = deposit.reward_governance_token_amount.checked_add(governance).ok_or(NexfinError::MathOverflow)?;
        deposit.reward_token_amount = deposit.reward_token_amount.checked_add(token).ok_or(NexfinError::MathOverflow)?;

        Ok(())
    }

    /// Burn amount  * 1_000_000_000 from user_token
    /// Update Trove
    ///
    ///
    /// Accounts expected:
    ///
    /// 0. `[signer]` The account of the person taking the trade
    /// 1. `[writable]` The Trove account
    /// 2. `[]` Token program
    /// 3. `[]` User token acc
    /// 4. `[]` Mint Token key
    // TODO: Add admin authority 
    // TODO: check if the admin authority is in the config info pda 
    pub fn update_trove(ctx: Context<UpdateTrove>, amount: u64) -> ProgramResult {
        // TODO: Check if the borrow matches the borrower account in the 
        let borrower = &ctx.accounts.authority;
        let user_token =  &mut ctx.accounts.user_token;
        let mint_token =  &mut ctx.accounts.token_mint;
        let trove =  &mut ctx.accounts.trove;

        // update the amount to close price
        trove.amount_to_close = (trove.amount_to_close).checked_sub(amount).ok_or(NexfinError::MathOverflow)?;

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

    pub fn load_price(ctx: Context<LoadPrice>, bump: u8) -> ProgramResult {
        msg!("Calling load price");
        let oracle = &ctx.accounts.price;
        let price_oracle = Price::load(&oracle).unwrap();

        msg!("Price_oracle price {:?}", price_oracle.agg.price);

        Ok(())
    }

    pub fn set_price(ctx: Context<SetPrice>, price: i64) -> ProgramResult {
        let oracle = &ctx.accounts.price;
        let mut price_oracle = Price::load(&oracle).unwrap();
        price_oracle.agg.price = price as i64;
        Ok(())
    }
}

#[derive(Accounts)]
pub struct SetPrice<'info> {
    #[account(mut)]
    pub price: AccountInfo<'info>,
}

#[derive(Accounts)]
#[instruction(bump: u8)]
pub struct LoadPrice<'info> {
    pub price: AccountInfo<'info>,
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

    // TODO: ask PS if this one is system_program (Hung)
    #[account(mut)]
    pub trove_owner: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct Borrow<'info> {
    #[account(signer, mut)]
    pub authority: AccountInfo<'info>,

    #[account(zero)]
    pub trove: ProgramAccount<'info, state::Trove>,

    #[account(address = spl_token::ID)]
    pub token_program: AccountInfo<'info>,

    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
pub struct AddBorrow<'info> {
    #[account(signer, mut)]
    pub authority: AccountInfo<'info>,

    #[account(zero)]
    pub trove: ProgramAccount<'info, state::Trove>,

    #[account(address = spl_token::ID)]
    pub token_program: AccountInfo<'info>,

    pub rent: Sysvar<'info, Rent>,
}
