use serde_json::Value;

use crate::config::config_snapshot;
use crate::error::AppError;

pub async fn run(payment_method_id_flag: Option<i64>) -> Result<Value, AppError> {
    Ok(config_snapshot(payment_method_id_flag))
}
