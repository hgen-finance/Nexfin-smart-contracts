use solana_program::{
    account_info::{AccountInfo, next_account_info},
    entrypoint::ProgramResult,
    msg,
    program::{invoke},
    program_error::ProgramError,
    program_pack::{IsInitialized, Pack},
    pubkey::Pubkey,
    sysvar::{rent::Rent, Sysvar},
};
use crate::{error::LiquityError, helpers, instruction::LiquityInstruction};
use crate::state::{Trove, Deposit};
use std::ops::{Sub, Add};
use crate::helpers::{get_depositors_fee, get_team_fee, get_trove_debt_amount};
use crate::params::SYSTEM_ACCOUNT_ADDRESS;

pub struct Processor;

impl Processor {
    pub fn process(program_id: &Pubkey, accounts: &[AccountInfo], instruction_data: &[u8]) -> ProgramResult {
        let instruction = LiquityInstruction::unpack(instruction_data)?;

        match instruction {
            LiquityInstruction::Borrow { borrow_amount, lamports } => {
                msg!("Instruction Borrow");
                Self::process_borrow(accounts, borrow_amount, lamports, program_id)
            }
            LiquityInstruction::CloseTrove {} => {
                msg!("Instruction Close Trove");
                Self::process_close_trove(accounts, program_id)
            }
            LiquityInstruction::LiquidateTrove {} => {
                msg!("Instruction Liquidate Trove");
                Self::process_liquidate_trove(accounts, program_id)
            }
            LiquityInstruction::WithdrawCoin {amount} => {
                msg!("Instruction Withdraw Coin");
                Self::process_withdraw_coin(accounts, amount, program_id)
            }
            LiquityInstruction::AddCoin {amount} => {
                msg!("Instruction Add Coin");
                Self::process_add_coin(accounts, amount, program_id)
            }
            LiquityInstruction::RedeemCoin {amount} => {
                msg!("Instruction Redeem Coin");
                Self::process_redeem_coin(accounts, amount, program_id)
            }
            LiquityInstruction::AddDeposit {amount} => {
                msg!("Instruction Add Deposit");
                Self::process_add_deposit(accounts, amount, program_id)
            }
            LiquityInstruction::WithdrawDeposit {amount} => {
                msg!("Instruction Withdraw Deposit");
                Self::process_withdraw_deposit(accounts, amount, program_id)
            }
            LiquityInstruction::ClaimDepositReward {} => {
                msg!("Instruction Claim Deposit Reward");
                Self::process_claim_deposit_reward(accounts, program_id)
            }
            LiquityInstruction::ReceiveTrove {} => {
                msg!("Instruction Trove Tokens Received");
                Self::process_receive_trove(accounts, program_id)
            }
            LiquityInstruction::AddDepositReward {coin, governance, token} => {
                msg!("Instruction Add Deposit Reward");
                Self::process_add_deposit_reward(accounts, coin, governance, token, program_id)
            }
        }
    }

    fn process_add_deposit_reward(
        accounts: &[AccountInfo],
        coin: u64,
        governance: u64,
        token: u64,
        _program_id: &Pubkey,
    ) -> ProgramResult
    {
        let accounts_info_iter = &mut accounts.iter();
        let depositor = next_account_info(accounts_info_iter)?;

        if !depositor.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }

        if *depositor.key != SYSTEM_ACCOUNT_ADDRESS {
            return Err(ProgramError::MissingRequiredSignature);
        }

        let deposit_account = next_account_info(accounts_info_iter)?;

        let mut deposit = Deposit::unpack_unchecked(&deposit_account.data.borrow())?;

        deposit.reward_coin_amount = deposit.reward_coin_amount.add(coin);
        deposit.reward_governance_token_amount = deposit.reward_governance_token_amount.add(governance);
        deposit.reward_token_amount = deposit.reward_token_amount.add(token);

        Deposit::pack(deposit, &mut deposit_account.data.borrow_mut())?;

        Ok(())
    }

    fn process_receive_trove(
        accounts: &[AccountInfo],
        _program_id: &Pubkey,
    ) -> ProgramResult
    {
        let accounts_info_iter = &mut accounts.iter();
        let sys_acc = next_account_info(accounts_info_iter)?;

        if !sys_acc.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }

        if *sys_acc.key != SYSTEM_ACCOUNT_ADDRESS {
            return Err(ProgramError::MissingRequiredSignature);
        }

        let trove_account = next_account_info(accounts_info_iter)?;

        let mut trove = Trove::unpack_unchecked(&trove_account.data.borrow())?;
        if trove.is_liquidated {
            return Err(LiquityError::TroveAlreadyLiquidated.into());
        }

        trove.is_received = true;

        Trove::pack(trove, &mut trove_account.data.borrow_mut())?;

        Ok(())
    }

    fn process_claim_deposit_reward(
        accounts: &[AccountInfo],
        _program_id: &Pubkey,
    ) -> ProgramResult
    {
        let accounts_info_iter = &mut accounts.iter();
        let depositor = next_account_info(accounts_info_iter)?;

        if !depositor.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }

        if *depositor.key != SYSTEM_ACCOUNT_ADDRESS {
            return Err(ProgramError::MissingRequiredSignature);
        }

        let deposit_account = next_account_info(accounts_info_iter)?;

        let mut deposit = Deposit::unpack_unchecked(&deposit_account.data.borrow())?;

        deposit.reward_governance_token_amount = 0;
        deposit.reward_token_amount = 0;
        deposit.reward_coin_amount = 0;

        Deposit::pack(deposit, &mut deposit_account.data.borrow_mut())?;

        Ok(())
    }

    fn process_withdraw_deposit(
        accounts: &[AccountInfo],
        amount: u64,
        _program_id: &Pubkey,
    ) -> ProgramResult
    {
        let accounts_info_iter = &mut accounts.iter();

        let sys_acc = next_account_info(accounts_info_iter)?;

        if !sys_acc.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }

        if *sys_acc.key != SYSTEM_ACCOUNT_ADDRESS {
            return Err(ProgramError::MissingRequiredSignature);
        }

        let deposit_account = next_account_info(accounts_info_iter)?;

        let mut deposit = Deposit::unpack_unchecked(&deposit_account.data.borrow())?;

        if amount > deposit.token_amount {
            return Err(LiquityError::InsufficientLiquidity.into());
        }

        deposit.token_amount = deposit.token_amount.sub(amount);

        Deposit::pack(deposit, &mut deposit_account.data.borrow_mut())?;

        Ok(())
    }

    fn process_add_deposit(
        accounts: &[AccountInfo],
        amount: u64,
        _program_id: &Pubkey,
    ) -> ProgramResult
    {
        let accounts_info_iter = &mut accounts.iter();
        let depositor = next_account_info(accounts_info_iter)?;

        if !depositor.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }

        let deposit_account = next_account_info(accounts_info_iter)?;

        let rent = &Rent::from_account_info(next_account_info(accounts_info_iter)?)?;

        if !rent.is_exempt(deposit_account.lamports(), deposit_account.data_len()) {
            return Err(LiquityError::NotRentExempt.into());
        }

        let mut deposit = Deposit::unpack_unchecked(&deposit_account.data.borrow())?;

        let token_program = next_account_info(accounts_info_iter)?;
        let temp_pda_token = next_account_info(accounts_info_iter)?;
        let temp_governance_token = next_account_info(accounts_info_iter)?;
        let token = next_account_info(accounts_info_iter)?;

        if deposit.is_initialized {
            deposit.token_amount = deposit.token_amount.add(amount);
        } else {
            deposit.is_initialized = true;
            deposit.token_amount = amount;
            deposit.reward_token_amount = 0;
            deposit.reward_governance_token_amount = 0;
            deposit.reward_coin_amount = 0;
            deposit.bank = *temp_pda_token.key;
            deposit.governance_bank = *temp_governance_token.key;
            deposit.owner = *depositor.key;
        }

        let transfer_to_initializer_ix = spl_token::instruction::burn(
            token_program.key,
            temp_pda_token.key,
            token.key,
            depositor.key,
            &[&depositor.key],
            amount * 1000000000,
        )?;

        msg!("Calling the token program to transfer tokens to the escrow's initializer...");
        invoke(
            &transfer_to_initializer_ix,
            &[
                token.clone(),
                temp_pda_token.clone(),
                depositor.clone(),
                token_program.clone(),
            ],
        )?;

        Deposit::pack(deposit, &mut deposit_account.data.borrow_mut())?;

        Ok(())
    }

    fn process_add_coin(
        accounts: &[AccountInfo],
        amount: u64,
        _program_id: &Pubkey,
    ) -> ProgramResult
    {
        let accounts_info_iter = &mut accounts.iter();
        let borrower = next_account_info(accounts_info_iter)?;

        if !borrower.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }

        let trove_account = next_account_info(accounts_info_iter)?;

        let mut trove = Trove::unpack_unchecked(&trove_account.data.borrow())?;

        if !trove.is_initialized() {
            return Err(LiquityError::TroveIsNotInitialized.into());
        }
        if trove.is_liquidated {
            return Err(LiquityError::TroveAlreadyLiquidated.into());
        }
        if *borrower.key != trove.owner {
            return Err(LiquityError::OnlyForTroveOwner.into());
        }

        let temp_lamport_account = next_account_info(accounts_info_iter)?;

        if temp_lamport_account.lamports() != amount {
            return Err(LiquityError::ExpectedAmountMismatch.into());
        }

        trove.lamports_amount = trove.lamports_amount.add(amount);

        Trove::pack(trove, &mut trove_account.data.borrow_mut())?;

        Ok(())
    }

    fn process_withdraw_coin(
        accounts: &[AccountInfo],
        amount: u64,
        _program_id: &Pubkey,
    ) -> ProgramResult
    {
        let accounts_info_iter = &mut accounts.iter();
        let borrower = next_account_info(accounts_info_iter)?;

        if !borrower.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }

        let trove_account = next_account_info(accounts_info_iter)?;

        let mut trove = Trove::unpack_unchecked(&trove_account.data.borrow())?;

        if !trove.is_initialized() {
            return Err(LiquityError::TroveIsNotInitialized.into());
        }
        if trove.is_liquidated {
            return Err(LiquityError::TroveAlreadyLiquidated.into());
        }
        if *borrower.key != trove.owner {
            return Err(LiquityError::OnlyForTroveOwner.into());
        }

        trove.lamports_amount = trove.lamports_amount.sub(amount);

        if !helpers::check_min_collateral_include_gas_fee(trove.borrow_amount, trove.lamports_amount) {
            return Err(LiquityError::InvalidCollateral.into());
        }

        Trove::pack(trove, &mut trove_account.data.borrow_mut())?;

        Ok(())
    }

    fn process_liquidate_trove(
        accounts: &[AccountInfo],
        _program_id: &Pubkey,
    ) -> ProgramResult
    {
        let accounts_info_iter = &mut accounts.iter();
        let liquidator = next_account_info(accounts_info_iter)?;

        if !liquidator.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }

        let trove_account = next_account_info(accounts_info_iter)?;
        let sys_account = next_account_info(accounts_info_iter)?;

        if *sys_account.key != SYSTEM_ACCOUNT_ADDRESS {
            msg!("Invalid d");
            return Err(ProgramError::MissingRequiredSignature);
        }

        let trove = Trove::unpack_unchecked(&trove_account.data.borrow())?;
        if trove.is_liquidated {
            return Err(LiquityError::TroveAlreadyLiquidated.into());
        }

        if !trove.is_received {
            return Err(LiquityError::TroveIsNotReceived.into());
        }

        msg!("Send lamports to the sys acc");
        **sys_account.lamports.borrow_mut() = sys_account.lamports()
            .checked_add(trove_account.lamports())
            .ok_or(LiquityError::AmountOverflow)?;

        **trove_account.lamports.borrow_mut() = 0;
        *trove_account.data.borrow_mut() = &mut [];

        Ok(())
    }

    fn process_close_trove(
        accounts: &[AccountInfo],
        _program_id: &Pubkey,
    ) -> ProgramResult
    {
        let accounts_info_iter = &mut accounts.iter();
        let borrower = next_account_info(accounts_info_iter)?;

        if !borrower.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }

        let trove_account = next_account_info(accounts_info_iter)?;

        let trove = Trove::unpack_unchecked(&trove_account.data.borrow())?;
        if trove.is_liquidated {
            return Err(LiquityError::TroveAlreadyLiquidated.into());
        }

        let token_program = next_account_info(accounts_info_iter)?;
        let temp_pda_token = next_account_info(accounts_info_iter)?;
        let token = next_account_info(accounts_info_iter)?;

        let transfer_to_initializer_ix = spl_token::instruction::burn(
            token_program.key,
            temp_pda_token.key,
            token.key,
            borrower.key,
            &[&borrower.key],
            trove.amount_to_close * 1000000000,
        )?;

        msg!("Calling the token program to transfer tokens to the escrow's initializer...");
        invoke(
            &transfer_to_initializer_ix,
            &[
                token.clone(),
                temp_pda_token.clone(),
                borrower.clone(),
                token_program.clone(),
            ],
        )?;

        msg!("Send back the lamports!");
        **borrower.lamports.borrow_mut() = borrower.lamports()
            .checked_add(trove_account.lamports())
            .ok_or(LiquityError::AmountOverflow)?;

        **trove_account.lamports.borrow_mut() = 0;
        *trove_account.data.borrow_mut() = &mut [];

        Ok(())
    }

    fn process_borrow(
        accounts: &[AccountInfo],
        borrow_amount: u64,
        lamports: u64,
        _program_id: &Pubkey,
    ) -> ProgramResult
    {
        // check collateral
        if !helpers::check_min_collateral_include_gas_fee(borrow_amount, lamports) {
            return Err(LiquityError::InvalidCollateral.into());
        }

        // Check accounts
        let accounts_info_iter = &mut accounts.iter();
        let borrower = next_account_info(accounts_info_iter)?;

        if !borrower.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }

        let trove_account = next_account_info(accounts_info_iter)?;

        let rent = &Rent::from_account_info(next_account_info(accounts_info_iter)?)?;

        if !rent.is_exempt(trove_account.lamports(), trove_account.data_len()) {
            return Err(LiquityError::NotRentExempt.into());
        }

        // Create Trove
        let mut trove = Trove::unpack_unchecked(&trove_account.data.borrow())?;
        if trove.is_initialized() {
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

        Trove::pack(trove, &mut trove_account.data.borrow_mut())?;

        Ok(())
    }

    fn process_redeem_coin(
        accounts: &[AccountInfo],
        amount: u64,
        _program_id: &Pubkey,
    ) -> ProgramResult
    {
        let accounts_info_iter = &mut accounts.iter();
        let borrower = next_account_info(accounts_info_iter)?;

        if !borrower.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }

        let trove_account = next_account_info(accounts_info_iter)?;

        let mut trove = Trove::unpack_unchecked(&trove_account.data.borrow())?;

        if !trove.is_initialized() {
            return Err(LiquityError::TroveIsNotInitialized.into());
        }
        if trove.is_liquidated {
            return Err(LiquityError::TroveAlreadyLiquidated.into());
        }

        trove.lamports_amount = trove.lamports_amount.sub(amount);

        Trove::pack(trove, &mut trove_account.data.borrow_mut())?;

        Ok(())
    }
}