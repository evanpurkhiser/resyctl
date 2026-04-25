use std::env;

use serde_json::{Value, json};

use crate::state;

pub const DEFAULT_CLIENT_KEY: &str = "AIcdK2rLXG6TYwJseSbmrBAy3RP81ocd";

pub fn resolve_client_key() -> String {
    DEFAULT_CLIENT_KEY.to_string()
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

    state::load().ok().and_then(|s| s.payment_method_id)
}

pub fn config_snapshot(cli_payment_id: Option<i64>) -> Value {
    let effective_client_key = resolve_client_key();
    let effective_payment = resolve_payment_method_id(cli_payment_id);
    let loaded_state = state::load().ok();
    let state_path = state::state_path().ok();
    let auth_token_resolved = loaded_state
        .as_ref()
        .and_then(|s| s.auth_token.as_deref())
        .filter(|t| !t.trim().is_empty());

    json!({
        "ok": true,
        "effective": {
            "client_key_suffix": suffix(&effective_client_key, 6),
            "auth_token_present": auth_token_resolved.is_some(),
            "auth_token_length": auth_token_resolved.map(|s| s.len()),
            "payment_method_id": effective_payment,
        },
        "sources": {
            "state_file": state_path.as_ref().map(|p| p.display().to_string()),
            "client_key": {
                "default_used": true,
            },
            "payment_method_id": {
                "cli_flag": cli_payment_id.is_some(),
                "env": env_has_any(&["RESSY_PAYMENT_METHOD_ID", "RESY_PAYMENT_METHOD_ID"]),
                "state": loaded_state
                    .as_ref()
                    .map(|s| s.payment_method_id.is_some())
                    .unwrap_or(false),
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
