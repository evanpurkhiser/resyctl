use serde_json::{Value, json};

use crate::api::ResyClient;
use crate::cli::AvailabilityArgs;
use crate::error::AppError;
use crate::util::{dates_in_month, extract_slots, start_to_hhmm, validate_date};

pub async fn run(client: &ResyClient, args: AvailabilityArgs) -> Result<Value, AppError> {
    match (&args.month, &args.date) {
        (Some(_), Some(_)) => {
            return Err(AppError::new(5, "pass only one of --month or --date"));
        }
        (None, None) => {
            return Err(AppError::new(5, "you must pass either --month or --date"));
        }
        _ => {}
    }

    if let Some(month) = args.month {
        if !args.days {
            return Err(AppError::new(
                5,
                "--month currently requires --days to return day-level availability",
            ));
        }

        let dates = dates_in_month(&month)?;
        let mut day_results = Vec::new();
        for date in dates {
            let date_str = date.format("%Y-%m-%d").to_string();
            let raw = client
                .find(
                    args.restaurant_id,
                    &date_str,
                    args.party_size,
                    args.lat,
                    args.lng,
                )
                .await?;
            let slots = extract_slots(&raw, args.restaurant_id, &date_str, args.party_size);
            if !slots.is_empty() {
                day_results.push(json!({
                    "date": date_str,
                    "available_slot_count": slots.len(),
                }));
            }
        }

        return Ok(json!({
            "ok": true,
            "mode": "days",
            "restaurant_id": args.restaurant_id,
            "month": month,
            "party_size": args.party_size,
            "days": day_results,
        }));
    }

    let date = args
        .date
        .ok_or_else(|| AppError::new(5, "--date is required for date availability mode"))?;
    validate_date(&date)?;

    let raw = client
        .find(
            args.restaurant_id,
            &date,
            args.party_size,
            args.lat,
            args.lng,
        )
        .await?;
    let mut slots = extract_slots(&raw, args.restaurant_id, &date, args.party_size);

    if let Some(seating) = args.seating {
        let seating_l = seating.to_lowercase();
        slots.retain(|slot| {
            slot.get("type")
                .and_then(Value::as_str)
                .map(|t| t.to_lowercase().contains(&seating_l))
                .unwrap_or(false)
        });
    }

    if args.time_after.is_some() || args.time_before.is_some() {
        slots.retain(|slot| {
            let time = slot
                .get("start")
                .and_then(Value::as_str)
                .and_then(start_to_hhmm);

            let Some(time) = time else { return false };
            let after_ok = args
                .time_after
                .as_deref()
                .map(|after| time >= after)
                .unwrap_or(true);
            let before_ok = args
                .time_before
                .as_deref()
                .map(|before| time <= before)
                .unwrap_or(true);
            after_ok && before_ok
        });
    }

    Ok(json!({
        "ok": true,
        "mode": "times",
        "restaurant_id": args.restaurant_id,
        "date": date,
        "party_size": args.party_size,
        "count": slots.len(),
        "slots": slots,
    }))
}
