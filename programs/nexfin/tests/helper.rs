#![cfg(feature = "test-bpf")]

use anchor_lang::solana_program::instruction::Instruction;
use assert_matches::assert_matches;
use solana_program_test::BanksClient;
use solana_sdk::signature::{Keypair, Signer};
use solana_sdk::transaction::Transaction;

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
