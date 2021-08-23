use solana_program::pubkey::Pubkey;

pub const MIN_COLLATERAL: f64 = 1.10;
/// 2 SOL as gase fee
pub const GAS_FEE: u64 = 200;

pub const TOTAL_FEE: u64 = DEPOSIT_FEE + TEAM_FEE;
pub const DEPOSIT_FEE: u64 = 4;
pub const TEAM_FEE: u64 = 1;

pub const GENS_TOKEN_ADDRESS: &str = "BCftECVv4u3XxqvBdWiG15iubdixbP6BvdX4hHXtLk7c";