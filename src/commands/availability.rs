use serde_json::{Value, json};

use crate::api::ResyClient;
use crate::cli::AvailabilityArgs;
use crate::error::{Error, InputError};
use crate::util::{extract_slots, to_json_value};

pub async fn run(client: &ResyClient, args: AvailabilityArgs) -> Result<Value, Error> {
    match (&args.month, &args.date) {
        (Some(_), Some(_)) => {
            return Err(InputError::AvailabilityCannotMixMonthAndDate.into());
        }
        (None, None) => {
            return Err(InputError::AvailabilityRequiresMonthOrDate.into());
        }
        _ => {}
    }

    if let Some(month) = args.month {
        if !args.days {
            return Err(InputError::AvailabilityMonthRequiresDays.into());
        }

        let mut day_results = Vec::new();
        for date in month.days() {
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
            let slots = extract_slots(&raw, args.restaurant_id, date, args.party_size)?;
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
            "month": month.to_string(),
            "party_size": args.party_size,
            "days": day_results,
        }));
    }

    let date = args
        .date
        .ok_or(InputError::AvailabilityDateModeRequiresDate)?;
    let date_str = date.to_string();

    let raw = client
        .find(
            args.restaurant_id,
            &date_str,
            args.party_size,
            args.lat,
            args.lng,
        )
        .await?;
    let mut slots = extract_slots(&raw, args.restaurant_id, date.0, args.party_size)?;

    if let Some(seating) = args.seating {
        let seating_l = seating.to_lowercase();
        slots.retain(|slot| slot.seating_contains(&seating_l));
    }

    if args.time_after.is_some() || args.time_before.is_some() {
        slots.retain(|slot| {
            let Some(time) = slot.local_start_time() else {
                return false;
            };
            let after_ok = args.time_after.map(|after| time >= after).unwrap_or(true);
            let before_ok = args
                .time_before
                .map(|before| time <= before)
                .unwrap_or(true);
            after_ok && before_ok
        });
    }

    Ok(json!({
        "ok": true,
        "mode": "times",
        "restaurant_id": args.restaurant_id,
        "date": date_str,
        "party_size": args.party_size,
        "count": slots.len(),
        "slots": slots,
        "raw": to_json_value(&raw)?,
    }))
}
