use serde_json::{Value, json};

use crate::api::ResyClient;
use crate::cli::ReservationsArgs;
use crate::error::AppError;

pub async fn run(client: &ResyClient, args: ReservationsArgs) -> Result<Value, AppError> {
    let raw = client.reservation_by_token(&args.resy_token).await?;

    let normalized: Vec<Value> = raw
        .reservations
        .iter()
        .map(|r| {
            let raw_item = serde_json::to_value(r).unwrap_or_else(|_| Value::Null);
            json!({
                "reservation_id": r.reservation_id,
                "resy_token": r.resy_token,
                "day": r.day,
                "time_slot": r.time_slot,
                "num_seats": r.num_seats,
                "status": {
                    "finished": r.status.as_ref().and_then(|s| s.finished),
                    "no_show": r.status.as_ref().and_then(|s| s.no_show),
                },
                "venue": {
                    "id": r.venue.as_ref().and_then(|v| v.id),
                    "name": r.venue.as_ref().and_then(|v| v.name.as_deref()),
                },
                "cancellation": {
                    "allowed": r.cancellation.as_ref().and_then(|c| c.allowed),
                    "fee_amount": r.cancellation.as_ref().and_then(|c| c.fee.as_ref()).and_then(|f| f.amount),
                    "fee_display": r.cancellation.as_ref().and_then(|c| c.fee.as_ref()).and_then(|f| f.display.as_ref()).and_then(|d| d.amount.as_deref()),
                    "fee_cutoff": r.cancellation.as_ref().and_then(|c| c.fee.as_ref()).and_then(|f| f.date_cut_off.as_deref()),
                    "refund_cutoff": r.cancellation.as_ref().and_then(|c| c.date_refund_cut_off.as_deref()),
                    "policy": r.cancellation_policy,
                },
                "payment": {
                    "payment_method": r.payment_method,
                    "invoice": {
                        "subtotal": r.payment.as_ref().and_then(|p| p.invoice.as_ref()).and_then(|i| i.subtotal),
                        "tax": r.payment.as_ref().and_then(|p| p.invoice.as_ref()).and_then(|i| i.tax),
                        "service_fee": r.payment.as_ref().and_then(|p| p.invoice.as_ref()).and_then(|i| i.service_fee),
                        "resy_fee": r.payment.as_ref().and_then(|p| p.invoice.as_ref()).and_then(|i| i.resy_fee),
                        "total": r.payment.as_ref().and_then(|p| p.invoice.as_ref()).and_then(|i| i.total),
                    }
                },
                "raw": raw_item,
            })
        })
        .collect();

    let raw_value = serde_json::to_value(&raw).unwrap_or_else(|_| Value::Null);

    Ok(json!({
        "ok": true,
        "input": {
            "resy_token_present": !args.resy_token.is_empty(),
        },
        "count": normalized.len(),
        "reservations": normalized,
        "metadata": raw.metadata,
        "raw": raw_value,
    }))
}
