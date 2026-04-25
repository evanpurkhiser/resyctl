use serde_json::{Value, json};

use crate::api::ResyClient;
use crate::cli::CancelArgs;
use crate::error::AppError;
use crate::util::to_json_value;

pub async fn run(client: &ResyClient, args: CancelArgs) -> Result<Value, AppError> {
    let mut effective_token = args.resy_token.clone();
    let lookup = client.reservation_by_token(&args.resy_token).await?;
    let reservation_snapshot = to_json_value(&lookup)?;
    if let Some(next_token) = lookup
        .reservations
        .first()
        .and_then(|r| r.resy_token.as_deref())
        .filter(|s| !s.is_empty())
    {
        effective_token = next_token.to_string();
    }

    if args.dry_run {
        return Ok(json!({
            "ok": true,
            "dry_run": true,
            "would_cancel": true,
            "input_resy_token_present": !args.resy_token.is_empty(),
            "effective_resy_token_present": !effective_token.is_empty(),
            "refreshed": true,
            "reservation_snapshot": reservation_snapshot,
        }));
    }

    if !args.yes {
        return Err(AppError::new(5, "cancel requires --yes (or use --dry-run)"));
    }

    let result = client.cancel(&effective_token).await?;
    let result_raw = to_json_value(&result)?;

    Ok(json!({
        "ok": true,
        "canceled": true,
        "refreshed": true,
        "input_resy_token_present": !args.resy_token.is_empty(),
        "effective_resy_token_present": !effective_token.is_empty(),
        "result": result_raw,
        "reservation_snapshot": reservation_snapshot,
    }))
}
