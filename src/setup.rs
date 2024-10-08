use std::{path::PathBuf, time::Duration};

use clap::Parser;
use jito_restaking_core::{ncn::Ncn, operator::Operator};
use jito_vault_core::vault::Vault;
use solana_sdk::{
    pubkey::Pubkey,
    signature::{read_keypair_file, Keypair},
    signer::Signer,
};

use crate::{restaking::RestakingHandler, vault::VaultHandler};

#[derive(Parser)]
#[command(about = "Setup Restaking Registration and Vault")]
pub struct Setup {
    /// RPC URL for the cluster
    #[arg(short, long, env, default_value = "https://api.devnet.solana.com")]
    rpc_url: String,

    /// Path to keypair used to pay
    #[arg(long, env, default_value = "~/.config/solana/id.json")]
    keypair: PathBuf,

    /// Vault program ID (Pubkey as base58 string)
    #[arg(
        long,
        env,
        default_value = "JaBCe1AWxqjdWMpNVj1NqjLgER1NDaAWUSa4v9ofPNG"
    )]
    vault_program_id: Pubkey,

    /// Restaking program ID (Pubkey as base58 string)
    #[arg(
        long,
        env,
        default_value = "HUZSpAbT5pHfpgm4Q5TnC5hkbkbJha6jYKDMN5SahRjv"
    )]
    restaking_program_id: Pubkey,

    /// Supported token pubkey
    #[arg(short, long)]
    token_mint_pubkey: Pubkey,
}

pub async fn command_setup(args: Setup) {
    let base = Keypair::new();
    let payer = read_keypair_file(args.keypair).expect("Failed to read keypair file");
    let restaking_handler = RestakingHandler::new(&args.rpc_url, &payer, args.restaking_program_id);
    let vault_handler = VaultHandler::new(
        &args.rpc_url,
        &payer,
        args.vault_program_id,
        args.restaking_program_id,
    );
    let ncn = Ncn::find_program_address(&args.restaking_program_id, &base.pubkey()).0;
    let vault = Vault::find_program_address(&args.vault_program_id, &base.pubkey()).0;

    vault_handler
        .initialize(&base, args.token_mint_pubkey)
        .await;

    restaking_handler.initialize_ncn(&base).await;

    tokio::time::sleep(Duration::from_secs(10)).await;

    // vault <> ncn
    restaking_handler
        .initialize_ncn_vault_ticket(ncn, vault)
        .await;
    restaking_handler.warmup_ncn_vault_ticket(ncn, vault).await;
    vault_handler.initialize_vault_ncn_ticket(vault, ncn).await;
    vault_handler.warmup_vault_ncn_ticket(vault, ncn).await;

    let operator = Operator::find_program_address(&args.restaking_program_id, &base.pubkey()).0;
    restaking_handler.initialize_operator(&base).await;

    tokio::time::sleep(Duration::from_secs(10)).await;

    // ncn <> operator
    restaking_handler
        .initialize_ncn_operator_state(ncn, operator)
        .await;
    restaking_handler.ncn_warmup_operator(ncn, operator).await;
    restaking_handler.operator_warmup_ncn(ncn, operator).await;

    // vault <> operator
    restaking_handler
        .initialize_operator_vault_ticket(operator, vault)
        .await;
    restaking_handler
        .warmup_operator_vault_ticket(operator, vault)
        .await;
    vault_handler
        .initialize_vault_operator_delegation(vault, operator)
        .await;
}
