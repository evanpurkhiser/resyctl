use serde_json::Value;

use crate::error::Error;

pub fn print_json(value: &Value) -> Result<(), Error> {
    let output = serde_json::to_string_pretty(value)
        .map_err(|e| Error::Internal(format!("failed to serialize output JSON: {e}")))?;
    println!("{output}");
    Ok(())
}
