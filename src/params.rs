use solana_program::pubkey::Pubkey;

pub const MIN_COLLATERAL: f64 = 0.4;
/// 2 SOL as gase fee
pub const GAS_FEE: u64 = 1;

// Update the team fee and depositor fee
pub const TOTAL_FEE: u64 = DEPOSIT_FEE + TEAM_FEE;
pub const DEPOSIT_FEE: u64 = 4;
pub const TEAM_FEE: u64 = 1;

pub const MIN_DEPOSIT_FEE: u64 = 4000;
pub const MIN_TEAM_FEE: u64 = 1000;

pub const GENS_TOKEN_ADDRESS: &str = "2U3Mf4umT4CpLhhdwpfmGiktyvhdrLrNNv4z4GgsXNMe";

pub const SYSTEM_ACCOUNT_ADDRESS: Pubkey = Pubkey::new_from_array([240,128,137,181,181,244,178,11,202,92,41,67,29,30,142,34,115,81,243,143,175,219,59,238,174,103,9,243,15,126,161,190]);
