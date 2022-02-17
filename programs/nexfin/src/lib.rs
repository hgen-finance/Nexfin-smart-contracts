use anchor_lang::prelude::*;
// use std::{cell::{Ref, RefMut},mem::size_of};
use std::mem::size_of;
pub mod error;
pub mod helpers;
pub mod params;
pub mod pc;
use pc::Price;
pub mod state;
use crate::helpers::{get_depositors_fee, get_team_fee, get_trove_debt_amount};
// use crate::params::SYSTEM_ACCOUNT_ADDRESS;
// use std::ops::{Add, Sub};

// use bytemuck::{from_bytes, from_bytes_mut, Pod, Zeroable};

use crate::error::NexfinError;
// use anchor_lang::AccountsClose;
use anchor_spl::token::{self, Burn, Mint, TokenAccount};
// use std::convert::TryInto;

declare_id!("HPwvr8B9KtM3CZwQg7V8pevfgsZfZBLiR3gL1HcEsGiD");

// TODO: Initialize the reserve(TVL) for the deposit
// TODO: Check more for imporving code practices and security later
// TODO: Need to add versioning to accounts (Very important for future changes !!!)
// TODO: Use bytemuck to align buffer (Important!!)
// TODO: Fix the stack offset (Important!!)

// TODO: Create pda account for the trove
// TODO: Create pda account for the deposit
// TODO: Separate the mint authority from config to different config(Important!!)
#[program]
pub mod nexfin {
    use super::*;

    // TODO: Consider put this trove to a PDA (Hung) 
    // TODO: Add the info on authroity and tokens in the different pda
    // TODO: Check for rent exemption
    // TODO: Set the minimum borrow amount to 100
    // TODO: Check if the user has enough SOL(lamports in their account)
    // TODO: check the collateral ratio when the function is called

    /// Borrow money
    /// Accounts expected:
    /// 0. `[signer]` The account of the person taking the trade
    /// 1. `[writable]` The account to store trove
    /// 2. `[]` The rent sysvar
    pub fn borrow(ctx: Context<Borrow>, borrow_amount: u64, lamports: u64, trove_account_bump: u8) -> ProgramResult {
        msg!("Instruction Borrow");
        let trove = &mut ctx.accounts.trove_account;

        // TODO: Add authority later
        // TODO: Check if the borrower is the signer
        // TODO: Check if it matches with the borrow authority in the borrow info pda
        // TODO: check for required conditions later
        // TODO: check the collateral ratio when the function is called
        // TODO: check if the minimum gens token is above 99 (done)
        let borrower = &ctx.accounts.authority;

        // check if the amount is not less than 100 token value
        if trove.borrow_amount < 100 {
            return Err(NexfinError::InvalidAmount.into());
        }

        if trove.is_initialized {
            return Err(ProgramError::AccountAlreadyInitialized);
        }

        trove.bump = trove_account_bump;
        trove.is_initialized = true; // initialize for newly created account
        trove.is_liquidated = false;
        trove.is_received = false;
        trove.borrow_amount = borrow_amount;
        trove.lamports_amount = lamports;
        trove.depositor_fee = get_depositors_fee(borrow_amount);
        trove.team_fee = get_team_fee(borrow_amount);
        trove.amount_to_close = get_trove_debt_amount(borrow_amount);
        trove.authority = *borrower.key;

        msg!("trove owner is {}", trove.authority);
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
    // TODO: Get the collateralCR for the borrow amount is not less than the total 110% CR (On hold)
    // TODO: Check if the user has enough sol(lamports) in their account to borrow the token amount (On hold)
    // TODO: Check if the config account is the pda of the current program id 
    pub fn add_borrow(ctx: Context<AddBorrow>, borrow_amount: u64, lamports: u64) -> ProgramResult {
        msg!("Instruction Borrow");
        let trove = &mut ctx.accounts.trove;

        // TODO: Check the borrow authority is in the borrow info pda
        // TODO: Check if the borrow authority info pda is owned by the system program
        // TODO: Check if the borrow authority is the signer
        // TODO: Check the borrower passed matches the owner in the trove
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
        //TODO: check for the collateral ratio before liquidating the trove

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
        if *borrower.key != trove.authority {
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
    // TODO (done): Check depositor is signer
    // TODO: Check the authority key is signer (use pda)
    // TODO (done): Check the owner of the authority key matches the owner in the deposit account
    // TODO (done): Check if the token program passed is valid
    // TODO: Add a config account to check for the authority
    // TODO: Check if the depositor has sufficent amount of token in the wallet (redundant)
    // TODO: Add admin as a signer
    // TODO: Check admin pubkey with the config account admin field
    // TODO: check if the config account is owned by the solana program
    // TODO: Check if the user has enough amount of gens in wallet
    pub fn add_deposit(ctx: Context<AddDeposit>, amount: u64, deposit_account_bump: u8) -> ProgramResult {
        let depositor = &mut ctx.accounts.authority;
        let deposit = &mut ctx.accounts.deposit_account;
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
            deposit.bump = deposit_account_bump;
            deposit.is_initialized = true;
            deposit.token_amount = amount;
            deposit.reward_token_amount = 0;
            deposit.reward_governance_token_amount = 0;
            deposit.reward_coin_amount = 0;
            deposit.bank = temp_pda_token.key();
            deposit.governance_bank = temp_governance_token.key();
            deposit.authority = *depositor.key;
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
    // TODO: Add a withdraw info account to check for the authroity and accounts
    // TODO: Add a admin as a signer
    ///  Withdraw deposit
    ///
    /// Accounts expected:
    ///
    /// 0. `[signer]` The account of the person taking the trade
    /// 1. `[writable]` The Deposit account
    pub fn withdraw_deposit(ctx: Context<WithdrawDeposit>, amount: u64) -> ProgramResult {
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
        let deposit = &mut ctx.accounts.deposit;

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
    pub fn receive_trove(ctx: Context<ReceiveTrove>, _trove_account: Pubkey) -> ProgramResult {
        // TODO: change sys account to the admin account
        // TODO: Check if the admin account matches the config info in pda account
        // TODO: Check if the config pda owner is the program
        let trove =  &mut ctx.accounts.trove;
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
        _deposit_account: Pubkey,
    ) -> ProgramResult {
        // TODO: check if the admin account is the signer
        // TODO: check if the deposit account is owned by the program
        // TODO: check if the admin account matches the config info
        // TODO: Check if the owner of the config info pda is the program
        // TODO: Retrive the rewards authority info
        let deposit =  &mut ctx.accounts.deposit;

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

    pub fn load_price(ctx: Context<LoadPrice>, _bump: u8) -> ProgramResult {
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

    // TODO: remove this funciton after the config is create
    pub fn initialize_config(ctx: Context<InitializeConfig>, _config_account_bump: u8, config: Config) -> ProgramResult {
        ctx.accounts.config_account.bump = config.bump;
        ctx.accounts.config_account.is_initialized = config.is_initialized;
        ctx.accounts.config_account.mint_account_authority = config.mint_account_authority;
        ctx.accounts.config_account.admin_account_authority = config.admin_account_authority;        
        Ok(())
    }

    pub fn change_config(ctx: Context<ChangeConfig>, config: Config) -> ProgramResult {
        ctx.accounts.config_account.is_initialized = config.is_initialized;
        ctx.accounts.config_account.mint_account_authority = config.mint_account_authority;
        ctx.accounts.config_account.admin_account_authority = config.admin_account_authority;        
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
#[instruction(deposit_account: Pubkey)]
pub struct AddDepositReward<'info> {
    #[account(mut)]
    pub admin_account_authority: Signer<'info>,

    #[account(mut, has_one = admin_account_authority, seeds = [b"config".as_ref(), admin_account_authority.key().to_bytes().as_ref()], bump = config.bump)]
    pub config: Account<'info, Config>,

    #[account(mut, seeds = [b"deposit", deposit_account.key().to_bytes().as_ref()], bump = deposit.bump)]
    pub deposit: ProgramAccount<'info, Deposit>,
}
#[derive(Accounts)]
#[instruction(trove_account: Pubkey)]
pub struct ReceiveTrove<'info> {
    #[account(mut)]
    pub admin_account_authority: Signer<'info>,

    #[account(mut, has_one = admin_account_authority, seeds = [b"config".as_ref(), admin_account_authority.key().to_bytes().as_ref()], bump = config.bump)]
    pub config: Account<'info, Config>,

    #[account(mut, seeds = [b"trove".as_ref(), trove_account.key().to_bytes().as_ref()], bump = trove.bump)]
    pub trove: ProgramAccount<'info, Trove>,
}

#[derive(Accounts)]
pub struct ClaimDepositReward<'info> {
    // only its respective depositor can claim its reward account 
    #[account(signer, mut)]
    pub authority: AccountInfo<'info>,

    #[account(mut)]
    pub admin_account_authority: Signer<'info>,

    #[account(mut, has_one = admin_account_authority, seeds = [b"config".as_ref(), admin_account_authority.key().to_bytes().as_ref()], bump = config.bump)]
    pub config: Account<'info, Config>,

    #[account(mut, has_one = authority, seeds = [b"deposit".as_ref(), authority.key().to_bytes().as_ref()], bump = deposit.bump)]
    pub deposit: ProgramAccount<'info, Deposit>,
}

// TODO: Check if the bump matches later for deposit acc
#[derive(Accounts)]
pub struct WithdrawDeposit<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(mut)]
    pub admin_account_authority: Signer<'info>,

    #[account(mut, has_one = admin_account_authority, seeds = [b"config".as_ref(), admin_account_authority.key().to_bytes().as_ref()], bump = config.bump)]
    pub config: Account<'info, Config>,

    #[account(mut, has_one = authority, seeds = [b"deposit".as_ref(),authority.key().to_bytes().as_ref()], bump = deposit.bump)]
    pub deposit: ProgramAccount<'info, Deposit>,
}

// TODO: Check if the bump matches deposit for trove acc
#[derive(Accounts)]
#[instruction(deposit_account_bump: u8)]
pub struct AddDeposit<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(mut)]
    pub admin_account_authority: Signer<'info>,

    #[account(mut, has_one = admin_account_authority, seeds = [b"config".as_ref(), admin_account_authority.key().to_bytes().as_ref()], bump = config.bump)]
    pub config: Account<'info, Config>,

    #[account(
        init_if_needed,
        seeds = [
            b"deposit".as_ref(),
            authority.key().as_ref(),
        ],
        bump = deposit_account_bump,
        payer = authority,
        space = Trove::LEN + 8
    )]
    pub deposit_account: Account<'info, Deposit>,

    pub rent: Sysvar<'info, Rent>,

    #[account(address = spl_token::ID)]
    pub token_program: AccountInfo<'info>,

    #[account(mut)]
    pub user_token: Account<'info, TokenAccount>,

    #[account(mut)]
    pub user_gov_token: Account<'info, TokenAccount>,

    #[account(mut)]
    pub token_mint: Account<'info, Mint>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct AddCoin<'info> {
    #[account(signer, mut)]
    pub authority: AccountInfo<'info>,

    #[account(mut)]
    pub admin_account_authority: Signer<'info>,

    #[account(mut, has_one = admin_account_authority, seeds = [b"config".as_ref(), admin_account_authority.key().to_bytes().as_ref()], bump = config.bump)]
    pub config: Account<'info, Config>,

    #[account(mut, seeds = [b"trove".as_ref(),authority.key().to_bytes().as_ref()], bump = trove.bump)]
    pub trove: ProgramAccount<'info, Trove>,

    #[account(signer, mut)]
    pub temp_lamport_account: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct WithdrawCoin<'info> {
    #[account(signer, mut)]
    pub authority: AccountInfo<'info>,

    #[account(mut)]
    pub admin_account_authority: Signer<'info>,

    #[account(mut, has_one = admin_account_authority, seeds = [b"config".as_ref(), admin_account_authority.key().to_bytes().as_ref()], bump = config.bump)]
    pub config: Account<'info, Config>,

    #[account(mut, seeds = [b"trove".as_ref(),authority.key.to_bytes().as_ref()], bump = trove.bump)]
    pub trove: ProgramAccount<'info, Trove>,
}

#[derive(Accounts)]
pub struct RedeemCoin<'info> {
    #[account(signer, mut)]
    pub authority: AccountInfo<'info>,

    #[account(mut)]
    pub admin_account_authority: Signer<'info>,

    #[account(mut, has_one = admin_account_authority, seeds = [b"config".as_ref(), admin_account_authority.key().to_bytes().as_ref()], bump = config.bump)]
    pub config: Account<'info, Config>,

    #[account(mut, seeds = [b"trove".as_ref(), authority.key.to_bytes().as_ref()], bump = trove.bump)]
    pub trove: ProgramAccount<'info, Trove>,
}

// TODO: Check if the bump matches later for trove acc
#[derive(Accounts)]
pub struct UpdateTrove<'info> {
    #[account(signer, mut)]
    pub authority: AccountInfo<'info>,

    #[account(mut)]
    pub admin_account_authority: Signer<'info>,

    #[account(mut, has_one = admin_account_authority, seeds = [b"config".as_ref(), admin_account_authority.key().to_bytes().as_ref()], bump = config.bump)]
    pub config: Account<'info, Config>,

    #[account(mut, has_one = authority, seeds = [b"trove".as_ref(),authority.key().to_bytes().as_ref()], bump = trove.bump)]
    pub trove: ProgramAccount<'info, Trove>,

    #[account(address = spl_token::ID)]
    pub token_program: AccountInfo<'info>,

    #[account(mut)]
    pub user_token: Account<'info, TokenAccount>,

    #[account(mut)]
    pub token_mint: Account<'info, Mint>,
}

// TODO: Check if the bump matches later for trove acc
#[derive(Accounts)]
pub struct CloseTrove<'info> {
    #[account(signer, mut)]
    pub authority: AccountInfo<'info>,

    #[account(mut)]
    pub admin_account_authority: Signer<'info>,

    #[account(mut, has_one = admin_account_authority, seeds = [b"config".as_ref(), admin_account_authority.key().to_bytes().as_ref()], bump = config.bump)]
    pub config: Account<'info, Config>,

    #[account(mut, close = authority, seeds = [b"trove".as_ref(),authority.key().to_bytes().as_ref()], bump = trove.bump)]
    pub trove: ProgramAccount<'info, Trove>,

    #[account(address = spl_token::ID)]
    pub token_program: AccountInfo<'info>,

    #[account(mut)]
    pub user_token: Account<'info, TokenAccount>,

    #[account(mut)]
    pub token_mint: Account<'info, Mint>,
}


#[derive(Accounts)]
pub struct LiquidateTrove<'info> {
    #[account(mut)]
    pub admin_account_authority: Signer<'info>,

    #[account(mut, has_one = admin_account_authority, seeds = [b"config".as_ref(), admin_account_authority.key().to_bytes().as_ref()], bump = config.bump)]
    pub config: Account<'info, Config>,

    #[account(mut, close = admin_account_authority, seeds = [b"trove".as_ref(), trove.authority.key().to_bytes().as_ref()], bump = trove.bump)]
    pub trove: ProgramAccount<'info, Trove>,

    // TODO: ask PS if this one is system_program (Hung)
    #[account(mut)]
    pub trove_owner: AccountInfo<'info>,
}

// TODO: Check if the bump matches later for trove acc
#[derive(Accounts)]
#[instruction(trove_account_bump: u8)]
#[repr(C)]
pub struct Borrow<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(mut)]
    pub admin_account_authority: Signer<'info>,

    #[account(mut, has_one = admin_account_authority, seeds = [b"config".as_ref(), admin_account_authority.key().to_bytes().as_ref()], bump = config.bump)]
    pub config: Account<'info, Config>,

    #[account(
        init_if_needed,
        seeds = [
            b"deposit".as_ref(),
            authority.key().as_ref(),
        ],
        bump = trove_account_bump,
        payer = authority,
        space = Trove::LEN + 8
    )]
    pub trove_account: Account<'info, Trove>,

    pub system_program: Program<'info, System>,

    #[account(address = spl_token::ID)]
    pub token_program: AccountInfo<'info>,

    pub rent: Sysvar<'info, Rent>,
}

// impl ZeroCopy for Borrow {}

// pub trait ZeroCopy: Pod {
//     fn load<'a>(account: &'a AccountInfo) -> Result<Ref<'a, Self>, ProgramError> {
//         let size = size_of::<Self>();
//         Ok(Ref::map(account.try_borrow_data()?, |data|{
//             from_bytes(&data[..size])
//         }))
//     }
//     fn load_mut<'a>(account: &'a AccountInfo) -> Result<RefMut<'a, Self>, ProgramError> {
//         let size = size_of::<Self>();
//         Ok(Ref::map(account.try_borrow_mut_data()?, |data|{
//             from_bytes_mut(&data[..size])
//         }))
//     }
// }

// TODO: Check if the bump matches later for trove acc
#[derive(Accounts)]
pub struct AddBorrow<'info> {
    #[account(signer, mut)]
    pub authority: AccountInfo<'info>,

    #[account(mut)]
    pub admin_account_authority: Signer<'info>,

    #[account(mut, has_one = admin_account_authority, seeds = [b"config".as_ref(), admin_account_authority.key().to_bytes().as_ref()], bump = config.bump)]
    pub config: Account<'info, Config>,

    #[account(mut, seeds=[b"trove".as_ref(), authority.key.to_bytes().as_ref()], bump = trove.bump)]
    pub trove: ProgramAccount<'info, Trove>,

    #[account(address = spl_token::ID)]
    pub token_program: AccountInfo<'info>,

    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
#[instruction(config_account_bump: u8)]
pub struct InitializeConfig<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(init, seeds = [b"config".as_ref(), authority.key().to_bytes().as_ref()], bump = config_account_bump, payer = authority, space = Config::LEN + 8)]
    pub config_account: Account<'info, Config>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct ChangeConfig<'info>{
    #[account(mut)]
    pub admin_account_authority: Signer<'info>,

    // Info: can validate that pda account is created by the different using seeds:program
    #[account(mut, seeds = [b"config".as_ref(), config_account.admin_account_authority.key().to_bytes().as_ref()], bump = config_account.bump, has_one = admin_account_authority)]
    pub config_account: Account<'info, Config>
}

// TODO: Need to serialize and de-serialize it
#[account]
pub struct Config {
    pub bump: u8,
    pub is_initialized: bool,
    pub mint_account_authority: Pubkey,
    pub admin_account_authority: Pubkey,
}

impl Config {
    /// space = 8 + 1 + 32 + 32 
    pub const LEN: usize = size_of::<Config>() + 8;
}

// TODO: Need to serialize and de-serialize it
#[account]
#[derive(Default, Debug)]
pub struct Deposit {
    pub bump: u8,
    pub is_initialized: bool,
    pub token_amount: u64,
    pub reward_token_amount: u64,
    pub reward_governance_token_amount: u64,
    pub reward_coin_amount: u64,
    pub bank: Pubkey,
    pub governance_bank: Pubkey,
    pub authority: Pubkey,
}

impl Deposit {
    /// space = 8 + 1 + 8 + 8 + 8 + 8 + 32 + 32 + 32
    pub const LEN: usize = size_of::<Deposit>() + 8;
}


// TODO: Need to serialize and de-serialize it
#[account]
#[derive(Default, Debug)]
pub struct Trove {
    pub bump: u8,
    pub is_initialized: bool,
    pub is_received: bool,
    pub is_liquidated: bool,
    pub borrow_amount: u64,
    pub lamports_amount: u64,
    pub team_fee: u64,
    pub depositor_fee: u64,
    pub amount_to_close: u64,
    pub authority: Pubkey,
}

impl Trove {
    /// space = 8 + 1 + 1 + 1 + 8 + 8 + 8 + 8 + 8 + 32
    pub const LEN: usize = size_of::<Trove>() + 8;
}