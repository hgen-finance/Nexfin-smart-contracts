use anchor_lang::prelude::*;

#[error]
pub enum StableCoinError {
    #[msg("User does not have enough SOL")]
    NoEnough,
    #[msg("The SOL/USD price is wrong.")]
    UsdPriceWrong,
}
