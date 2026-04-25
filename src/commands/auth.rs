use std::fs;
use std::path::Path;

use serde_json::{Value, json};

use crate::api::ResyClient;
use crate::cli::{AuthArgs, AuthCommand, LoginArgs};
use crate::config::{resolve_api_key, resolve_auth_token};
use crate::error::AppError;
use crate::util::to_json_value;

pub async fn run(args: AuthArgs) -> Result<Value, AppError> {
    match args.command {
        AuthCommand::Status => status().await,
        AuthCommand::Login(login_args) => login(login_args).await,
    }
}

async fn status() -> Result<Value, AppError> {
    let api_key = resolve_api_key();
    let auth_token = resolve_auth_token()?;
    let client = ResyClient::new(&api_key, &auth_token)?;
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
    let api_key = resolve_api_key();
    let password = resolve_password(&args)?;

    let client = ResyClient::unauthenticated(&api_key)?;
    let auth = client.auth_password(&args.email, &password).await?;

    if args.write_secrets {
        write_secrets(&auth)?;
    }

    Ok(json!({
        "ok": true,
        "login": {
            "email": args.email,
            "token_present": auth.token.is_some(),
            "payment_method_id": auth.payment_method_id,
            "wrote_secrets": args.write_secrets,
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

fn write_secrets(auth: &crate::models::AuthPasswordResponse) -> Result<(), AppError> {
    fs::create_dir_all("secrets")
        .map_err(|e| AppError::new(4, format!("failed creating secrets directory: {e}")))?;

    let token = auth
        .token
        .as_deref()
        .ok_or_else(|| AppError::new(4, "auth response missing token"))?;

    fs::write("secrets/resy_auth_token", format!("{token}\n"))
        .map_err(|e| AppError::new(4, format!("failed writing secrets/resy_auth_token: {e}")))?;

    if let Some(payment_method_id) = auth.payment_method_id {
        fs::write(
            "secrets/resy_payment_method_id",
            format!("{}\n", payment_method_id),
        )
        .map_err(|e| {
            AppError::new(
                4,
                format!("failed writing secrets/resy_payment_method_id: {e}"),
            )
        })?;
    }

    Ok(())
}
