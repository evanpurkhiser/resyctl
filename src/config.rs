use std::env;
use std::fs;
use std::path::Path;

use serde_json::{Value, json};

use crate::error::AppError;

pub const DEFAULT_API_KEY: &str = "AIcdK2rLXG6TYwJseSbmrBAy3RP81ocd";

pub fn resolve_auth_token() -> Result<String, AppError> {
    let default_path = Path::new("secrets/resy_auth_token");
    if default_path.exists() {
        let token = fs::read_to_string(default_path).map_err(|e| {
            AppError::new(4, format!("failed reading secrets/resy_auth_token: {e}"))
        })?;
        if !token.trim().is_empty() {
            return Ok(token.trim().to_string());
        }
    }

    Err(AppError::new(
        5,
        "missing auth token; run `resyctl auth login` first",
    ))
}

pub fn resolve_api_key() -> String {
    DEFAULT_API_KEY.to_string()
}

pub fn resolve_payment_method_id(flag: Option<i64>) -> Option<i64> {
    if flag.is_some() {
        return flag;
    }
    for key in ["RESSY_PAYMENT_METHOD_ID", "RESY_PAYMENT_METHOD_ID"] {
        if let Ok(v) = env::var(key)
            && let Ok(parsed) = v.trim().parse::<i64>()
        {
            return Some(parsed);
        }
    }

    let default_path = Path::new("secrets/resy_payment_method_id");
    if default_path.exists()
        && let Ok(v) = fs::read_to_string(default_path)
        && let Ok(parsed) = v.trim().parse::<i64>()
    {
        return Some(parsed);
    }
    None
}

pub fn config_snapshot(cli_payment_id: Option<i64>) -> Value {
    let effective_api_key = resolve_api_key();
    let effective_payment = resolve_payment_method_id(cli_payment_id);
    let auth_resolved = resolve_auth_token().ok();

    json!({
        "ok": true,
        "effective": {
            "api_key_suffix": suffix(&effective_api_key, 6),
            "auth_token_present": auth_resolved.is_some(),
            "auth_token_length": auth_resolved.as_ref().map(|s| s.len()),
            "payment_method_id": effective_payment,
        },
        "sources": {
            "auth_token": {
                "file": Path::new("secrets/resy_auth_token").exists(),
            },
            "api_key": {
                "default_used": true,
            },
            "payment_method_id": {
                "cli_flag": cli_payment_id.is_some(),
                "env": env_has_any(&["RESSY_PAYMENT_METHOD_ID", "RESY_PAYMENT_METHOD_ID"]),
                "file": Path::new("secrets/resy_payment_method_id").exists(),
            }
        }
    })
}

fn env_has_any(keys: &[&str]) -> bool {
    keys.iter().any(|key| env::var(key).is_ok())
}

fn suffix(value: &str, keep: usize) -> String {
    if value.len() <= keep {
        return value.to_string();
    }
    value[value.len() - keep..].to_string()
}
