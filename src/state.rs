use std::fs;
use std::io::Write;
#[cfg(unix)]
use std::os::unix::fs::OpenOptionsExt;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::error::{Error, IoError};

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

pub fn state_path() -> Result<PathBuf, Error> {
    let base = dirs::state_dir()
        .or_else(|| dirs::home_dir().map(|h| h.join(".local/state")))
        .ok_or(IoError::StateDirUnresolved)?;
    Ok(base.join("resyctl/state.json"))
}

pub fn load() -> Result<State, Error> {
    let path = state_path()?;
    if !path.exists() {
        return Ok(State::default());
    }
    let contents = fs::read_to_string(&path).map_err(|source| IoError::ReadFile {
        path: path.display().to_string(),
        source,
    })?;
    if contents.trim().is_empty() {
        return Ok(State::default());
    }
    serde_json::from_str(&contents)
        .map_err(|source| {
            IoError::ParseStateFile {
                path: path.display().to_string(),
                source,
            }
            .into()
        })
}

pub fn save(state: &State) -> Result<(), Error> {
    let path = state_path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|source| IoError::CreateDir {
            path: parent.display().to_string(),
            source,
        })?;
    }

    let contents = serde_json::to_string_pretty(state).map_err(IoError::SerializeState)?;

    let mut options = fs::OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    options.mode(0o600);

    let mut file = options.open(&path).map_err(|source| IoError::OpenFile {
        path: path.display().to_string(),
        source,
    })?;
    file.write_all(contents.as_bytes())
        .and_then(|_| file.write_all(b"\n"))
        .map_err(|source| IoError::WriteFile {
            path: path.display().to_string(),
            source,
        })?;

    Ok(())
}
