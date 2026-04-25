use chrono::Utc;
use serde_json::{Value, json};

use crate::api::ResyClient;
use crate::cli::BookArgs;
use crate::error::AppError;
use crate::util::{decode_slot_id, quote_summary, to_json_value};

pub async fn run(
    client: &ResyClient,
    args: BookArgs,
    cli_payment_method_id: Option<i64>,
) -> Result<Value, AppError> {
    let slot = decode_slot_id(&args.slot_id)?;
    let quote_details = client.details_with_commit(&slot.config_id, 0).await?;
    let summary = quote_summary(&quote_details)?;

    let fee_amount = summary.fee_amount();

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
            format!("booking blocked by policy: fee {fee_amount} exceeds --max-fee {max_fee}"),
        ));
    }

    if let Some(max_cutoff_hours) = args.max_cutoff_hours {
        let now = Utc::now();
        let hours_until_cutoff = summary.fee_cutoff_at().map(|ts| (ts - now).num_hours());

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

    let commit_details = client.details_with_commit(&slot.config_id, 1).await?;

    let book_token = commit_details
        .book_token
        .as_ref()
        .and_then(|t| t.value.as_deref())
        .ok_or_else(|| AppError::new(4, "details response missing book_token.value"))?;

    let payment_method_id = cli_payment_method_id.or_else(|| {
        quote_details
            .user
            .as_ref()
            .and_then(|u| u.payment_methods.as_ref())
            .and_then(|arr| arr.first())
            .and_then(|v| v.id)
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

    let booking_result = client
        .book(book_token, payment_method_id, false, false)
        .await?;
    let booking_raw = to_json_value(&booking_result)?;

    Ok(json!({
        "ok": true,
        "booked": true,
        "slot": slot,
        "quote": summary,
        "book_token_expires": commit_details.book_token.as_ref().and_then(|t| t.date_expires.as_deref()),
        "reservation_id": booking_result.reservation_id,
        "resy_token": booking_result.resy_token,
        "payment_method_id": payment_method_id,
        "result": booking_raw,
    }))
}
