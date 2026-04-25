use serde_json::{Value, json};

use crate::api::ResyClient;
use crate::cli::CancelArgs;
use crate::error::AppError;

pub async fn run(client: &ResyClient, args: CancelArgs) -> Result<Value, AppError> {
    let mut effective_token = args.resy_token.clone();
    let mut reservation_snapshot = Value::Null;

    if args.refresh_token {
        let lookup = client.reservation_by_token(&args.resy_token).await?;
        reservation_snapshot = serde_json::to_value(&lookup).unwrap_or_else(|_| Value::Null);
        if let Some(next_token) = lookup
            .reservations
            .first()
            .and_then(|r| r.resy_token.as_deref())
            .filter(|s| !s.is_empty())
        {
            effective_token = next_token.to_string();
        }
    }

    if args.dry_run {
        return Ok(json!({
            "ok": true,
            "dry_run": true,
            "would_cancel": true,
            "input_resy_token_present": !args.resy_token.is_empty(),
            "effective_resy_token_present": !effective_token.is_empty(),
            "refreshed": args.refresh_token,
            "reservation_snapshot": reservation_snapshot,
        }));
    }

    if !args.yes {
        return Err(AppError::new(
            5,
            "cancel requires --yes (or use --dry-run)",
        ));
    }

    let result = client.cancel(&effective_token).await?;
    let result_raw = serde_json::to_value(&result).unwrap_or_else(|_| Value::Null);

    Ok(json!({
        "ok": true,
        "canceled": true,
        "refreshed": args.refresh_token,
        "input_resy_token_present": !args.resy_token.is_empty(),
        "effective_resy_token_present": !effective_token.is_empty(),
        "result": result_raw,
        "reservation_snapshot": reservation_snapshot,
    }))
}
