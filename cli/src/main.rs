use anchor_client::{solana_client::rpc_client::RpcClient, Client, ClientError};
use anchor_lang::prelude::*;
use clap::Result;
use solana_sdk::{
    commitment_config, instruction::Instruction, pubkey::Pubkey, signature::read_keypair_file,
    signer::Signer, system_program, sysvar,
};
use std::{str::FromStr, thread, time::Duration};

use nexfin_program::{self};
fn main() {
    let matches = clap::App::new("Crank hgen program")
        .version("1.0")
        .author("batphonghan")
        .about("Crank services to update price feed of pyth network")
        .arg(
            clap::Arg::with_name("program_id")
                .long("program_id")
                .default_value("3cwgwP3wfgbmMDRm1LTHJ2mQZQmt6uRohuF8Sf2YEr8w"),
        )
        .arg(
            clap::Arg::with_name("cluster")
                .short("c")
                .long("cluster")
                .default_value("https://api.testnet.solana.com"),
        )
        .arg(
            clap::Arg::with_name("wallet")
                .short("w")
                .long("wallet")
                .default_value("~/.config/solana/id.json"),
        )
        .arg(
            clap::Arg::with_name("price")
                .long("price")
                .default_value("7VJsBtJzgTftYzEeooSDYyjKXvYRWJHdwvbwfBvTg9K"),
        )
        .get_matches();

    let wallet = matches.value_of("wallet").unwrap();
    let wallet = shellexpand::tilde(wallet).to_string();
    println!("Value for wallet: {}", wallet);

    let cluster_url = matches.value_of("cluster").unwrap();
    println!("Value for cluster: {}", &cluster_url);

    let program_id_str = matches.value_of("program_id").unwrap();
    println!("Value for program ID: {}", program_id_str);

    let program_id = Pubkey::from_str(program_id_str).unwrap();

    let price_str = matches.value_of("price").unwrap();
    println!("Value for price ID: {}", price_str);

    let price = Pubkey::from_str(price_str).unwrap();

    let payer = read_keypair_file(wallet.clone()).expect("Requires a keypair file");

    let cluster = anchor_client::Cluster::from_str(cluster_url).unwrap();

    let client = Client::new_with_options(
        cluster,
        payer,
        commitment_config::CommitmentConfig::processed(),
    );
    let program_client = client.program(program_id);

    let price_ac = RpcClient::new(cluster_url.to_string())
        .get_account(&price)
        .unwrap();
    println!("PRICE ACC {:?} ", price_ac);
    loop {
        let payer = read_keypair_file(wallet.clone()).expect("Requires a keypair file");
        thread::sleep(Duration::from_millis(1000));

        let (price_pda, bump) = Pubkey::find_program_address(&[b"price"], &program_id);
        let rs = program_client
            .request()
            .accounts(nexfin_program::accounts::LoadPrice { price: price })
            .args(nexfin_program::instruction::LoadPrice { bump })
            .send();
        match rs {
            Ok(s) => {
                println!("TX: {}", s);
                let acc = program_client.account::<nexfin_program::state::Price>(price);
                println!("{:?}", acc);
                println!("===================");
            }
            Err(e) => {
                println!("ERR {:?}, wait for 3 minutes before try again", e);
                thread::sleep(Duration::from_secs(60 * 3));
            }
        }
    }
}
