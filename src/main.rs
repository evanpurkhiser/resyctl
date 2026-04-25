mod api;
mod cli;
mod commands;
mod config;
mod error;
mod models;
mod output;
mod state;
mod types;
mod util;

use std::process::ExitCode;

use clap::Parser;
use serde_json::json;

use api::ResyClient;
use cli::{Cli, Command};
use config::{resolve_client_key, resolve_payment_method_id};
use error::Error;
use output::print_json;

#[tokio::main]
async fn main() -> ExitCode {
    let cli = Cli::parse();
    let result = run(cli).await;

    match result {
        Ok(output) => match print_json(&output) {
            Ok(()) => ExitCode::SUCCESS,
            Err(e) => {
                emit_error(&e);
                ExitCode::from(e.exit_code())
            }
        },
        Err(e) => {
            emit_error(&e);
            ExitCode::from(e.exit_code())
        }
    }
}

fn emit_error(e: &Error) {
    let payload = json!({
        "ok": false,
        "kind": e.kind(),
        "error": e.to_string(),
    });
    let _ = print_json(&payload);
}

async fn run(cli: Cli) -> Result<serde_json::Value, Error> {
    match cli.command {
        Command::Auth(args) => commands::auth::run(args).await,
        Command::Config(_) => commands::config_cmd::run().await,
        command => {
            let client_key = resolve_client_key();
            let client = ResyClient::from_state(&client_key)?;

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
                Command::Auth(_) | Command::Config(_) => Err(Error::Internal(
                    "unreachable command state".to_string(),
                )),
            }
        }
    }
}
