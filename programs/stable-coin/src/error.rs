use anchor_lang::prelude::*;

#[error]
pub enum StableCoinError {
    #[msg("User does not have enough SOL")]
    NoEnough,
    #[msg("The SOL/USD price is wrong.")]
    UsdPriceWrong,
    #[msg("The account is not initialized.")]
    NoInit,
    #[msg("You don't deposit any SOL.")]
    NoDeposit,
    #[msg("Escrow account doesn't have enough SOL.")]
    NoEnoughSolEscrow,
}
