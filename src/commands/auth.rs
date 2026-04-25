use std::fs;
use std::path::Path;

use serde_json::{Value, json};

use crate::api::ResyClient;
use crate::cli::{AuthArgs, AuthCommand, LoginArgs};
use crate::config::{resolve_auth_token, resolve_client_key};
use crate::error::AppError;
use crate::state;
use crate::util::to_json_value;

pub async fn run(args: AuthArgs) -> Result<Value, AppError> {
    match args.command {
        AuthCommand::Status => status().await,
        AuthCommand::Login(login_args) => login(login_args).await,
    }
}

async fn status() -> Result<Value, AppError> {
    let client_key = resolve_client_key();
    let auth_token = resolve_auth_token()?;
    let client = ResyClient::new(&client_key, &auth_token)?;
    let user = client.user().await?;
    let name = [
        user.first_name.as_deref().unwrap_or_default(),
        user.last_name.as_deref().unwrap_or_default(),
    ]
    .join(" ")
    .trim()
    .to_string();
    let raw = to_json_value(&user)?;

    Ok(json!({
        "ok": true,
        "authenticated": true,
        "user": {
            "id": user.id,
            "name": name,
            "email": user.email,
            "payment_method_id": user.payment_method_id,
            "num_bookings": user.num_bookings,
        },
        "raw": raw,
    }))
}

async fn login(args: LoginArgs) -> Result<Value, AppError> {
    let client_key = resolve_client_key();
    let password = resolve_password(&args)?;

    let client = ResyClient::unauthenticated(&client_key)?;
    let auth = client.auth_password(&args.email, &password).await?;
    write_state(&auth)?;

    Ok(json!({
        "ok": true,
        "login": {
            "email": args.email,
            "token_present": auth.token.is_some(),
            "payment_method_id": auth.payment_method_id,
            "wrote_state": true,
        },
        "raw": to_json_value(&auth)?,
    }))
}

fn resolve_password(args: &LoginArgs) -> Result<String, AppError> {
    if let Some(password) = &args.password
        && !password.is_empty()
    {
        return Ok(password.clone());
    }

    if let Some(path) = &args.password_file {
        let p = Path::new(path);
        let value = fs::read_to_string(p)
            .map_err(|e| AppError::new(5, format!("failed reading password file {path}: {e}")))?;
        let trimmed = value.trim().to_string();
        if trimmed.is_empty() {
            return Err(AppError::new(5, format!("password file {path} is empty")));
        }
        return Ok(trimmed);
    }

    Err(AppError::new(
        5,
        "auth login requires --password or --password-file",
    ))
}

fn write_state(auth: &crate::models::AuthPasswordResponse) -> Result<(), AppError> {
    let token = auth
        .token
        .as_deref()
        .ok_or_else(|| AppError::new(4, "auth response missing token"))?;

    let mut current = state::load().unwrap_or_default();
    current.auth_token = Some(token.to_string());
    if let Some(payment_method_id) = auth.payment_method_id {
        current.payment_method_id = Some(payment_method_id);
    }
    state::save(&current)
}
