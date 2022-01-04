use crate::params::{DEPOSIT_FEE, TEAM_FEE};
use crate::{params::GAS_FEE, params::MIN_COLLATERAL};
use anchor_lang::solana_program::native_token::lamports_to_sol;
use std::ops::Mul;

pub fn check_min_collateral_include_gas_fee(amount: u64, lamports: u64) -> bool {
    get_lamport_price(lamports - GAS_FEE) / amount as f64 >= MIN_COLLATERAL
}

pub fn get_trove_sent_amount(amount: u64) -> u64 {
    get_trove_debt_amount(amount) - get_depositors_fee(amount) - get_team_fee(amount)
}

pub fn get_trove_debt_amount(amount: u64) -> u64 {
    amount
    // TODO change this back with deducted gas fee
    //amount - GAS_FEE
}

pub fn get_depositors_fee(amount: u64) -> u64 {
    get_trove_debt_amount(amount) * (DEPOSIT_FEE) / 1000
}

pub fn get_team_fee(amount: u64) -> u64 {
    get_trove_debt_amount(amount) * (TEAM_FEE) / 1000
}

fn get_lamport_price(lamports: u64) -> f64 {
    // TODO get price for lamports from oracle
    lamports_to_sol(lamports).mul(1000000000 as f64)
}
