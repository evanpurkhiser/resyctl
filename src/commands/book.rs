use chrono::Utc;
use serde_json::{Value, json};

use crate::api::ResyClient;
use crate::cli::BookArgs;
use crate::error::AppError;
use crate::util::{decode_slot_id, parse_rfc3339_utc, quote_summary};

pub async fn run(
    client: &ResyClient,
    args: BookArgs,
    cli_payment_method_id: Option<i64>,
) -> Result<Value, AppError> {
    let slot = decode_slot_id(&args.slot_id)?;
    let details = client.details(&slot).await?;
    let summary = quote_summary(&details);

    let fee_amount = summary
        .get("fee_amount")
        .and_then(Value::as_f64)
        .unwrap_or(0.0);

    if fee_amount > 0.0 && !args.allow_fee {
        return Err(AppError::new(
            3,
            "booking blocked by policy: cancellation fee present; pass --allow-fee to override",
        ));
    }

    if let Some(max_fee) = args.max_fee
        && fee_amount > max_fee
    {
        return Err(AppError::new(
            3,
            format!(
                "booking blocked by policy: fee {fee_amount} exceeds --max-fee {max_fee}"
            ),
        ));
    }

    if let Some(max_cutoff_hours) = args.max_cutoff_hours {
        let fee_cutoff = summary.get("fee_cutoff").and_then(Value::as_str);
        let now = Utc::now();
        let hours_until_cutoff = fee_cutoff
            .and_then(parse_rfc3339_utc)
            .map(|ts| (ts - now).num_hours());

        match hours_until_cutoff {
            Some(hours) if hours < max_cutoff_hours => {
                return Err(AppError::new(
                    3,
                    format!(
                        "booking blocked by policy: cutoff {hours}h is less than --max-cutoff-hours {max_cutoff_hours}"
                    ),
                ));
            }
            None => {
                return Err(AppError::new(
                    3,
                    "booking blocked by policy: cutoff unavailable for --max-cutoff-hours check",
                ));
            }
            _ => {}
        }
    }

    let book_token = details
        .pointer("/book_token/value")
        .and_then(Value::as_str)
        .ok_or_else(|| AppError::new(4, "details response missing book_token.value"))?;

    let payment_method_id = cli_payment_method_id.or_else(|| {
        details
            .pointer("/user/payment_methods")
            .and_then(Value::as_array)
            .and_then(|arr| arr.first())
            .and_then(|v| v.get("id"))
            .and_then(Value::as_i64)
    });

    if args.dry_run {
        return Ok(json!({
            "ok": true,
            "dry_run": true,
            "would_book": true,
            "slot": slot,
            "quote": summary,
            "payment_method_id": payment_method_id,
        }));
    }

    if !args.yes {
        return Err(AppError::new(
            5,
            "booking requires --yes (or use --dry-run)",
        ));
    }

    let booking_result = client.book(book_token, payment_method_id).await?;

    Ok(json!({
        "ok": true,
        "booked": true,
        "slot": slot,
        "quote": summary,
        "payment_method_id": payment_method_id,
        "result": booking_result,
    }))
}
