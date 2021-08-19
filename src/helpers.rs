use crate::{params::MIN_COLLATERAL, params::GAS_FEE};
use solana_program::native_token::lamports_to_sol;
use std::ops::Mul;

pub fn check_min_collateral_include_gas_fee(
    amount: u64,
    lamports: u64
) -> bool {
    lamports_to_sol(lamports - GAS_FEE) / lamports_to_sol(amount) as f64 >= MIN_COLLATERAL
}


fn get_lamport_price(lamports: u64) -> f64 {
    // TODO get price for lamports from oracle
    lamports_to_sol(lamports).mul(70.0 as f64)
}