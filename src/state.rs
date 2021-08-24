use solana_program::{
    program_pack::{IsInitialized, Pack, Sealed},
    program_error::ProgramError,
    pubkey::Pubkey,
};
use arrayref::{array_mut_ref, array_ref, array_refs, mut_array_refs};
use std::convert::TryInto;

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

impl Sealed for Deposit {}

impl IsInitialized for Deposit {
    fn is_initialized(&self) -> bool {
        self.is_initialized
    }
}

impl Pack for Deposit {
    const LEN: usize = 129;
    fn unpack_from_slice(src: &[u8]) -> Result<Self, ProgramError> {
        let src = array_ref![src, 0, Deposit::LEN];
        let (
            is_initialized,
            token_amount,
            reward_token_amount,
            reward_governance_token_amount,
            reward_coin_amount,
            bank,
            governance_bank,
            owner,
        ) = array_refs![src, 1, 8, 8, 8, 8, 32, 32, 32];
        let is_initialized = match is_initialized {
            [0] => false,
            [1] => true,
            _ => return Err(ProgramError::InvalidAccountData),
        };

        Ok(Deposit {
            is_initialized,
            token_amount: u64::from_le_bytes(*token_amount),
            reward_token_amount: u64::from_le_bytes(*reward_token_amount),
            reward_governance_token_amount: u64::from_le_bytes(*reward_governance_token_amount),
            reward_coin_amount: u64::from_le_bytes(*reward_coin_amount),
            bank: Pubkey::new_from_array(*bank),
            governance_bank: Pubkey::new_from_array(*governance_bank),
            owner: Pubkey::new_from_array(*owner),
        })
    }

    fn pack_into_slice(&self, dst: &mut [u8]) {
        let dst = array_mut_ref![dst, 0, Deposit::LEN];
        let (
            is_initialized_dst,
            token_amount_dst,
            reward_token_amount_dst,
            reward_governance_token_amount_dst,
            reward_coin_amount_dst,
            bank_dst,
            governance_bank_dst,
            owner_dst,
        ) = mut_array_refs![dst, 1, 8, 8, 8, 8, 32, 32, 32];

        let Deposit {
            is_initialized,
            token_amount,
            reward_token_amount,
            reward_governance_token_amount,
            reward_coin_amount,
            bank,
            governance_bank,
            owner,
        } = self;

        is_initialized_dst[0] = *is_initialized as u8;
        *token_amount_dst = token_amount.to_le_bytes();
        *reward_token_amount_dst = reward_token_amount.to_le_bytes();
        *reward_governance_token_amount_dst = reward_governance_token_amount.to_le_bytes();
        *reward_coin_amount_dst = reward_coin_amount.to_le_bytes();
        owner_dst.copy_from_slice(owner.as_ref());
        bank_dst.copy_from_slice(bank.as_ref());
        governance_bank_dst.copy_from_slice(governance_bank.as_ref());
    }
}

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

impl Sealed for Trove {}

impl IsInitialized for Trove {
    fn is_initialized(&self) -> bool {
        self.is_initialized
    }
}

impl Pack for Trove {
    const LEN: usize = 75;
    fn unpack_from_slice(src: &[u8]) -> Result<Self, ProgramError> {
        let src = array_ref![src, 0, Trove::LEN];
        let (
            is_initialized,
            is_received,
            is_liquidated,
            borrow_amount,
            lamports_amount,
            team_fee,
            depositor_fee,
            amount_to_close,
            owner,
        ) = array_refs![src, 1, 1, 1, 8, 8, 8, 8, 8, 32];
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

        let is_received = match is_received {
            [0] => false,
            [1] => true,
            _ => return Err(ProgramError::InvalidAccountData),
        };

        Ok(Trove {
            is_initialized,
            is_received,
            is_liquidated,
            borrow_amount: u64::from_le_bytes(*borrow_amount),
            lamports_amount: u64::from_le_bytes(*lamports_amount),
            team_fee: u64::from_le_bytes(*team_fee),
            depositor_fee: u64::from_le_bytes(*depositor_fee),
            amount_to_close: u64::from_le_bytes(*amount_to_close),
            owner: Pubkey::new_from_array(*owner),
        })
    }

    fn pack_into_slice(&self, dst: &mut [u8]) {
        let dst = array_mut_ref![dst, 0, Trove::LEN];
        let (
            is_initialized_dst,
            is_received_dst,
            is_liquidated_dst,
            borrow_amount_dst,
            lamports_amount_dst,
            team_fee_dst,
            depositor_fee_dst,
            amount_to_close_dst,
            owner_dst,
        ) = mut_array_refs![dst,  1, 1, 1, 8, 8, 8, 8, 8, 32];

        let Trove {
            is_initialized,
            is_received,
            is_liquidated,
            borrow_amount,
            lamports_amount,
            team_fee,
            depositor_fee,
            amount_to_close,
            owner,
        } = self;

        is_initialized_dst[0] = *is_initialized as u8;
        is_received_dst[0] = *is_received as u8;
        is_liquidated_dst[0] = *is_liquidated as u8;
        *borrow_amount_dst = borrow_amount.to_le_bytes();
        *lamports_amount_dst = lamports_amount.to_le_bytes();
        *team_fee_dst = team_fee.to_le_bytes();
        *depositor_fee_dst = depositor_fee.to_le_bytes();
        *amount_to_close_dst = amount_to_close.to_le_bytes();
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