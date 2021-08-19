use solana_program::{
    program_pack::{IsInitialized, Pack, Sealed},
    program_error::ProgramError,
    pubkey::Pubkey,
};
use arrayref::{array_mut_ref, array_ref, array_refs, mut_array_refs};
use std::io::Read;
use std::convert::TryInto;
use solana_program::log::sol_log_compute_units;

fn slice_to_arr(chunk: &[u8]) -> &[u8; 32] {
    chunk.try_into().expect("slice with incorrect length")
}

pub struct Trove {
    pub is_initialized: bool,
    pub is_liquidated: bool,
    pub borrow_amount: u64,
    pub lamports_amount: u64,
    pub owner: Pubkey,
}

impl Sealed for Trove {}

impl IsInitialized for Trove {
    fn is_initialized(&self) -> bool {
        self.is_initialized
    }
}

impl Pack for Trove {
    const LEN: usize = 50;
    fn unpack_from_slice(src: &[u8]) -> Result<Self, ProgramError> {
        let src = array_ref![src, 0, Trove::LEN];
        let (
            is_initialized,
            is_liquidated,
            borrow_amount,
            lamports_amount,
            owner,
        ) = array_refs![src, 1, 1, 8, 8, 32];
        let is_initialized = match is_initialized {
            [0] => false,
            [1] => true,
            _ => return Err(ProgramError::InvalidAccountData),
        };

        let is_liquidated = match is_liquidated {
            [0] => false,
            [1] => true,
            _ => return Err(ProgramError::InvalidAccountData),
        };

        Ok(Trove {
            is_initialized,
            is_liquidated,
            borrow_amount: u64::from_le_bytes(*borrow_amount),
            lamports_amount: u64::from_le_bytes(*lamports_amount),
            owner: Pubkey::new_from_array(*owner),
        })
    }

    fn pack_into_slice(&self, dst: &mut [u8]) {
        let dst = array_mut_ref![dst, 0, Trove::LEN];
        let (
            is_initialized_dst,
            is_liquidated_dst,
            borrow_amount_dst,
            lamports_amount_dst,
            owner_dst,
        ) = mut_array_refs![dst,  1, 1, 8, 8, 32];

        let Trove {
            is_initialized,
            is_liquidated,
            borrow_amount,
            lamports_amount,
            owner,
        } = self;

        is_initialized_dst[0] = *is_initialized as u8;
        is_liquidated_dst[0] = *is_liquidated as u8;
        *borrow_amount_dst = borrow_amount.to_le_bytes();
        *lamports_amount_dst = lamports_amount.to_le_bytes();
        owner_dst.copy_from_slice(owner.as_ref());
    }
}

pub struct Escrow {
    pub is_initialized: bool,
    pub initializer_pubkey: Pubkey,
    pub temp_token_account_pubkey: Pubkey,
    pub initializer_token_to_receive_account_pubkey: Pubkey,
    pub expected_amount: u64,
}

impl Sealed for Escrow {}

impl IsInitialized for Escrow {
    fn is_initialized(&self) -> bool {
        self.is_initialized
    }
}

impl Pack for Escrow {
    const LEN: usize = 105;
    fn unpack_from_slice(src: &[u8]) -> Result<Self, ProgramError> {
        let src = array_ref![src, 0, Escrow::LEN];
        let (
            is_initialized,
            initializer_pubkey,
            temp_token_account_pubkey,
            initializer_token_to_receive_account_pubkey,
            expected_amount,
        ) = array_refs![src, 1, 32, 32, 32, 8];
        let is_initialized = match is_initialized {
            [0] => false,
            [1] => true,
            _ => return Err(ProgramError::InvalidAccountData),
        };

        Ok(Escrow {
            is_initialized,
            initializer_pubkey: Pubkey::new_from_array(*initializer_pubkey),
            temp_token_account_pubkey: Pubkey::new_from_array(*temp_token_account_pubkey),
            initializer_token_to_receive_account_pubkey: Pubkey::new_from_array(*initializer_token_to_receive_account_pubkey),
            expected_amount: u64::from_le_bytes(*expected_amount),
        })
    }

    fn pack_into_slice(&self, dst: &mut [u8]) {
        let dst = array_mut_ref![dst, 0, Escrow::LEN];
        let (
            is_initialized_dst,
            initializer_pubkey_dst,
            temp_token_account_pubkey_dst,
            initializer_token_to_receive_account_pubkey_dst,
            expected_amount_dst,
        ) = mut_array_refs![dst, 1, 32, 32, 32, 8];

        let Escrow {
            is_initialized,
            initializer_pubkey,
            temp_token_account_pubkey,
            initializer_token_to_receive_account_pubkey,
            expected_amount,
        } = self;

        is_initialized_dst[0] = *is_initialized as u8;
        initializer_pubkey_dst.copy_from_slice(initializer_pubkey.as_ref());
        temp_token_account_pubkey_dst.copy_from_slice(temp_token_account_pubkey.as_ref());
        initializer_token_to_receive_account_pubkey_dst.copy_from_slice(initializer_token_to_receive_account_pubkey.as_ref());
        *expected_amount_dst = expected_amount.to_le_bytes();
    }
}