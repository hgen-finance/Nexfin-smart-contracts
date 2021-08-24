use std::convert::TryInto;

use crate::error::LiquityError::InvalidInstruction;
use solana_program::{
    msg,
};
use crate::error::LiquityError;
use solana_program::program_error::ProgramError;

pub enum LiquityInstruction {

    /// Borrow money
    ///
    ///
    /// Accounts expected:
    ///
    /// 0. `[signer]` The account of the person taking the trade
    /// 1. `[writable]` The account to store trove
    /// 2. `[]` The rent sysvar
    Borrow {
        /// the amount the taker expects to be paid in the other token, as a u64 because that's the max possible supply of a token
        borrow_amount: u64,
        lamports: u64
    },

    /// Close Trove
    ///
    ///
    /// Accounts expected:
    ///
    /// 0. `[signer]` The account of the person taking the trade
    /// 1. `[writable]` The Trove account
    /// 2. `[]` Token program
    /// 3. `[]` User token acc
    /// 4. `[]` Mint Token key
    CloseTrove {},

    /// Liquidate Trove
    ///
    /// Accounts expected:
    ///
    /// 0. `[signer]` The account of the person taking the trade
    /// 1. `[writable]` The Trove account
    /// 2. `[writable]` The Trove owner
    LiquidateTrove {},

    /// Withdraw Coin
    ///
    /// Accounts expected:
    ///
    /// 0. `[signer]` The account of the person taking the trade
    /// 1. `[writable]` The Trove account
    WithdrawCoin {
        amount: u64,
    },

    /// Redeem Coin
    ///
    /// Accounts expected:
    ///
    /// 0. `[signer]` The account of the person taking the trade
    /// 1. `[writable]` The Trove account
    RedeemCoin {
        amount: u64,
    },

    /// Add Coin
    ///
    /// Accounts expected:
    ///
    /// 0. `[signer]` The account of the person taking the trade
    /// 1. `[writable]` The Trove account
    /// 2. `[writable]` The Temp Account to get lamports
    AddCoin {
        amount: u64,
    },

    /// Add deposit
    ///
    /// Accounts expected:
    ///
    /// 0. `[signer]` The account of the person taking the trade
    /// 1. `[writable]` The Deposit account
    /// 2. `[]` The rent sysvar
    /// 3. `[]` Token program
    /// 4. `[]` User token acc
    /// 4. `[]` User governance token acc
    /// 5. `[]` Mint Token key
    AddDeposit {
        amount: u64,
    },

    ///  Withdraw deposit
    ///
    /// Accounts expected:
    ///
    /// 0. `[signer]` The account of the person taking the trade
    /// 1. `[writable]` The Deposit account
    WithdrawDeposit {
        amount: u64
    },

    ///  Claim deposit reward
    ///
    /// Accounts expected:
    ///
    /// 0. `[signer]` The account of the person taking the trade
    /// 1. `[writable]` The Deposit account
    ClaimDepositReward {},


    /// Trove received
    ///
    ///
    /// Accounts expected:
    ///
    /// 0. `[signer]` Sys acc
    /// 1. `[writable]` The Trove account
    ReceiveTrove {},


    /// Set Deposit reward
    ///
    ///
    /// Accounts expected:
    ///
    /// 0. `[signer]` Sys acc
    /// 1. `[writable]` The Deposit account
    AddDepositReward {
        coin: u64,
        governance: u64,
        token: u64
    },
}



impl LiquityInstruction {
    /// Unpacks a byte buffer into a [EscrowInstruction](enum.EscrowInstruction.html).
    pub fn unpack(input: &[u8]) -> Result<Self, ProgramError> {
        let (tag, rest) = input.split_first().ok_or(InvalidInstruction)?;

        Ok(match tag {
            0 => {
                let (borrow_amount, rest) = Self::unpack_u64(rest)?;
                let (lamports, _rest) = Self::unpack_u64(rest)?;
                Self::Borrow {
                    borrow_amount,
                    lamports
                }
            },
            1 => {
                Self::CloseTrove {}
            },
            2 => {
                Self::LiquidateTrove {}
            },
            3 => {
                let (amount, _rest) = Self::unpack_u64(rest)?;
                Self::WithdrawCoin {
                    amount
                }
            },
            4 => {
                let (amount, _rest) = Self::unpack_u64(rest)?;
                Self::AddCoin {
                    amount
                }
            },
            5 => {
                let (amount, _rest) = Self::unpack_u64(rest)?;
                Self::RedeemCoin {
                    amount
                }
            },
            6 => {
                let (amount, _rest) = Self::unpack_u64(rest)?;
                Self::AddDeposit {
                    amount
                }
            },
            7 => {
                let (amount, _rest) = Self::unpack_u64(rest)?;
                Self::WithdrawDeposit {
                    amount
                }
            },
            8 => {
                Self::ClaimDepositReward {}
            },
            9 => {
                Self::ReceiveTrove {}
            },
            10 => {
                let (coin, rest) = Self::unpack_u64(rest)?;
                let (governance, rest) = Self::unpack_u64(rest)?;
                let (token, _rest) = Self::unpack_u64(rest)?;

                Self::AddDepositReward {
                    coin,
                    governance,
                    token
                }
            }
            _ => return Err(InvalidInstruction.into()),
        })
    }

    fn unpack_u64(input: &[u8]) -> Result<(u64, &[u8]), ProgramError> {
        if input.len() < 8 {
            msg!("u64 cannot be unpacked");
            return Err(LiquityError::InstructionUnpackError.into());
        }
        let (bytes, rest) = input.split_at(8);
        let value = bytes
            .get(..8)
            .and_then(|slice| slice.try_into().ok())
            .map(u64::from_le_bytes)
            .ok_or(LiquityError::InstructionUnpackError)?;
        Ok((value, rest))
    }
}