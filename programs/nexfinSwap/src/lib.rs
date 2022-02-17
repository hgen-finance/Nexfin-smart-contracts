use anchor_lang::prelude::*;
use solana_program::{declare_id, pubkey::Pubkey};

pub mod constraints;
pub mod curve;
pub mod error;
pub mod instruction;
pub mod processor;
pub mod state;

// TODO: change this to the account address that the swap program will be deployed
declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");

