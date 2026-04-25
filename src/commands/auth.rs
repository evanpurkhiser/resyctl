use std::fs;
use std::path::Path;

use serde_json::{Value, json};

use crate::api::ResyClient;
use crate::cli::{AuthArgs, AuthCommand, LoginArgs};
use crate::config::{resolve_api_key, resolve_auth_token};
use crate::error::AppError;

pub async fn run(args: AuthArgs, api_key_flag: Option<String>, auth_token_flag: Option<String>) -> Result<Value, AppError> {
    match args.command {
        AuthCommand::Status => status(api_key_flag, auth_token_flag).await,
        AuthCommand::Login(login_args) => login(login_args, api_key_flag).await,
    }
}

async fn status(api_key_flag: Option<String>, auth_token_flag: Option<String>) -> Result<Value, AppError> {
    let api_key = resolve_api_key(api_key_flag);
    let auth_token = resolve_auth_token(auth_token_flag)?;
    let client = ResyClient::new(&api_key, &auth_token)?;
    let user = client.user().await?;
    let name = [
        user.get("first_name").and_then(Value::as_str).unwrap_or_default(),
        user.get("last_name").and_then(Value::as_str).unwrap_or_default(),
    ]
    .join(" ")
    .trim()
    .to_string();

    Ok(json!({
        "ok": true,
        "authenticated": true,
        "user": {
            "id": user.get("id").and_then(Value::as_i64),
            "name": name,
            "email": user.get("em_address").and_then(Value::as_str),
            "payment_method_id": user.get("payment_method_id"),
            "num_bookings": user.get("num_bookings"),
        },
        "raw": user,
    }))
}

async fn login(args: LoginArgs, api_key_flag: Option<String>) -> Result<Value, AppError> {
    let api_key = resolve_api_key(api_key_flag);
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
            "token_present": auth.get("token").and_then(Value::as_str).is_some(),
            "payment_method_id": auth.get("payment_method_id"),
            "wrote_secrets": args.write_secrets,
        },
        "raw": auth,
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

fn write_secrets(auth: &Value) -> Result<(), AppError> {
    fs::create_dir_all("secrets")
        .map_err(|e| AppError::new(4, format!("failed creating secrets directory: {e}")))?;

    let token = auth
        .get("token")
        .and_then(Value::as_str)
        .ok_or_else(|| AppError::new(4, "auth response missing token"))?;

    fs::write("secrets/resy_auth_token", format!("{token}\n"))
        .map_err(|e| AppError::new(4, format!("failed writing secrets/resy_auth_token: {e}")))?;

    if let Some(payment_method_id) = auth.get("payment_method_id") {
        fs::write("secrets/resy_payment_method_id", format!("{}\n", payment_method_id))
            .map_err(|e| AppError::new(4, format!("failed writing secrets/resy_payment_method_id: {e}")))?;
    }

    Ok(())
}
