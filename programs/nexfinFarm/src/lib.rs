use anchor_lang::prelude::*;

declare_id!("2YpiK1GJ9H7hMbjGFZRrhYPXFPPR6bg1LWoxW1YGQJiD");

#[program]
pub mod nexfin_farm {
    use super::*;
    pub fn initialize(ctx: Context<Initialize>) -> ProgramResult {
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize {}
