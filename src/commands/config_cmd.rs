use serde_json::Value;

use crate::config::config_snapshot;
use crate::error::Error;

pub async fn run() -> Result<Value, Error> {
    Ok(config_snapshot(None))
}
