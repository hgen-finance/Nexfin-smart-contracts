#![cfg(feature = "test-bpf")]

mod helper;
use std::str::FromStr;

use anchor_lang::solana_program::{instruction::Instruction, pubkey::Pubkey, sysvar};
use anchor_lang::{InstructionData, ToAccountMetas};
use helper::process_and_assert_ok;
use solana_program_test::{processor, tokio, BanksClient, ProgramTest};
use solana_sdk::signature::{Keypair, Signer};
use solana_sdk::system_instruction;
use std::mem::size_of;

use nexfin_program::state::Trove;
#[tokio::test]
async fn test_trove() {
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
    let space = size_of::<Trove>();
    let create_ix = system_instruction::create_account(
        &payer.pubkey(),
        &trove.pubkey(),
        rent.minimum_balance(space),
        space as u64,
        &program_id,
    );
    process_and_assert_ok(&[create_ix], &payer, &[&payer, &trove], &mut banks_client).await;

    let inx = Instruction {
        program_id,
        accounts: nexfin_program::accounts::Borrow {
            authority: authority.pubkey(),
            trove: trove.pubkey(),
            rent: sysvar::rent::ID,
        }
        .to_account_metas(None),
        data: nexfin_program::instruction::Borrow {
            borrow_amount: 10,
            lamports: 100,
        }
        .data(),
    };

    process_and_assert_ok(&[inx], &payer, &[&payer, &authority], &mut banks_client).await;
}
