#![cfg(feature = "test-bpf")]

mod helper;
use anchor_lang::solana_program::{instruction::Instruction, pubkey::Pubkey, sysvar};
use anchor_lang::AccountDeserialize;
use anchor_lang::{InstructionData, ToAccountMetas};
use helper::process_and_assert_ok;
use solana_program_test::{processor, tokio, ProgramTest};
use solana_sdk::signature::{Keypair, Signer};
use solana_sdk::system_instruction;

use nexfin_program::helpers::{get_depositors_fee, get_team_fee, get_trove_debt_amount};
use nexfin_program::state::Trove;

#[tokio::test]
async fn test_trove_borrow() {
    let init = helper::setup().await;

    let program_id = init.program_id;
    let authority = init.authority();
    let trove = init.trove();
    let payer = init.payer();
    let mut banks_client = init.banks_client;

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
    assert_eq!(trove_state.depositor_fee, get_depositors_fee(borrow_amount));
    assert_eq!(trove_state.team_fee, get_team_fee(borrow_amount));
    assert_eq!(
        trove_state.amount_to_close,
        get_trove_debt_amount(borrow_amount)
    );
    assert_eq!(trove_state.owner, authority.pubkey());
}
