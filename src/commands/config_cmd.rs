use serde_json::Value;

use crate::config::config_snapshot;
use crate::error::AppError;

pub async fn run() -> Result<Value, AppError> {
    Ok(config_snapshot(None))
}
