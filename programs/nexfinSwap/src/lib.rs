pub use solana_program;

pub mod constraints;
pub mod curve;
pub mod error;
pub mod instruction;
pub mod processor;
pub mod state;

#[cfg(not(feature = "no-entrypoint"))]
mod entrypoint;

solana_program::declare_id!("Exf39M5HifaYUkiYHkATR2ehMSwWMVsSshMgpXdbJHqn");

