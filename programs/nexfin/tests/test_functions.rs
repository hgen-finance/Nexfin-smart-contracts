#![cfg(feature = "test-bpf")]

mod helper;
use anchor_lang::solana_program::{instruction::Instruction, pubkey::Pubkey, sysvar};
use anchor_lang::AccountDeserialize;
use anchor_lang::{InstructionData, ToAccountMetas};
use helper::process_and_assert_ok;
use solana_program_test::{processor, tokio, BanksClient, ProgramTest};
use solana_sdk::signature::{Keypair, Signer};
use solana_sdk::{system_instruction, system_program};
use std::mem::size_of;
use std::str::FromStr;

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
    let space = size_of::<Trove>() as u64 + 8;
    let create_ix = system_instruction::create_account(
        &payer.pubkey(),
        &trove.pubkey(),
        rent.minimum_balance(space as usize),
        space,
        &program_id,
    );
    process_and_assert_ok(&[create_ix], &payer, &[&payer, &trove], &mut banks_client).await;

    let borrow_amount = 100;
    let lamports = 100;
    let inx = Instruction {
        program_id,
        accounts: nexfin_program::accounts::Borrow {
            authority: authority.pubkey(),
            trove: trove.pubkey(),
            rent: sysvar::rent::ID,
        }
        .to_account_metas(None),
        data: nexfin_program::instruction::Borrow {
            borrow_amount,
            lamports,
        }
        .data(),
    };

    process_and_assert_ok(&[inx], &payer, &[&payer, &authority], &mut banks_client).await;
    let trove_account = banks_client
        .get_account(trove.pubkey())
        .await
        .unwrap()
        .unwrap();

    let trove_state = Trove::try_deserialize(&mut trove_account.data.as_ref()).unwrap();
    assert_eq!(trove_state.is_initialized, true);
    assert_eq!(trove_state.is_liquidated, false);
    assert_eq!(trove_state.is_received, false);
    assert_eq!(trove_state.borrow_amount, borrow_amount);
    assert_eq!(trove_state.lamports_amount, lamports);
}
