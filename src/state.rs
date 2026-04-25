use std::fs;
use std::io::Write;
use std::os::unix::fs::OpenOptionsExt;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::error::AppError;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct State {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auth_token: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payment_method_id: Option<i64>,
}

pub fn state_path() -> Result<PathBuf, AppError> {
    let base = dirs::state_dir()
        .or_else(|| dirs::home_dir().map(|h| h.join(".local/state")))
        .ok_or_else(|| AppError::new(5, "could not resolve state directory"))?;
    Ok(base.join("resyctl/state.json"))
}

pub fn load() -> Result<State, AppError> {
    let path = state_path()?;
    if !path.exists() {
        return Ok(State::default());
    }
    let contents = fs::read_to_string(&path)
        .map_err(|e| AppError::new(4, format!("failed reading {}: {e}", path.display())))?;
    if contents.trim().is_empty() {
        return Ok(State::default());
    }
    serde_json::from_str(&contents)
        .map_err(|e| AppError::new(4, format!("failed parsing {}: {e}", path.display())))
}

pub fn save(state: &State) -> Result<(), AppError> {
    let path = state_path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| AppError::new(4, format!("failed creating {}: {e}", parent.display())))?;
    }

    let contents = serde_json::to_string_pretty(state)
        .map_err(|e| AppError::new(4, format!("failed serializing state: {e}")))?;

    let mut file = fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(&path)
        .map_err(|e| AppError::new(4, format!("failed opening {}: {e}", path.display())))?;
    file.write_all(contents.as_bytes())
        .and_then(|_| file.write_all(b"\n"))
        .map_err(|e| AppError::new(4, format!("failed writing {}: {e}", path.display())))?;

    Ok(())
}
