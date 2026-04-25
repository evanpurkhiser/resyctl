use serde_json::{Value, json};

use crate::api::ResyClient;
use crate::cli::QuoteArgs;
use crate::error::AppError;
use crate::util::{decode_slot_id, quote_summary};

pub async fn run(client: &ResyClient, args: QuoteArgs) -> Result<Value, AppError> {
    let slot = decode_slot_id(&args.slot_id)?;
    let details = client.details_with_commit(&slot.config_id, 0).await?;
    let summary = quote_summary(&details);
    let raw = serde_json::to_value(&details).unwrap_or_else(|_| Value::Null);

    Ok(json!({
        "ok": true,
        "slot_id": args.slot_id,
        "slot": slot,
        "quote": summary,
        "raw": raw,
    }))
}
