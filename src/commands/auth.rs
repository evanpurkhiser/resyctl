use std::io::{self, BufRead, Write};

use serde_json::{Value, json};

use crate::api::ResyClient;
use crate::cli::{AuthArgs, AuthCommand, LoginArgs};
use crate::config::resolve_client_key;
use crate::error::{ApiError, Error, InputError, IoError};
use crate::state;
use crate::util::to_json_value;

pub async fn run(args: AuthArgs) -> Result<Value, Error> {
    match args.command {
        AuthCommand::Status => status().await,
        AuthCommand::Login(login_args) => login(login_args).await,
    }
}

async fn status() -> Result<Value, Error> {
    let client_key = resolve_client_key();
    let client = ResyClient::from_state(&client_key)?;
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

async fn login(args: LoginArgs) -> Result<Value, Error> {
    let client_key = resolve_client_key();
    let email = resolve_email(&args)?;
    let password = resolve_password(&args)?;

    let client = ResyClient::unauthenticated(&client_key)?;
    let auth = client.auth_password(&email, &password).await?;
    write_state(&email, &password, &auth)?;

    Ok(json!({
        "ok": true,
        "login": {
            "email": email,
            "token_present": auth.token.is_some(),
            "payment_method_id": auth.payment_method_id,
            "wrote_state": true,
        },
        "raw": to_json_value(&auth)?,
    }))
}

fn resolve_email(args: &LoginArgs) -> Result<String, Error> {
    if let Some(email) = &args.email
        && !email.trim().is_empty()
    {
        return Ok(email.trim().to_string());
    }
    prompt_line("Email: ")
}

fn resolve_password(args: &LoginArgs) -> Result<String, Error> {
    if let Some(password) = &args.password
        && !password.is_empty()
    {
        return Ok(password.clone());
    }

    let password = rpassword::prompt_password("Password: ").map_err(IoError::PasswordPrompt)?;
    if password.is_empty() {
        return Err(InputError::EmptyPassword.into());
    }
    Ok(password)
}

fn prompt_line(prompt: &str) -> Result<String, Error> {
    let mut stdout = io::stdout();
    stdout
        .write_all(prompt.as_bytes())
        .and_then(|_| stdout.flush())
        .map_err(IoError::PromptWrite)?;

    let mut buf = String::new();
    io::stdin()
        .lock()
        .read_line(&mut buf)
        .map_err(IoError::PromptRead)?;

    let trimmed = buf.trim().to_string();
    if trimmed.is_empty() {
        return Err(InputError::EmptyPromptInput.into());
    }
    Ok(trimmed)
}

fn write_state(
    email: &str,
    password: &str,
    auth: &crate::models::AuthPasswordResponse,
) -> Result<(), Error> {
    let token = auth
        .token
        .as_deref()
        .ok_or(ApiError::AuthResponseMissingToken)?;

    let mut current = state::load().unwrap_or_default();
    current.email = Some(email.to_string());
    current.password = Some(password.to_string());
    current.auth_token = Some(token.to_string());
    if let Some(payment_method_id) = auth.payment_method_id {
        current.payment_method_id = Some(payment_method_id);
    }
    state::save(&current)
}
