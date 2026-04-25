use serde_json::{Value, json};

use crate::api::ResyClient;
use crate::cli::QuoteArgs;
use crate::error::AppError;
use crate::util::{QuoteSummary, decode_slot_id, to_json_value};

pub async fn run(client: &ResyClient, args: QuoteArgs) -> Result<Value, AppError> {
    let slot = decode_slot_id(&args.slot_id)?;
    let details = client.details_with_commit(&slot.config_id, 0).await?;
    let summary = QuoteSummary::try_from(&details)?;
    let raw = to_json_value(&details)?;

    Ok(json!({
        "ok": true,
        "slot_id": args.slot_id,
        "slot": slot,
        "quote": summary,
        "raw": raw,
    }))
}
