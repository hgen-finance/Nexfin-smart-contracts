use anchor_lang::prelude::*;
use anchor_lang::solana_program::{program_error::ProgramError, pubkey::Pubkey};

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
