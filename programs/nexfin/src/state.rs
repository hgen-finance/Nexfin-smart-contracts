use anchor_lang::prelude::*;
use anchor_lang::solana_program::{program_error::ProgramError, pubkey::Pubkey};
use std::mem::size_of;

#[account]
#[derive(Default, Debug)]
pub struct Deposit {
    pub is_initialized: bool,
    pub token_amount: u64,
    pub reward_token_amount: u64,
    pub reward_governance_token_amount: u64,
    pub reward_coin_amount: u64,
    pub bank: Pubkey,
    pub governance_bank: Pubkey,
    pub owner: Pubkey,
}

impl Deposit {
    /// space = 8 + 1 + 8 + 8 + 8 + 8 + 32 + 32 + 32
    pub const LEN: usize = size_of::<Deposit>() + 8;
}

#[account]
#[derive(Default, Debug)]
pub struct Trove {
    pub is_initialized: bool,
    pub is_received: bool,
    pub is_liquidated: bool,
    pub borrow_amount: u64,
    pub lamports_amount: u64,
    pub team_fee: u64,
    pub depositor_fee: u64,
    pub amount_to_close: u64,
    pub owner: Pubkey,
}

impl Trove {
    /// space = 8 + 1 + 1 + 1 + 8 + 8 + 8 + 8 + 8 + 32
    pub const LEN: usize = size_of::<Trove>() + 8;
}

#[account]
#[derive(Default, Debug)]
pub struct Price {
    pub price: i64,
}

#[account]
#[derive(Default, Debug)]
pub struct Escrow {
    pub is_initialized: bool,
    pub initializer_pubkey: Pubkey,
    pub temp_token_account_pubkey: Pubkey,
    pub initializer_token_to_receive_account_pubkey: Pubkey,
    pub expected_amount: u64,
}
