use anchor_lang::prelude::*;

#![forbid(unsafe_code)]
pub mod farm;
pub mod id;
pub mod instruction;
pub mod log;
pub mod math;
pub mod pack;
pub mod pool;
pub mod program;
pub mod refdb;
pub mod string;
pub mod token;
pub mod traits;
pub mod vault;

declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");

#[program]
pub mod nexfin_sdk {
    use super::*;
    pub fn initialize(ctx: Context<Initialize>) -> ProgramResult {
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize {}



