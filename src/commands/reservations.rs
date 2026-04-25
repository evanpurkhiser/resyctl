use serde_json::{Value, json};
use chrono::NaiveDate;
use chrono::Utc;

use crate::api::ResyClient;
use crate::cli::ReservationsArgs;
use crate::error::AppError;

pub async fn run(client: &ResyClient, args: ReservationsArgs) -> Result<Value, AppError> {
    let raw = client
        .reservations(args.resy_token.as_deref(), args.limit, args.offset)
        .await?;

    let today = Utc::now().date_naive();
    let apply_upcoming_filter = !args.all && args.upcoming;

    let normalized_all: Vec<Value> = raw
        .reservations
        .iter()
        .filter(|r| {
            if !apply_upcoming_filter {
                return true;
            }

            let day_ok = r
                .day
                .as_deref()
                .and_then(|d| NaiveDate::parse_from_str(d, "%Y-%m-%d").ok())
                .map(|d| d >= today)
                .unwrap_or(false);

            let not_finished = r
                .status
                .as_ref()
                .and_then(|s| s.finished)
                .map(|v| v == 0)
                .unwrap_or(true);

            let not_no_show = r
                .status
                .as_ref()
                .and_then(|s| s.no_show)
                .map(|v| v == 0)
                .unwrap_or(true);

            day_ok && not_finished && not_no_show
        })
        .map(|r| {
            let raw_item = serde_json::to_value(r).unwrap_or_else(|_| Value::Null);
            let venue_id = r
                .venue
                .as_ref()
                .and_then(|v| v.id)
                .or(r.venue_id);
            let venue_name = r
                .venue
                .as_ref()
                .and_then(|v| v.name.as_deref())
                .map(str::to_string)
                .or_else(|| {
                    venue_id.and_then(|id| {
                        raw.venues
                            .as_ref()
                            .and_then(|v| v.get(id.to_string()))
                            .and_then(|v| v.get("name"))
                            .and_then(Value::as_str)
                            .map(str::to_string)
                    })
                });
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
                    "id": venue_id,
                    "name": venue_name,
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

    let mut normalized = normalized_all;
    normalized.sort_by(|a, b| {
        let ad = a.get("day").and_then(Value::as_str).unwrap_or("");
        let at = a.get("time_slot").and_then(Value::as_str).unwrap_or("");
        let bd = b.get("day").and_then(Value::as_str).unwrap_or("");
        let bt = b.get("time_slot").and_then(Value::as_str).unwrap_or("");
        (ad, at).cmp(&(bd, bt))
    });

    let raw_value = serde_json::to_value(&raw).unwrap_or_else(|_| Value::Null);

    Ok(json!({
        "ok": true,
        "input": {
            "resy_token_present": args.resy_token.as_ref().map(|s| !s.is_empty()).unwrap_or(false),
            "upcoming": apply_upcoming_filter,
            "all": args.all,
            "limit": args.limit,
            "offset": args.offset,
        },
        "count": normalized.len(),
        "reservations": normalized,
        "metadata": raw.metadata,
        "raw": raw_value,
    }))
}
