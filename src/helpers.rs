use crate::{params::MIN_COLLATERAL, params::GAS_FEE};
use solana_program::native_token::lamports_to_sol;
use std::ops::Mul;
use crate::params::{DEPOSIT_FEE, TEAM_FEE};

pub fn check_min_collateral_include_gas_fee(
    amount: u64,
    lamports: u64
) -> bool {
    get_lamport_price(lamports - GAS_FEE) / amount as f64 >= MIN_COLLATERAL
}

pub fn get_trove_sent_amount(
    amount: u64
) -> u64 {
    get_trove_debt_amount(amount) - get_depositors_fee(amount) - get_team_fee(amount)
}

pub fn get_trove_debt_amount(
    amount: u64
) -> u64 {
    amount - GAS_FEE
}

pub fn get_depositors_fee(
    amount: u64
) -> u64 {
    get_trove_debt_amount(amount) * (DEPOSIT_FEE) / 100
}

pub fn get_team_fee(
    amount: u64
) -> u64 {
    get_trove_debt_amount(amount) * (TEAM_FEE) / 100
}

fn get_lamport_price(lamports: u64) -> f64 {
    // TODO get price for lamports from oracle
    lamports_to_sol(lamports).mul(70.0 as f64)
}
