#![cfg(feature = "test-bpf")]

use anchor_lang::prelude::Pubkey;
use anchor_lang::solana_program::{instruction::Instruction, program_pack::Pack};
use anchor_lang::{InstructionData, ToAccountMetas};
use assert_matches::assert_matches;
use nexfin_program::helpers::{get_depositors_fee, get_team_fee, get_trove_debt_amount};
use nexfin_program::state::Trove;
use solana_program_test::BanksClient;
use solana_program_test::{processor, tokio, ProgramTest};
use solana_sdk::signature::{Keypair, Signer};
use solana_sdk::transaction::Transaction;
use solana_sdk::transport::Result;
use solana_sdk::{commitment_config::CommitmentLevel, system_instruction, system_program};
use spl_associated_token_account;
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
    token_mint: [u8; 64],
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

    pub fn token_mint(&self) -> Keypair {
        Keypair::from_bytes(self.token_mint.as_ref()).unwrap()
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

    let token_mint = Keypair::new();

    init_user_token(&mut banks_client, &authority, &token_mint, &payer).await;

    InitResult {
        program_id,
        authority: authority.to_bytes(),
        banks_client,
        trove: trove.to_bytes(),
        payer: payer.to_bytes(),
        token_mint: token_mint.to_bytes(),
    }
}

pub async fn initialize_mint(
    banks_client: &mut BanksClient,
    payer: &Keypair,
    token_mint: &Keypair,
    authority: &Pubkey,
    decimals: u8,
) {
    let rent = banks_client.get_rent().await.unwrap();
    let token_mint_account_rent = rent.minimum_balance(spl_token::state::Mint::LEN);
    let recent_blockhash = banks_client.get_recent_blockhash().await.unwrap();
    let transaction = Transaction::new_signed_with_payer(
        &[
            system_instruction::create_account(
                &payer.pubkey(),
                &token_mint.pubkey(),
                token_mint_account_rent,
                spl_token::state::Mint::LEN as u64,
                &spl_token::id(),
            ),
            spl_token::instruction::initialize_mint(
                &spl_token::id(),
                &token_mint.pubkey(),
                authority,
                None,
                decimals,
            )
            .unwrap(),
        ],
        Some(&payer.pubkey()),
        &[payer, token_mint],
        recent_blockhash,
    );

    assert_matches!(banks_client.process_transaction(transaction).await, Ok(()));
}

pub async fn mint_to(
    owner: &Keypair,
    token_mint: &Pubkey,
    account_pubkey: &Pubkey,
    amount: u64,
    banks_client: &mut BanksClient,
) {
    let recent_blockhash = banks_client.get_recent_blockhash().await.unwrap();
    let transaction = Transaction::new_signed_with_payer(
        &[spl_token::instruction::mint_to(
            &spl_token::id(),
            token_mint,
            account_pubkey,
            &owner.pubkey(),
            &[],
            amount,
        )
        .unwrap()],
        Some(&owner.pubkey()),
        &[owner],
        recent_blockhash,
    );

    assert_matches!(banks_client.process_transaction(transaction).await, Ok(()));
}

async fn init_user_token(
    banks_client: &mut BanksClient,
    user_keypair: &Keypair,
    token_keypair: &Keypair,
    payer_keypair: &Keypair,
) -> Pubkey {
    initialize_mint(
        banks_client,
        &payer_keypair,
        &token_keypair,
        &payer_keypair.pubkey(),
        6,
    )
    .await;

    process_ins(
        banks_client,
        &[
            spl_associated_token_account::create_associated_token_account(
                &payer_keypair.pubkey(),
                &user_keypair.pubkey(),
                &token_keypair.pubkey(),
            ),
        ],
        &payer_keypair,
        &[],
    )
    .await
    .ok()
    .unwrap_or_else(|| panic!("Can not create ATA account"));

    let user_ata = spl_associated_token_account::get_associated_token_address(
        &user_keypair.pubkey(),
        &token_keypair.pubkey(),
    );

    mint_to(
        payer_keypair,
        &token_keypair.pubkey(),
        &user_ata,
        1_000_000 * 1_000_000_000,
        banks_client,
    )
    .await;

    user_ata
}
pub async fn process_ins(
    banks_client: &mut BanksClient,
    instructions: &[Instruction],
    payer: &Keypair,
    signers: &[&Keypair],
) -> Result<()> {
    let recent_blockhash = banks_client.get_recent_blockhash().await.unwrap();

    let mut all_signers = vec![payer];
    all_signers.extend_from_slice(signers);

    let mut tx = Transaction::new_with_payer(instructions, Some(&payer.pubkey()));
    if let Err(e) = tx.try_sign(&all_signers, recent_blockhash) {
        panic!(">>> Transaction::sign failed with error {:?}", e)
    }

    banks_client
        .process_transaction_with_commitment(tx, CommitmentLevel::Finalized)
        .await
}
