use serde_json::Value;

use crate::error::AppError;

pub fn print_json(value: &Value) -> Result<(), AppError> {
    let output = serde_json::to_string_pretty(value)
        .map_err(|e| AppError::new(4, format!("failed to serialize output JSON: {e}")))?;
    println!("{output}");
    Ok(())
}
