use solana_program::{
    account_info::{AccountInfo, next_account_info},
    entrypoint::ProgramResult,
    msg,
    program::{invoke, invoke_signed},
    program_error::ProgramError,
    program_pack::{IsInitialized, Pack},
    pubkey::Pubkey,
    sysvar::{rent::Rent, Sysvar},
};
use solana_program::system_instruction::create_account;
use spl_token::state::Account as TokenAccount;

use crate::{error::LiquityError, helpers, instruction::LiquityInstruction, state::Escrow, AUTHORITY_MINT};
use crate::state::{Trove, Deposit};
use std::ops::{Sub, Add};
use crate::helpers::{get_trove_sent_amount, sent_trove_fee_to_depositors, get_depositors_fee, get_team_fee, get_trove_debt_amount};
use solana_program::log::sol_log_64;
use crate::params::SYSTEM_ACCOUNT_ADDRESS;

pub struct Processor;

impl Processor {
    pub fn process(program_id: &Pubkey, accounts: &[AccountInfo], instruction_data: &[u8]) -> ProgramResult {
        let instruction = LiquityInstruction::unpack(instruction_data)?;

        match instruction {
            LiquityInstruction::InitEscrow { amount } => {
                msg!("Instruction: InitEscrow");
                Self::process_init_escrow(accounts, amount, program_id)
            }
            LiquityInstruction::Exchange { amount } => {
                msg!("Instruction Exchange");
                Self::process_exchange(accounts, amount, program_id)
            }
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
        }
    }

    fn process_receive_trove(
        accounts: &[AccountInfo],
        program_id: &Pubkey,
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
        program_id: &Pubkey,
    ) -> ProgramResult
    {
        let accounts_info_iter = &mut accounts.iter();
        let depositor = next_account_info(accounts_info_iter)?;

        if !depositor.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }

        let deposit_account = next_account_info(accounts_info_iter)?;

        let mut deposit = Deposit::unpack_unchecked(&deposit_account.data.borrow())?;

        if deposit.owner != *depositor.key {
            return Err(LiquityError::OnlyForDepositOwner.into());
        }

        deposit.reward_governance_token_amount = 0;
        deposit.reward_token_amount = 0;
        deposit.reward_coin_amount = 0;

        Deposit::pack(deposit, &mut deposit_account.data.borrow_mut())?;

        Ok(())
    }

    fn process_withdraw_deposit(
        accounts: &[AccountInfo],
        amount: u64,
        program_id: &Pubkey,
    ) -> ProgramResult
    {
        let accounts_info_iter = &mut accounts.iter();
        let depositor = next_account_info(accounts_info_iter)?;

        if !depositor.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }

        let deposit_account = next_account_info(accounts_info_iter)?;

        let mut deposit = Deposit::unpack_unchecked(&deposit_account.data.borrow())?;

        if deposit.owner != *depositor.key {
            return Err(LiquityError::OnlyForDepositOwner.into());
        }

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
        program_id: &Pubkey,
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

        if deposit.is_initialized {
            deposit.token_amount = deposit.token_amount.add(amount);
        } else {
            deposit.is_initialized = true;
            deposit.token_amount = amount;
            deposit.reward_token_amount = 0;
            deposit.reward_governance_token_amount = 0;
            deposit.reward_coin_amount = 0;
            deposit.owner = *depositor.key;
        }

        let token_program = next_account_info(accounts_info_iter)?;
        let temp_pda_token = next_account_info(accounts_info_iter)?;
        let token = next_account_info(accounts_info_iter)?;

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
        program_id: &Pubkey,
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
        program_id: &Pubkey,
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
        program_id: &Pubkey,
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

        let mut trove = Trove::unpack_unchecked(&trove_account.data.borrow())?;
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
        program_id: &Pubkey,
    ) -> ProgramResult
    {
        let accounts_info_iter = &mut accounts.iter();
        let borrower = next_account_info(accounts_info_iter)?;

        if !borrower.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }

        let trove_account = next_account_info(accounts_info_iter)?;

        let mut trove = Trove::unpack_unchecked(&trove_account.data.borrow())?;
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
        program_id: &Pubkey,
    ) -> ProgramResult {
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

    fn process_exchange(
        accounts: &[AccountInfo],
        amount_expected_by_taker: u64,
        program_id: &Pubkey,
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let taker = next_account_info(account_info_iter)?;

        if !taker.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }

        let takers_sending_token_account = next_account_info(account_info_iter)?;

        let takers_token_to_receive_account = next_account_info(account_info_iter)?;

        let pdas_temp_token_account = next_account_info(account_info_iter)?;
        let pdas_temp_token_account_info =
            TokenAccount::unpack(&pdas_temp_token_account.data.borrow())?;
        let (pda, bump_seed) = Pubkey::find_program_address(&[b"escrow"], program_id);

        if amount_expected_by_taker != pdas_temp_token_account_info.amount {
            return Err(LiquityError::ExpectedAmountMismatch.into());
        }

        let initializers_main_account = next_account_info(account_info_iter)?;
        let initializers_token_to_receive_account = next_account_info(account_info_iter)?;
        let escrow_account = next_account_info(account_info_iter)?;

        let escrow_info = Escrow::unpack(&escrow_account.data.borrow())?;

        if escrow_info.temp_token_account_pubkey != *pdas_temp_token_account.key {
            return Err(ProgramError::InvalidAccountData);
        }

        if escrow_info.initializer_pubkey != *initializers_main_account.key {
            return Err(ProgramError::InvalidAccountData);
        }

        if escrow_info.initializer_token_to_receive_account_pubkey != *initializers_token_to_receive_account.key {
            return Err(ProgramError::InvalidAccountData);
        }

        let token_program = next_account_info(account_info_iter)?;

        let transfer_to_initializer_ix = spl_token::instruction::transfer(
            token_program.key,
            takers_sending_token_account.key,
            initializers_token_to_receive_account.key,
            taker.key,
            &[&taker.key],
            escrow_info.expected_amount,
        )?;

        msg!("Calling the token program to transfer tokens to the escrow's initializer...");
        invoke(
            &transfer_to_initializer_ix,
            &[
                takers_sending_token_account.clone(),
                initializers_token_to_receive_account.clone(),
                taker.clone(),
                token_program.clone(),
            ],
        )?;
        let pda_account = next_account_info(account_info_iter)?;

        let transfer_to_taker_ix = spl_token::instruction::transfer(
            token_program.key,
            pdas_temp_token_account.key,
            takers_token_to_receive_account.key,
            &pda,
            &[&pda],
            pdas_temp_token_account_info.amount,
        )?;
        msg!("Calling the token program to transfer tokens to the taker...");
        invoke_signed(
            &transfer_to_taker_ix,
            &[
                pdas_temp_token_account.clone(),
                takers_token_to_receive_account.clone(),
                pda_account.clone(),
                token_program.clone(),
            ],
            &[&[&b"escrow"[..], &[bump_seed]]],
        )?;

        let close_pdas_temp_acc_ix = spl_token::instruction::close_account(
            token_program.key,
            pdas_temp_token_account.key,
            initializers_main_account.key,
            &pda,
            &[&pda],
        )?;
        msg!("Calling the token program to close pda's temp account...");
        invoke_signed(
            &close_pdas_temp_acc_ix,
            &[
                pdas_temp_token_account.clone(),
                initializers_main_account.clone(),
                pda_account.clone(),
                token_program.clone(),
            ],
            &[&[&b"escrow"[..], &[bump_seed]]],
        )?;

        msg!("Closing the escrow account...");
        **initializers_main_account.lamports.borrow_mut() = initializers_main_account.lamports()
            .checked_add(escrow_account.lamports())
            .ok_or(LiquityError::AmountOverflow)?;
        **escrow_account.lamports.borrow_mut() = 0;
        *escrow_account.data.borrow_mut() = &mut [];

        Ok(())
    }

    fn process_init_escrow(
        accounts: &[AccountInfo],
        amount: u64,
        program_id: &Pubkey,
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let initializer = next_account_info(account_info_iter)?;

        if !initializer.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }

        let temp_token_account = next_account_info(account_info_iter)?;

        let token_to_receive_account = next_account_info(account_info_iter)?;
        if *token_to_receive_account.owner != spl_token::id() {
            return Err(ProgramError::IncorrectProgramId);
        }

        let escrow_account = next_account_info(account_info_iter)?;
        let rent = &Rent::from_account_info(next_account_info(account_info_iter)?)?;

        if !rent.is_exempt(escrow_account.lamports(), escrow_account.data_len()) {
            return Err(LiquityError::NotRentExempt.into());
        }

        let mut escrow_info = Escrow::unpack_unchecked(&escrow_account.data.borrow())?;
        if escrow_info.is_initialized() {
            return Err(ProgramError::AccountAlreadyInitialized);
        }

        escrow_info.is_initialized = true;
        escrow_info.initializer_pubkey = *initializer.key;
        escrow_info.temp_token_account_pubkey = *temp_token_account.key;
        escrow_info.initializer_token_to_receive_account_pubkey = *token_to_receive_account.key;
        escrow_info.expected_amount = amount;

        Escrow::pack(escrow_info, &mut escrow_account.data.borrow_mut())?;
        let (pda, _bump_seed) = Pubkey::find_program_address(&[b"escrow"], program_id);

        let token_program = next_account_info(account_info_iter)?;
        let owner_change_ix = spl_token::instruction::set_authority(
            token_program.key,
            temp_token_account.key,
            Some(&pda),
            spl_token::instruction::AuthorityType::AccountOwner,
            initializer.key,
            &[&initializer.key],
        )?;

        msg!("Calling the token program to transfer token account ownership...");
        invoke(
            &owner_change_ix,
            &[
                temp_token_account.clone(),
                initializer.clone(),
                token_program.clone(),
            ],
        )?;

        Ok(())
    }

    fn process_redeem_coin(
        accounts: &[AccountInfo],
        amount: u64,
        program_id: &Pubkey,
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