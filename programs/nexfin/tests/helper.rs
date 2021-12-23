#![cfg(feature = "test-bpf")]

use anchor_lang::prelude::Pubkey;
use anchor_lang::solana_program::instruction::Instruction;
use assert_matches::assert_matches;
use solana_program_test::BanksClient;
use solana_sdk::signature::{Keypair, Signer};
use solana_sdk::transaction::Transaction;

use anchor_lang::{InstructionData, ToAccountMetas};
use nexfin_program::helpers::{get_depositors_fee, get_team_fee, get_trove_debt_amount};
use nexfin_program::state::Trove;
use solana_program_test::{processor, tokio, ProgramTest};
use solana_sdk::{system_instruction, system_program};
use std::mem::size_of;
use std::str::FromStr;

pub async fn process_and_assert_ok(
    instructions: &[Instruction],
    payer: &Keypair,
    signers: &[&Keypair],
    banks_client: &mut BanksClient,
) {
    let recent_blockhash = banks_client.get_latest_blockhash().await.unwrap();

    let mut all_signers = vec![payer];
    all_signers.extend_from_slice(signers);

    let tx = Transaction::new_signed_with_payer(
        &instructions,
        Some(&payer.pubkey()),
        &all_signers,
        recent_blockhash,
    );

    assert_matches!(banks_client.process_transaction(tx).await, Ok(()));
}

pub struct InitResult {
    pub program_id: Pubkey,
    payer: [u8; 64],
    authority: [u8; 64],
    trove: [u8; 64],
    pub banks_client: BanksClient,
}

impl InitResult {
    pub fn authority(&self) -> Keypair {
        Keypair::from_bytes(self.authority.as_ref()).unwrap()
    }
    pub fn payer(&self) -> Keypair {
        Keypair::from_bytes(self.payer.as_ref()).unwrap()
    }

    pub fn trove(&self) -> Keypair {
        Keypair::from_bytes(self.trove.as_ref()).unwrap()
    }
}

pub async fn setup() -> InitResult {
    let program_id = Pubkey::from_str("g6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS").unwrap();
    let program_test = ProgramTest::new(
        "nexfin_program",
        program_id,
        processor!(nexfin_program::entry),
    );

    let (mut banks_client, payer, _) = program_test.start().await;

    let authority = Keypair::new();
    let trove = Keypair::new();

    let rent = banks_client.get_rent().await.unwrap();
    let space = size_of::<Trove>() as u64 + 8;
    let create_ix = system_instruction::create_account(
        &payer.pubkey(),
        &trove.pubkey(),
        rent.minimum_balance(space as usize),
        space,
        &program_id,
    );
    process_and_assert_ok(&[create_ix], &payer, &[&payer, &trove], &mut banks_client).await;

    InitResult {
        program_id,
        authority: authority.to_bytes(),
        banks_client,
        trove: trove.to_bytes(),
        payer: payer.to_bytes(),
    }
}
