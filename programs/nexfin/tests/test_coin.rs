#![cfg(feature = "test-bpf")]
#![allow(unused)]
mod helper;
use anchor_lang::solana_program::{instruction::Instruction, pubkey::Pubkey, sysvar};
use anchor_lang::AccountDeserialize;
use anchor_lang::{InstructionData, ToAccountMetas};
use anchor_spl::token;
use helper::process_and_assert_ok;
use solana_program_test::{processor, tokio, ProgramTest};
use solana_sdk::signature::{Keypair, Signer};
use solana_sdk::system_instruction;

use nexfin_program::helpers::{get_depositors_fee, get_team_fee, get_trove_debt_amount};
use nexfin_program::{params, state::Trove};

#[tokio::test]
async fn test_withdraw() {
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

    let withdraw_inx = Instruction {
        program_id,
        accounts: nexfin_program::accounts::WithdrawCoin {
            authority: authority.pubkey(),
            trove: trove.pubkey(),
        }
        .to_account_metas(None),
        data: nexfin_program::instruction::WithdrawCoin {
            amount: borrow_amount,
        }
        .data(),
    };
    process_and_assert_ok(
        &[withdraw_inx],
        &payer,
        &[&payer, &authority],
        &mut banks_client,
    )
    .await;
}

#[tokio::test]
async fn test_redeem_coin() {
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

    let withdraw_inx = Instruction {
        program_id,
        accounts: nexfin_program::accounts::RedeemCoin {
            authority: authority.pubkey(),
            trove: trove.pubkey(),
        }
        .to_account_metas(None),
        data: nexfin_program::instruction::RedeemCoin {
            amount: borrow_amount,
        }
        .data(),
    };
    process_and_assert_ok(
        &[withdraw_inx],
        &payer,
        &[&payer, &authority],
        &mut banks_client,
    )
    .await;
}
