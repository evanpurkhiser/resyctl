mod api;
mod cli;
mod commands;
mod config;
mod error;
mod models;
mod output;
mod types;
mod util;

use std::process::ExitCode;

use clap::Parser;
use serde_json::json;

use api::ResyClient;
use cli::{Cli, Command};
use config::{resolve_api_key, resolve_auth_token, resolve_payment_method_id};
use error::AppError;
use output::print_json;

#[tokio::main]
async fn main() -> ExitCode {
    let cli = Cli::parse();
    let result = run(cli).await;

    match result {
        Ok(output) => {
            if let Err(e) = print_json(&output) {
                let err = json!({
                    "ok": false,
                    "code": 4,
                    "error": format!("failed to print JSON: {}", e.message),
                });
                let _ = print_json(&err);
                ExitCode::from(4)
            } else {
                ExitCode::SUCCESS
            }
        }
        Err(e) => {
            let err = json!({"ok": false, "code": e.code, "error": e.message});
            let _ = print_json(&err);
            ExitCode::from(e.code as u8)
        }
    }
}

async fn run(cli: Cli) -> Result<serde_json::Value, AppError> {
    match cli.command {
        Command::Auth(args) => commands::auth::run(args).await,
        Command::Config(_) => commands::config_cmd::run().await,
        command => {
            let auth_token = resolve_auth_token()?;
            let api_key = resolve_api_key();
            let client = ResyClient::new(&api_key, &auth_token)?;

            match command {
                Command::Search(args) => commands::search::run(&client, args).await,
                Command::Availability(args) => commands::availability::run(&client, args).await,
                Command::Quote(args) => commands::quote::run(&client, args).await,
                Command::Book(args) => {
                    let payment_method_id = resolve_payment_method_id(args.payment_method_id);
                    commands::book::run(&client, args, payment_method_id).await
                }
                Command::Reservations(args) => commands::reservations::run(&client, args).await,
                Command::PaymentMethods(args) => {
                    commands::payment_methods::run(&client, args).await
                }
                Command::Cancel(args) => commands::cancel::run(&client, args).await,
                Command::Auth(_) | Command::Config(_) => {
                    Err(AppError::new(5, "unreachable command state"))
                }
            }
        }
    }
}
