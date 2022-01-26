use crate::{params::MIN_COLLATERAL, params::GAS_FEE};
use solana_program::native_token::lamports_to_sol;
use std::ops::Mul;
use crate::params::{DEPOSIT_FEE, TEAM_FEE, MIN_DEPOSIT_FEE, MIN_TEAM_FEE, TOTAL_FEE, MIN_TOTAL_FEE};

pub fn check_min_collateral_include_gas_fee(
    amount: u64,
    lamports: u64
) -> bool {
    get_lamport_price(lamports - GAS_FEE) / amount as f64 >= MIN_COLLATERAL
}

pub fn get_trove_sent_amount(
    amount: u64
) -> u64 {
    get_trove_debt_amount(amount)*1000 - get_depositors_fee(amount) - get_team_fee(amount)
}

pub fn add_fees_on_pay(amount: u64) -> u64{
    amount * 1000 + get_team_fee(amount) + get_depositors_fee(amount)
}

pub fn get_trove_debt_amount(
    amount: u64
) -> u64 {
    amount
    // TODO change this back with deducted gas fee
    //amount - GAS_FEE
}

pub fn get_depositors_fee(
    amount: u64
) -> u64 {
    let dep_fee = get_trove_debt_amount(amount) * (DEPOSIT_FEE);
    if dep_fee < MIN_DEPOSIT_FEE 
        {MIN_DEPOSIT_FEE}
     else 
        {dep_fee}
    
}

pub fn get_team_fee(
    amount: u64
) -> u64 {
    let team_fee = get_trove_debt_amount(amount) * (TEAM_FEE);
    if team_fee < MIN_TEAM_FEE 
       { MIN_TEAM_FEE }
     else 
       {team_fee }
    
}

pub fn get_total_fee(amount:u64) -> u64{
    let total_fee = amount * (TOTAL_FEE);
    if total_fee < (MIN_TOTAL_FEE) 
       { MIN_TOTAL_FEE }
     else 
       { total_fee }
    
}

fn get_lamport_price(lamports: u64) -> f64 {
    // TODO get price for lamports from oracle
    lamports_to_sol(lamports).mul(1000000000_f64)
}
