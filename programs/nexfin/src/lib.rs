use anchor_lang::prelude::*;
// use std::{cell::{Ref, RefMut},mem::size_of};
use std::mem::size_of;
pub mod error;
pub mod helpers;
pub mod params;
pub mod pc;
use pc::Price;
pub mod state;
use crate::helpers::{get_depositors_fee, get_team_fee};
// use crate::params::SYSTEM_ACCOUNT_ADDRESS;
// use std::ops::{Add, Sub};

// use bytemuck::{from_bytes, from_bytes_mut, Pod, Zeroable};

use crate::error::NexfinError;
// use anchor_lang::AccountsClose;
use anchor_spl::token::{self, Burn, Mint, MintTo, TokenAccount};
use anchor_lang::solana_program::{program::invoke, system_instruction};

use std::convert::TryInto;

// for pyth price for borrow
use pyth_client;

pub const COLLATERAL_RATIO:i32 = 110;

declare_id!("HPwvr8B9KtM3CZwQg7V8pevfgsZfZBLiR3gL1HcEsGiD");

// TODO: Initialize the reserve(TVL) for the deposit
// TODO: Check more for imporving code practices and security later
// TODO: Need to add versioning to accounts (important for future changes !!!)
// TODO: Use bytemuck to align buffer (Important!!)
// TODO: Fix the stack offset (Important!!)

#[program]
pub mod nexfin {
    use super::*;

    /// Borrow money
    /// Accounts expected:
    /// 0. `[signer]` The account of the person taking the trade
    /// 1. `[writable]` The account to store trove
    /// 2. `[]` The rent sysvar
    pub fn borrow(ctx: Context<Borrow>, borrow_amount: u64, lamports: u64, trove_account_bump: u8, sol_account_bump:u8, mint_account_bump: u8, fee_account_bump:u8, team_fee_account_bump:u8) -> ProgramResult {
        msg!("Instruction Borrow");
        let trove = &mut ctx.accounts.trove_account;
        let sol_trove = &mut ctx.accounts.sol_trove;

        let fee = &mut ctx.accounts.fee_account;
        let team_fee = &mut ctx.accounts.team_fee_account;

        let borrower = &ctx.accounts.authority;

        // check if the user has sufficent amount in the wallet
        if **ctx.accounts.authority.lamports.borrow() < lamports {
            return Err(NexfinError::InsufficientLiquidity.into());
        }

        // check if the amount is not less than 100 token value
        if borrow_amount < 100 {
            return Err(NexfinError::InvalidAmount.into());
        }

        // check for SOL price
        let pyth_price_info = &ctx.accounts.pyth_sol_account;
        let pyth_price_data = &pyth_price_info.try_borrow_data()?;
        let pyth_price = pyth_client::cast::<pyth_client::Price>(pyth_price_data);

        let sol_price = pyth_price.agg.price as u128;
        
        // check the collateral ratio (110%)
        let collateral_price = sol_price.checked_mul(lamports as u128).ok_or(NexfinError::MathOverflow)?; 

        let collateral_ratio = collateral_price.checked_mul(100).ok_or(NexfinError::MathOverflow)?.checked_div(borrow_amount as u128).ok_or(NexfinError::MathOverflow)?.checked_div(1_000_000_000).ok_or(NexfinError::MathOverflow)?.checked_div(100_000_000).ok_or(NexfinError::MathOverflow)?;

        // calculate the fee in sol
        let dep_fee_in_gens = get_depositors_fee(borrow_amount);
        let team_fee_in_gens = get_team_fee(borrow_amount);
        let dep_fee_in_sol = dep_fee_in_gens.checked_mul(10_000_000_000_000).ok_or(NexfinError::MathOverflow)?.checked_div(sol_price.try_into().unwrap()).ok_or(NexfinError::MathOverflow)?;
        let team_fee_in_sol = team_fee_in_gens.checked_mul(10_000_000_000_000).ok_or(NexfinError::MathOverflow)?.checked_div(sol_price.try_into().unwrap()).ok_or(NexfinError::MathOverflow)?;

        if collateral_ratio > COLLATERAL_RATIO.try_into().unwrap() {
            invoke(
                &system_instruction::transfer(
                    ctx.accounts.authority.key,
                    sol_trove.to_account_info().key,
                    lamports
                ),
                &[
                    ctx.accounts.authority.to_account_info().clone(),
                    sol_trove.to_account_info().clone(),
                    ctx.accounts.system_program.to_account_info().clone()
                ]
            )?;

            invoke(
                &system_instruction::transfer(
                    ctx.accounts.authority.key,
                    &fee.key(),
                    dep_fee_in_sol
                ),
                &[
                    ctx.accounts.authority.to_account_info().clone(),
                    fee.to_account_info().clone(),
                    ctx.accounts.system_program.to_account_info().clone()
                ]
            )?;

            invoke(
                &system_instruction::transfer(
                    ctx.accounts.authority.key,
                    &team_fee.key(),
                    team_fee_in_sol
                ),
                &[
                    ctx.accounts.authority.to_account_info().clone(),
                    team_fee.to_account_info().clone(),
                    ctx.accounts.system_program.to_account_info().clone()
                ]
            )?;

            // adding fee info
            fee.bump = fee_account_bump;
            fee.is_initialized = true;
            fee.sol_amount = fee.sol_amount.checked_add(dep_fee_in_sol).ok_or(NexfinError::MathOverflow)?;

            team_fee.bump = team_fee_account_bump;
            team_fee.is_initialized = true;
            team_fee.sol_amount = team_fee.sol_amount.checked_add(team_fee_in_sol).ok_or(NexfinError::MathOverflow)?;

            // Mint
            // TODO:  add a account to handle multiple tokens later
            let seeds:&[&[u8]; 2] = &[
                b"mint-authority",
                &[mint_account_bump]
            ];
            let signer = &[&seeds[..]];
            let cpi_accounts = MintTo {
                mint: ctx.accounts.stable_coin.to_account_info(),
                to: ctx.accounts.user_token.to_account_info(),
                authority: ctx.accounts.token_authority.to_account_info(),
            };

            let cpi_program = ctx.accounts.token_program.to_account_info();
            let cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, signer);

            // token::mint_to(cpi_ctx, (borrow_amount)*1_000_000_000)?;
            token::mint_to(cpi_ctx,borrow_amount*1_00)?;


            trove.bump = trove_account_bump;
            trove.sol_bump = sol_account_bump;
            trove.is_initialized = true; // initialize for newly created account
            trove.is_liquidated = false;
            trove.is_received = false;
            trove.borrow_amount = borrow_amount;
            trove.lamports_amount = lamports;
            trove.depositor_fee = get_depositors_fee(borrow_amount);
            trove.team_fee = get_team_fee(borrow_amount);
            trove.amount_to_close = borrow_amount;
            trove.authority = *borrower.key;

            msg!("Trove owner is {}", trove.authority);
            msg!("The borrow amount is {}", trove.borrow_amount);

        } else {
            return Err(NexfinError::BorrowTooLarge.into());
        }        

        Ok(())
    }

    /// Add Borrow amount
    ///
    ///
    /// Accounts expected:
    /// 0. `[signer]` The account of the person taking the trade
    /// 1. `[writable]` The account to store trove
    /// 2. `[]` The rent sysvar
    pub fn add_borrow(ctx: Context<AddBorrow>, borrow_amount: u64, lamports: u64, mint_account_bump: u8) -> ProgramResult {
        msg!("Instruction Add Borrow");
        let trove = &mut ctx.accounts.trove;
        let sol_trove = &mut ctx.accounts.sol_trove;

        // check for SOL price
        let pyth_price_info = &ctx.accounts.pyth_sol_account;
        let pyth_price_data = &pyth_price_info.try_borrow_data()?;
        let pyth_price = pyth_client::cast::<pyth_client::Price>(pyth_price_data);

        let sol_price = pyth_price.agg.price as u128;

        let mut total_collateral_price = sol_price.checked_mul(trove.lamports_amount as u128).ok_or(NexfinError::MathOverflow)?;
        total_collateral_price = total_collateral_price.checked_add(sol_price.checked_mul(lamports as u128).ok_or(NexfinError::MathOverflow)?).ok_or(NexfinError::MathOverflow)?;

        let total_borrow_amount = trove.borrow_amount.checked_add(borrow_amount).ok_or(NexfinError::MathOverflow)?;

        let collateral_ratio = total_collateral_price.checked_mul(100).ok_or(NexfinError::MathOverflow)?.checked_div(total_borrow_amount as u128).ok_or(NexfinError::MathOverflow)?.checked_div(1_000_000_000).ok_or(NexfinError::MathOverflow)?.checked_div(100_000_000).ok_or(NexfinError::MathOverflow)?;

        if collateral_ratio > COLLATERAL_RATIO.try_into().unwrap() {

            invoke(
                &system_instruction::transfer(
                    ctx.accounts.authority.key,
                    sol_trove.to_account_info().key,
                    lamports
                ),
                &[
                    ctx.accounts.authority.to_account_info().clone(),
                    sol_trove.to_account_info().clone(),
                    ctx.accounts.system_program.to_account_info().clone()
                ]
            )?;

            // Mint
            // TODO:  add a account to handle multiple tokens later
            let seeds:&[&[u8]; 2] = &[
                b"mint-authority",
                &[mint_account_bump]
            ];
            let signer = &[&seeds[..]];
            let cpi_accounts = MintTo {
                mint: ctx.accounts.stable_coin.to_account_info(),
                to: ctx.accounts.user_token.to_account_info(),
                authority: ctx.accounts.token_authority.to_account_info(),
            };

            let cpi_program = ctx.accounts.token_program.to_account_info();
            let cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, signer);

            // token::mint_to(cpi_ctx,borrow_amount*1_000_000_000)?;
            token::mint_to(cpi_ctx,borrow_amount*1_00)?;


            trove.lamports_amount = trove.lamports_amount.checked_add(lamports).ok_or(NexfinError::MathOverflow)?;
            trove.amount_to_close = trove.amount_to_close.checked_add(borrow_amount).ok_or(NexfinError::MathOverflow)?;
            trove.borrow_amount = trove.borrow_amount.checked_add(borrow_amount).ok_or(NexfinError::MathOverflow)?;
        
        }

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
    pub fn close_trove(ctx: Context<CloseTrove>, _sol_account_bump:u8) -> ProgramResult {
        let trove = &mut ctx.accounts.trove;
        let sol_trove = ctx.accounts.sol_trove.to_account_info();

        if trove.is_liquidated {
            return Err(NexfinError::TroveAlreadyLiquidated.into());
        }

        let borrower = ctx.accounts.authority.lamports();
        let user_token = &mut ctx.accounts.user_token;
        let mint_token = &mut ctx.accounts.token_mint;

        // let amount_to_burn = trove.amount_to_close * 1_000_000_000;
        let amount_to_burn = trove.amount_to_close * 1_00;


        let burn_ctx = CpiContext::new(
            ctx.accounts.token_program.clone(),
            Burn {
                authority: ctx.accounts.authority.to_account_info(),
                mint: mint_token.to_account_info(),
                to: user_token.to_account_info(),
            },
        );

        msg!("Calling the token program to transfer tokens to the escrow's initializer...");
        token::burn(burn_ctx, amount_to_burn)?;

        msg!("Send back the lamports!");
        **ctx.accounts.authority.lamports.borrow_mut() = borrower
        .checked_add(sol_trove.lamports())
        .unwrap();
        **sol_trove.lamports.borrow_mut() = 0;

        Ok(())
    }

    /// Liquidate Trove
    ///
    /// Accounts expected:
    ///
    /// 0. `[signer]` The account of the person taking the trade
    /// 1. `[writable]` The Trove account
    /// 2. `[writable]` The Trove owner
    pub fn liquidate_trove(ctx: Context<LiquidateTrove>, _trove_bump:u8) -> ProgramResult {
        let trove = &mut ctx.accounts.trove;
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
    pub fn withdraw_coin(ctx: Context<WithdrawCoin>, amount: u64, _trove_bump: u8, ) -> ProgramResult {
        let trove = &mut ctx.accounts.trove;

        if !trove.is_initialized {
            return Err(NexfinError::TroveIsNotInitialized.into());
        }

        if trove.is_liquidated {
            return Err(NexfinError::TroveAlreadyLiquidated.into());
        }

        trove.lamports_amount = trove.lamports_amount.checked_sub(amount).ok_or(NexfinError::MathOverflow)?;

        // Does the from account have enough lamports to transfer?
        if **ctx.accounts.sol_trove.try_borrow_lamports()? < amount {
            return Err(NexfinError::InsufficientLiquidity.into());
        }

        // TODO: change the logic for chekcing collateral ratio after the sol is withdrawed


        // Debit from_account and credit to_account
        // TODO add check add and check sum
        **ctx.accounts.sol_trove.try_borrow_mut_lamports()? -= amount;
        **ctx.accounts.authority.try_borrow_mut_lamports()? += amount;

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

        //TODO: add this later
        // let deposit_account = &deposit.to_account_info();
        // let rent = &Rent::from_account_info(deposit_account)?;

        // if !rent.is_exempt(deposit_account.lamports(), deposit_account.data_len()) {
        //     return Err(NexfinError::NotRentExempt.into());
        // }

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

        // let amount_to_burn = amount * 1_000_000_000;
        let amount_to_burn = amount * 1_00;
    
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
    pub fn withdraw_deposit(ctx: Context<WithdrawDeposit>, amount: u64, mint_account_bump: u8, _deposit_account_bump: u8) -> ProgramResult {
        let deposit = &mut ctx.accounts.deposit;

        if amount > deposit.token_amount {
            return Err(NexfinError::AttemptToWithdrawTooMuch.into());
        }

        // Mint
        //TODO: need to burn the other token in the deposit wallet
        // TODO:  add a account to handle multiple tokens later
        let seeds:&[&[u8]; 2] = &[
            b"mint-authority",
            &[mint_account_bump]
        ];
        let signer = &[&seeds[..]];
        let cpi_accounts = MintTo {
            mint: ctx.accounts.stable_coin.to_account_info(),
            to: ctx.accounts.user_token.to_account_info(),
            authority: ctx.accounts.token_authority.to_account_info(),
        };

        let cpi_program = ctx.accounts.token_program.to_account_info();
        let cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, signer);

        // token::mint_to(cpi_ctx, amount.checked_mul(1_000_000_000).ok_or(NexfinError::MathOverflow)?)?;
        token::mint_to(cpi_ctx, amount.checked_mul(1_00).ok_or(NexfinError::MathOverflow)?)?;


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
    pub fn claim_deposit_reward(ctx: Context<ClaimDepositReward>, mint_account_bump: u8, _deposit_account_bump: u8, _reward_vault_bump: u8) -> ProgramResult {
        let deposit = &mut ctx.accounts.deposit;

        let seeds:&[&[u8]; 2] = &[
            b"mint-authority",
            &[mint_account_bump]
        ];
        let signer = &[&seeds[..]];
        let cpi_accounts = MintTo {
            mint: ctx.accounts.stable_coin.to_account_info(),
            to: ctx.accounts.user_token_account.to_account_info(),
            authority: ctx.accounts.token_authority.to_account_info(),
        };

        let cpi_program = ctx.accounts.token_program.to_account_info();
        let cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, signer);

        msg!("User token reward is {}", deposit.reward_token_amount);
        msg!("User coin reward is {}", deposit.reward_coin_amount);

        token::mint_to(cpi_ctx, deposit.reward_token_amount.checked_mul(10_000_000).ok_or(NexfinError::MathOverflow)?)?;

        //TODO add sol rewards from the reward coin vault

        deposit.reward_governance_token_amount = 0; // not finalised yet on this !!!
        deposit.reward_token_amount = 0; // stable coin reward from the borrow fees set to zero after withdrawl
        deposit.reward_coin_amount = 0;  // sol rewards from the liquidated trove fees set to zero after withdrawl

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

        // let amount_to_burn = amount * 1_000_000_000;
        let amount_to_burn = amount * 1_00; // TODO only on devnet for 2 decimal token
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

    // TODO: function to create config
    pub fn initialize_config(ctx: Context<InitializeConfig>, _config_account_bump: u8, bump:u8, is_initialized:bool, mint_account_authority: Pubkey, admin_account_authority: Pubkey) -> ProgramResult {
        ctx.accounts.config_account.bump = bump;
        ctx.accounts.config_account.is_initialized = is_initialized;
        ctx.accounts.config_account.mint_account_authority = mint_account_authority;
        ctx.accounts.config_account.admin_account_authority = admin_account_authority;        
        Ok(())
    }

    pub fn change_config(ctx: Context<ChangeConfig>, is_initialized:bool, mint_account_authority: Pubkey, admin_account_authority: Pubkey) -> ProgramResult {
        ctx.accounts.config_account.is_initialized = is_initialized;
        ctx.accounts.config_account.mint_account_authority = mint_account_authority;
        ctx.accounts.config_account.admin_account_authority = admin_account_authority;        
        Ok(())
    }

    pub fn initialize_admin(ctx: Context<InitializeAdmin>, admin_account_bump:u8, admin_account_authority: Pubkey) -> ProgramResult {
        ctx.accounts.admin_account.bump = admin_account_bump;
        ctx.accounts.admin_account.authority = admin_account_authority;        
        Ok(())
    }

    pub fn initialize_token_acc(_ctx: Context<InitializeTokenAcc>, _mint_account_bump: u8) -> ProgramResult {
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
#[instruction(coin: u64, governance: u64, token: u64)]
pub struct AddDepositReward<'info> {
    #[account(mut)]
    pub admin_account_authority: Signer<'info>,

    #[account(mut, has_one = admin_account_authority, seeds = [b"config".as_ref(), admin_account_authority.key().to_bytes().as_ref()], bump = config.bump)]
    pub config: Account<'info, Config>,

    #[account(mut, seeds = [b"deposit", deposit.authority.to_bytes().as_ref()], bump = deposit.bump)]
    pub deposit: ProgramAccount<'info, Deposit>,
}
#[derive(Accounts)]
#[instruction(trove_account: Pubkey)]
pub struct ReceiveTrove<'info> {
    // #[account(mut)]
    // pub admin_account_authority: Signer<'info>,

    // #[account(mut, has_one = admin_account_authority, seeds = [b"config".as_ref(), admin_account_authority.key().to_bytes().as_ref()], bump = config.bump)]
    // pub config: Account<'info, Config>,

    #[account(mut, seeds = [b"borrowertrove".as_ref(), trove_account.key().to_bytes().as_ref()], bump = trove.bump)]
    pub trove: ProgramAccount<'info, Trove>,
}

#[derive(Accounts)]
#[instruction(mint_account_bump: u8, deposit_account_bump: u8, reward_vault_bump: u8)]
pub struct ClaimDepositReward<'info> {
    // only its respective depositor can claim its reward account 
    #[account(signer, mut)]
    pub authority: AccountInfo<'info>,

    #[account(mut, has_one = authority, seeds = [b"deposit".as_ref(), authority.key().to_bytes().as_ref()], bump = deposit_account_bump)]
    pub deposit: ProgramAccount<'info, Deposit>,

    #[account(
        seeds=[
            b"mint-authority"
        ],
        bump = mint_account_bump
    )]
    pub token_authority: AccountInfo<'info>,

    #[account(mut)]
    pub stable_coin: Account<'info, Mint>,

    #[account(mut)]
    pub user_token_account: Account<'info, TokenAccount>,

    #[account(address = spl_token::ID)]
    pub token_program: AccountInfo<'info>,

    #[account(mut, seeds = [b"rewardVault".as_ref()], bump = reward_vault_bump)]
    pub reward_coin_vault: AccountInfo<'info>,

    pub system_program: Program<'info, System>,
}

// TODO: Check if the bump matches later for deposit acc
#[derive(Accounts)]
#[instruction(amount: u64, mint_account_bump: u8, deposit_account_bump: u8)]
pub struct WithdrawDeposit<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(mut, has_one = authority, seeds = [b"deposit".as_ref(),authority.key().to_bytes().as_ref()], bump = deposit_account_bump)]
    pub deposit: ProgramAccount<'info, Deposit>,

    #[account(
        seeds=[
            b"mint-authority"
        ],
        bump = mint_account_bump
    )]
    pub token_authority: AccountInfo<'info>,

    #[account(mut)]
    pub stable_coin: Account<'info, Mint>,

    #[account(mut)]
    pub user_token: Account<'info, TokenAccount>,

    #[account(address = spl_token::ID)]
    pub token_program: AccountInfo<'info>,

}

// TODO: Check if the bump matches deposit for trove acc
#[derive(Accounts)]
#[instruction(amount: u64, deposit_account_bump: u8)]
pub struct AddDeposit<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(
        init_if_needed,
        seeds = [
            b"deposit",
            authority.key().to_bytes().as_ref(),
        ],
        bump = deposit_account_bump,
        payer = authority,
        space = Deposit::LEN + 8
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

    #[account(mut, seeds = [b"borrowertrove".as_ref(),authority.key().to_bytes().as_ref()], bump = trove.bump)]
    pub trove: ProgramAccount<'info, Trove>,

    #[account(signer, mut)]
    pub temp_lamport_account: AccountInfo<'info>,
}

#[derive(Accounts)]
#[instruction(amount: u64, trove_bump: u8)]
pub struct WithdrawCoin<'info> {
    #[account(signer, mut)]
    pub authority: AccountInfo<'info>,

    #[account(mut, seeds = [b"borrowertrove".as_ref(),authority.key.to_bytes().as_ref()], bump = trove.bump)]
    pub trove: ProgramAccount<'info, Trove>,

    #[account(mut, seeds = [b"solTrove".as_ref(),authority.key().to_bytes().as_ref()], bump = trove.sol_bump)]
    pub sol_trove: AccountInfo<'info>,


    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct RedeemCoin<'info> {
    #[account(signer, mut)]
    pub authority: AccountInfo<'info>,

    #[account(mut, seeds = [b"borrowertrove".as_ref(), authority.key.to_bytes().as_ref()], bump = trove.bump)]
    pub trove: ProgramAccount<'info, Trove>,
}

// TODO: Check if the bump matches later for trove acc
#[derive(Accounts)]
pub struct UpdateTrove<'info> {
    #[account(signer, mut)]
    pub authority: AccountInfo<'info>,

    #[account(mut, has_one = authority, seeds = [b"borrowertrove",authority.key().to_bytes().as_ref()], bump = trove.bump)]
    pub trove: ProgramAccount<'info, Trove>,

    #[account(mut, seeds = [b"solTrove".as_ref(),authority.key().to_bytes().as_ref()], bump = trove.sol_bump)]
    pub sol_trove: AccountInfo<'info>,


    #[account(address = spl_token::ID)]
    pub token_program: AccountInfo<'info>,

    #[account(mut)]
    pub user_token: Account<'info, TokenAccount>,

    #[account(mut)]
    pub token_mint: Account<'info, Mint>,
}

// TODO: Check if the bump matches later for trove acc
#[derive(Accounts)]
#[instruction(sol_account_bump:u8)]
pub struct CloseTrove<'info> {
    #[account(signer, mut)]
    pub authority: AccountInfo<'info>,

    #[account(mut, close = authority, seeds = [b"borrowertrove".as_ref(),authority.key().to_bytes().as_ref()], bump = trove.bump)]
    pub trove: ProgramAccount<'info, Trove>,

    #[account(mut, seeds = [b"solTrove".as_ref(),authority.key().to_bytes().as_ref()], bump = sol_account_bump)]
    pub sol_trove: AccountInfo<'info>,

    #[account(address = spl_token::ID)]
    pub token_program: AccountInfo<'info>,

    #[account(mut)]
    pub user_token: Account<'info, TokenAccount>,

    #[account(mut)]
    pub token_mint: Account<'info, Mint>,
}


#[derive(Accounts)]
#[instruction(trove_bump:u8)]
pub struct LiquidateTrove<'info> {

    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(
        mut,
        has_one = authority,
        seeds = [
            b"admin",
        ],
        bump = admin.bump,
    )]
    pub admin_account_authority: Account<'info, Admin>,
    
    #[account(mut, has_one = admin_account_authority, seeds = [b"config".as_ref(), admin_account_authority.key().to_bytes().as_ref()], bump = config.bump)]
    pub config: Account<'info, Config>,

    #[account(mut, close = admin_account_authority, seeds = [b"borrowertrove".as_ref(), trove.authority.key().to_bytes().as_ref()], bump = trove_bump)]
    pub trove: ProgramAccount<'info, Trove>,

}

#[derive(Accounts)]
#[instruction(borrow_amount: u64, lamports: u64, trove_account_bump: u8, sol_account_bump:u8, mint_account_bump:u8, fee_account_bump:u8, team_fee_account_bump:u8)]
pub struct Borrow<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(
        init_if_needed,
        seeds = [
            b"borrowertrove",
            authority.key().to_bytes().as_ref(),
        ],
        bump = trove_account_bump,
        payer = authority,
        space = Trove::LEN + 8
    )]
    pub trove_account: Account<'info, Trove>,

    #[account(init_if_needed, seeds = [b"solTrove",authority.key().to_bytes().as_ref()], bump = sol_account_bump, payer = authority, space =  0)]
    pub sol_trove: AccountInfo<'info>,

    #[account(
        init_if_needed,
        seeds = [
            b"fee"
        ],
        bump = fee_account_bump,
        payer = authority,
        space = Fee::LEN + 8
    )]
    pub fee_account: Account<'info, Fee>,

    #[account(
        init_if_needed,
        seeds = [
            b"teamfee"
        ],
        bump = team_fee_account_bump,
        payer = authority,
        space = Fee::LEN + 8
    )]
    pub team_fee_account: Account<'info, Fee>,

    #[account(
        seeds=[
            b"mint-authority"
        ],
        bump = mint_account_bump
    )]
    pub token_authority: AccountInfo<'info>,

    #[account(mut)]
    pub stable_coin: Account<'info, Mint>,

    #[account(mut)]
    pub user_token: Account<'info, TokenAccount>,

    pub pyth_sol_account: AccountInfo<'info>,

    pub system_program: Program<'info, System>,

    #[account(address = spl_token::ID)]
    pub token_program: AccountInfo<'info>,

    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
#[instruction( borrow_amount: u64, lamports: u64, mint_account_bump:u8)]
pub struct AddBorrow<'info> {
    #[account(signer, mut)]
    pub authority: AccountInfo<'info>,

    #[account(mut, seeds=[b"borrowertrove", authority.key.to_bytes().as_ref()], bump = trove.bump)]
    pub trove: ProgramAccount<'info, Trove>,

    #[account(mut, seeds = [b"solTrove".as_ref(),authority.key().to_bytes().as_ref()], bump = trove.sol_bump)]
    pub sol_trove: AccountInfo<'info>,

    #[account(
        seeds=[
            b"mint-authority".as_ref()
        ],
        bump = mint_account_bump
    )]
    pub token_authority: AccountInfo<'info>,

    #[account(mut)]
    pub stable_coin: Account<'info, Mint>,

    #[account(mut)]
    pub user_token: Account<'info, TokenAccount>,

    pub pyth_sol_account: AccountInfo<'info>,

    pub system_program: Program<'info, System>,

    #[account(address = spl_token::ID)]
    pub token_program: AccountInfo<'info>,

    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
#[instruction(mint_account_bump: u8)]
pub struct InitializeTokenAcc<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(
        init,
        seeds=[
            b"mint-authority"
        ],
        bump = mint_account_bump,
        space =  8 + 2 + 4 + 200 + 1,
        payer = authority 
    )]
    pub token_authority: AccountInfo<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(config_account_bump: u8)]
pub struct InitializeConfig<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(init, seeds = [b"config".as_ref(), authority.key().as_ref()], bump = config_account_bump, payer = authority, space = Config::LEN + 8)]
    pub config_account: Account<'info, Config>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(admin_account_bump: u8)]
pub struct InitializeAdmin<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(init, seeds = [b"admin".as_ref(), authority.key().as_ref()], bump = admin_account_bump, payer = authority, space = Admin::LEN + 8)]
    pub admin_account: Account<'info, Admin>,

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

#[account]
#[derive(Default, Debug)]
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

#[account]
#[derive(Default, Debug)]
pub struct Trove {
    pub bump: u8,
    pub sol_bump: u8,
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
    /// space = 8 + 8 + 1 + 1 + 1 + 8 + 8 + 8 + 8 + 8 + 32
    pub const LEN: usize = size_of::<Trove>() + 8;
}

#[account]
#[derive(Default, Debug)]
pub struct Fee {
    pub bump: u8,
    pub is_initialized: bool,
    pub sol_amount: u64,
}

impl Fee {
    /// space = 8 + 1 + 8 + 8 + 8
    pub const LEN: usize = size_of::<Fee>() + 8;
}

#[account]
#[derive(Default, Debug)]
pub struct Admin {
    pub bump: u8,
    pub authority: Pubkey,
}

impl Admin {
    /// space = 8 + 1 + 8
    pub const LEN: usize = size_of::<Deposit>() + 8;
}