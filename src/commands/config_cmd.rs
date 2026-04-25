use serde_json::Value;

use crate::config::config_snapshot;
use crate::error::AppError;

pub async fn run(
    auth_token_flag: Option<String>,
    api_key_flag: Option<String>,
    payment_method_id_flag: Option<i64>,
) -> Result<Value, AppError> {
    Ok(config_snapshot(
        auth_token_flag.as_deref(),
        api_key_flag.as_deref(),
        payment_method_id_flag,
    ))
}
