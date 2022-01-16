use solana_program::{
    account_info::{AccountInfo, next_account_info},
    entrypoint::ProgramResult,
    msg,
    program::{invoke, invoke_signed},
    program_error::ProgramError,
    program_pack::{IsInitialized, Pack},
    pubkey::Pubkey,
    sysvar::{rent::Rent, Sysvar},
    system_instruction,
};
use crate::{error::LiquityError, helpers, instruction::LiquityInstruction};
use crate::state::{Trove, Deposit};
use std::ops::{Sub, Add};
use crate::helpers::{get_depositors_fee, get_team_fee, get_trove_debt_amount, get_trove_sent_amount};
use crate::params::{SYSTEM_ACCOUNT_ADDRESS};

use std::convert::TryInto;


pub struct Processor;

impl Processor {
    pub fn process(program_id: &Pubkey, accounts: &[AccountInfo], instruction_data: &[u8]) -> ProgramResult {
        let instruction = LiquityInstruction::unpack(instruction_data)?;

        match instruction {
            LiquityInstruction::Borrow { borrow_amount, lamports, bump_seed } => {
                msg!("Instruction Borrow");
                Self::process_borrow(accounts, borrow_amount, lamports, bump_seed, program_id)
            }
            LiquityInstruction::UpdateTrove{amount} => {
                msg!("Instruction Update Trove");
                Self::process_update_trove(accounts, amount, program_id)
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
            LiquityInstruction::WithdrawDeposit {amount, bump_seed} => {
                msg!("Instruction Withdraw Deposit");
                Self::process_withdraw_deposit(accounts, amount, bump_seed, program_id)
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
            LiquityInstruction::AddBorrow { borrow_amount, lamports, bump_seed } => {
                msg!("Instruction Add Borrow");
                Self::process_add_borrow(accounts, borrow_amount, lamports, bump_seed, program_id)
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
        bump_seed: u8,
        program_id: &Pubkey,
    ) -> ProgramResult
    {
        msg!("Trying to withraw the gens from the pool");
        let accounts_info_iter = &mut accounts.iter();       

        let token_program = next_account_info(accounts_info_iter)?;
        let mint_addr = next_account_info(accounts_info_iter)?;
        let token_mint_acc = next_account_info(accounts_info_iter)?;
        let depositor_acc_info = next_account_info(accounts_info_iter)?;
        let deposit_account = next_account_info(accounts_info_iter)?;
        let pda_mint = next_account_info(accounts_info_iter)?;

        // Checking if passed PDA and expected PDA are equal
        // TODO set the main wallet as seed and 3 seeds
        let signers_seeds: &[&[u8]; 2] = &[
            b"test",
            &[bump_seed],
        ];

        msg!("matching the passed pda");
        let pda = Pubkey::create_program_address(signers_seeds, program_id)?;

        if pda.ne(&pda_mint.key) {
            return Err(ProgramError::InvalidAccountData);
        }

        //  if !depositor.is_signer {
        //     return Err(ProgramError::MissingRequiredSignature);
        // }

        // if !sys_acc.is_signer {
        //     return Err(ProgramError::MissingRequiredSignature);
        // }

        // if *sys_acc.key != SYSTEM_ACCOUNT_ADDRESS {
        //     return Err(ProgramError::MissingRequiredSignature);
        // }

        let transfer_to_initializer_ix = spl_token::instruction::mint_to(
            token_program.key,
            mint_addr.key,
            token_mint_acc.key,
            pda_mint.key,
            &[],
            amount * 1_000_000_000,
        )?;

        let mut deposit = Deposit::unpack_unchecked(&deposit_account.data.borrow())?;

        if amount > deposit.token_amount {
            return Err(LiquityError::InsufficientLiquidity.into());
        }

        deposit.token_amount = deposit.token_amount.sub(amount);
        msg!("the new deposit token amount is {}", deposit.token_amount);

        Deposit::pack(deposit, &mut deposit_account.data.borrow_mut())?;

        msg!("Calling the token program to mint token to users wallet...");
        invoke_signed(
            &transfer_to_initializer_ix,
            &[
                mint_addr.clone(),
                token_mint_acc.clone(),
                pda_mint.clone(),
            ],
            &[&[b"test", &[bump_seed]]]
        )?;

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


    fn process_update_trove(
        accounts: &[AccountInfo],
        amount: u64,
        _program_id: &Pubkey
    ) -> ProgramResult 
    {
        let accounts_info_iter = &mut accounts.iter();
        let borrower = next_account_info(accounts_info_iter)?;

        if !borrower.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }

        let trove_account = next_account_info(accounts_info_iter)?;

        // make the trove mutable to update the trove amount
        let mut trove = Trove::unpack_unchecked(&trove_account.data.borrow())?;
        if trove.is_liquidated {
            return Err(LiquityError::TroveAlreadyLiquidated.into());
        }

        let token_program = next_account_info(accounts_info_iter)?;
        let temp_pda_token = next_account_info(accounts_info_iter)?;
        let token = next_account_info(accounts_info_iter)?;


        
        let transfer_to_initializer_ix = spl_token::instruction::burn(
            token_program.key,
            temp_pda_token.key, // token account key
            token.key, // token mint address key
            borrower.key, // authority key
            &[&borrower.key], // signer pub key
            amount * 1000000000
        )?;
        

        

        // update the amount to close price
        trove.amount_to_close = (trove.amount_to_close).sub(amount);

        msg!("the amount is {}", amount);
        msg!("amount to close is {}", trove.amount_to_close);

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

        // check if the trove accout amount is paid in full to send back back the deposited sol
        // if trove.amount_to_close == amount{
        //     msg!("Send back the lamports!");
        //     **borrower.lamports.borrow_mut() = borrower.lamports()
        //         .checked_add(trove_account.lamports())
        //         .ok_or(LiquityError::AmountOverflow)?;

        //     **trove_account.lamports.borrow_mut() = 0;

            
        //      *trove_account.data.borrow_mut() = &mut [];
        // }
        
        // pack the updated trove account data
        Trove::pack(trove, &mut trove_account.data.borrow_mut())?;

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

        // make the trove mutable to update the trove amount
        let trove = Trove::unpack_unchecked(&trove_account.data.borrow())?;
        if trove.is_liquidated {
            return Err(LiquityError::TroveAlreadyLiquidated.into());
        }

        let token_program = next_account_info(accounts_info_iter)?;
        let temp_pda_token = next_account_info(accounts_info_iter)?;
        let token = next_account_info(accounts_info_iter)?;

        msg!("the borrow key is {}", borrower.key);
        msg!("the token key is {}", token.key);
        msg!("the token temp key is {}", temp_pda_token.key);
        msg!("the token program key is {}", token_program.key);
        msg!("the amount to be closed is  {}", trove.amount_to_close);

        
        let transfer_to_initializer_ix = spl_token::instruction::burn(
            token_program.key,
            temp_pda_token.key, // token account key
            token.key, // token mint address key
            borrower.key, // authority key
            &[&borrower.key], // signer pub key
            trove.amount_to_close * 1000000000
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

    // TODO: check if the pda pass matches with the pda signing in the transaction before invoking signed instruction through PDA
    fn process_borrow(
        accounts: &[AccountInfo],
        borrow_amount: u64,
        lamports: u64,
        bump_seed: u8,
        program_id: &Pubkey,
    ) -> ProgramResult
    {

        // check collateral
        if !helpers::check_min_collateral_include_gas_fee(borrow_amount, lamports) {
            return Err(LiquityError::InvalidCollateral.into());
        }

        // const ACCOUNT_DATA_LEN: usize = 1; // space for the account

        // Check accounts
        let accounts_info_iter = &mut accounts.iter();
        let borrower = next_account_info(accounts_info_iter)?;

        if !borrower.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }

        let trove_account = next_account_info(accounts_info_iter)?;

        let rent = &Rent::from_account_info(next_account_info(accounts_info_iter)?)?;

        let token_program = next_account_info(accounts_info_iter)?;
        let mint_addr = next_account_info(accounts_info_iter)?;
        let token_mint_acc = next_account_info(accounts_info_iter)?;
        let pda_mint = next_account_info(accounts_info_iter)?;

        if !rent.is_exempt(trove_account.lamports(), trove_account.data_len()) {
            return Err(LiquityError::NotRentExempt.into());
        }

        msg!("the accounts info is {:?}", pda_mint);

        // Checking if passed PDA and expected PDA are equal
        // TODO set the main wallet as seed
        let signers_seeds: &[&[u8]; 2] = &[
            b"test",
            &[bump_seed],
        ];

        msg!("matching the passed pda");
        let pda = Pubkey::create_program_address(signers_seeds, program_id)?;

        if pda.ne(&pda_mint.key) {
            return Err(ProgramError::InvalidAccountData);
        }

        // used to create a pda account (Check this)
        // Assessing required lamports and creating transaction instruction
        // let lamports_required = Rent::get()?.minimum_balance(ACCOUNT_DATA_LEN);
        // let create_pda_account_ix = system_instruction::create_account(
        //     &borrower.key,
        //     &pda_mint.key,
        //     lamports_required,
        //     ACCOUNT_DATA_LEN.try_into().unwrap(),
        //     &program_id,
        // );
        // // Invoking the instruction but with PDAs as additional signer
        // invoke_signed(
        //     &create_pda_account_ix,
        //     &[
        //         borrower.clone(),
        //         pda_mint.clone(),
        //         sys_program.clone(),
        //     ],
        //     &[signers_seeds],
        // )?;

        let transfer_to_initializer_ix = spl_token::instruction::mint_to(
            token_program.key,
            mint_addr.key,
            token_mint_acc.key,
            pda_mint.key,
            &[],
            get_trove_sent_amount(borrow_amount)* 1_000_000,
        )?;

        

        // // Create Trove
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

        msg!("trove owner is {}", trove.owner);
        msg!("the borrow amount is {}", trove.borrow_amount);
        Trove::pack(trove, &mut trove_account.data.borrow_mut())?;

        msg!("Calling the token program to mint token to users wallet...");
        invoke_signed(
            &transfer_to_initializer_ix,
            &[
                mint_addr.clone(),
                token_mint_acc.clone(),            
                pda_mint.clone(),
            ],
            &[signers_seeds] // passing bump seed to reduce the compute load
        )?;

        Ok(())
    
    }

    fn process_add_borrow(
        accounts: &[AccountInfo],
        borrow_amount: u64,
        lamports: u64,
        bump_seed: u8,
        _program_id: &Pubkey,
    ) -> ProgramResult
    {

        let accounts_info_iter = &mut accounts.iter();
        let borrower = next_account_info(accounts_info_iter)?;

        if !borrower.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }

        let trove_account = next_account_info(accounts_info_iter)?;
        msg!("the trove account data is {:?}", trove_account);
        let rent = &Rent::from_account_info(next_account_info(accounts_info_iter)?)?;

        if !rent.is_exempt(trove_account.lamports(), trove_account.data_len()) {
            return Err(LiquityError::NotRentExempt.into());
        }

        let token_program = next_account_info(accounts_info_iter)?;
        let mint_addr = next_account_info(accounts_info_iter)?;
        let token_mint_acc = next_account_info(accounts_info_iter)?;
        let pda_mint = next_account_info(accounts_info_iter)?;

         // Checking if passed PDA and expected PDA are equal
         // TODO set the main wallet as seed
        let signers_seeds: &[&[u8]; 2] = &[
            b"test",
            &[bump_seed],
        ];

        msg!("matching the passed pda");
        msg!("the program id is {:?}", _program_id);
        let pda = Pubkey::create_program_address(signers_seeds, _program_id)?;

        msg!("the client pda is {:?}", &pda_mint.key);
        msg!("the program pda is {:?}", &pda);
        if pda.ne(&pda_mint.key) {
            return Err(ProgramError::InvalidAccountData);
        }

        msg!("reached here");

        let transfer_to_initializer_ix = spl_token::instruction::mint_to(
            token_program.key,
            mint_addr.key,
            token_mint_acc.key,
            pda_mint.key,
            &[pda_mint.key],
            get_trove_sent_amount(borrow_amount)* 1_000_000,
        )?;

        

        let mut trove = Trove::unpack_unchecked(&trove_account.data.borrow())?;

        msg!("this is working !!");
        if trove.is_liquidated {
            return Err(LiquityError::TroveAlreadyLiquidated.into());
        }

        msg!("the borrow key is {}", *borrower.key);
        msg!("the trove owner key is {}", trove.owner);
        if *borrower.key != trove.owner {
            return Err(LiquityError::OnlyForTroveOwner.into());
        }

        let _temp_borrowed_amount = trove.amount_to_close;

        trove.lamports_amount = trove.lamports_amount.add(lamports);
        trove.amount_to_close = trove.amount_to_close.add(get_trove_debt_amount(borrow_amount));
        trove.borrow_amount = trove.borrow_amount.add(borrow_amount);
        trove.lamports_amount = trove.lamports_amount;
        trove.depositor_fee = trove.depositor_fee.add(get_depositors_fee(borrow_amount));
        trove.team_fee = trove.team_fee.add(get_team_fee(borrow_amount));

        Trove::pack(trove, &mut trove_account.data.borrow_mut())?;

        msg!("Calling the token program to mint token to users wallet...");
        invoke_signed(
            &transfer_to_initializer_ix,
            &[
                mint_addr.clone(),
                token_mint_acc.clone(),            
                pda_mint.clone(),
            ],
            &[signers_seeds] // passing bump seed to reduce the compute load
        )?;

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