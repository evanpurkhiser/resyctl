use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use chrono::{Datelike, NaiveDate, Utc};
use serde_json::{Value, json};

use crate::error::AppError;
use crate::models::{DetailsResponse, FindResponse};
use crate::types::SlotId;

pub fn validate_date(date: &str) -> Result<(), AppError> {
    NaiveDate::parse_from_str(date, "%Y-%m-%d")
        .map(|_| ())
        .map_err(|_| AppError::new(5, format!("invalid date format: {date} (expected YYYY-MM-DD)")))
}

pub fn dates_in_month(month: &str) -> Result<Vec<NaiveDate>, AppError> {
    let m = format!("{month}-01");
    let first = NaiveDate::parse_from_str(&m, "%Y-%m-%d")
        .map_err(|_| AppError::new(5, format!("invalid month format: {month} (expected YYYY-MM)")))?;

    let mut dates = Vec::new();
    let mut day = first;
    while day.month() == first.month() {
        dates.push(day);
        day = day
            .succ_opt()
            .ok_or_else(|| AppError::new(4, "failed iterating month dates"))?;
    }
    Ok(dates)
}

pub fn parse_rfc3339_utc(value: &str) -> Option<chrono::DateTime<Utc>> {
    chrono::DateTime::parse_from_rfc3339(value)
        .ok()
        .map(|dt| dt.with_timezone(&Utc))
}

pub fn start_to_hhmm(start: &str) -> Option<&str> {
    start.split_whitespace().nth(1).and_then(|t| t.get(0..5))
}

pub fn encode_slot_id(payload: &SlotId) -> String {
    let raw = serde_json::to_vec(payload).unwrap_or_else(|_| b"{}".to_vec());
    URL_SAFE_NO_PAD.encode(raw)
}

pub fn decode_slot_id(slot_id: &str) -> Result<SlotId, AppError> {
    let raw = URL_SAFE_NO_PAD
        .decode(slot_id.as_bytes())
        .map_err(|_| AppError::new(5, "invalid slot_id encoding"))?;
    serde_json::from_slice::<SlotId>(&raw)
        .map_err(|_| AppError::new(5, "invalid slot_id payload"))
}

pub fn extract_slots(find: &FindResponse, venue_id: i64, day: &str, party_size: u8) -> Vec<Value> {
    let mut out = Vec::new();
    let venues = find.results.as_ref().map(|v| &v.venues);
    for venue in venues.into_iter().flatten() {
        for slot in &venue.slots {
            let config_id = slot
                .config
                .as_ref()
                .and_then(|c| c.token.as_ref())
                .cloned()
                .unwrap_or_default();
            if config_id.is_empty() {
                continue;
            }

            let slot_type = slot.config.as_ref().and_then(|c| c.kind.clone());
            let start = slot.date.as_ref().and_then(|d| d.start.clone());
            let end = slot.date.as_ref().and_then(|d| d.end.clone());
            let slot_id = encode_slot_id(&SlotId {
                config_id,
                day: day.to_string(),
                party_size,
                venue_id,
                start: start.clone(),
                slot_type: slot_type.clone(),
            });

            let raw = serde_json::to_value(slot).unwrap_or_else(|_| Value::Null);

            out.push(json!({
                "slot_id": slot_id,
                "start": start,
                "end": end,
                "type": slot_type,
                "party_min": slot.size.as_ref().and_then(|s| s.min),
                "party_max": slot.size.as_ref().and_then(|s| s.max),
                "is_paid": slot.payment.as_ref().and_then(|p| p.is_paid).unwrap_or(false),
                "payment": slot.payment.as_ref().map(|p| json!({
                    "is_paid": p.is_paid,
                    "cancellation_fee": p.cancellation_fee,
                    "deposit_fee": p.deposit_fee,
                    "secs_cancel_cut_off": p.secs_cancel_cut_off,
                    "time_cancel_cut_off": p.time_cancel_cut_off,
                    "secs_change_cut_off": p.secs_change_cut_off,
                    "time_change_cut_off": p.time_change_cut_off,
                })),
                "raw": raw,
            }));
        }
    }
    out
}

pub fn quote_summary(details: &DetailsResponse) -> Value {
    let policy_text = details
        .cancellation
        .as_ref()
        .and_then(|c| c.display.as_ref())
        .and_then(|d| d.policy.as_ref())
        .map(|p| p.join("\n"))
        .filter(|s| !s.is_empty());

    let payment_methods = details
        .user
        .as_ref()
        .and_then(|u| u.payment_methods.as_ref())
        .cloned()
        .unwrap_or_default();

    json!({
        "book_token_expires": details.book_token.as_ref().and_then(|t| t.date_expires.as_ref()),
        "fee_amount": details.cancellation.as_ref().and_then(|c| c.fee.as_ref()).and_then(|f| f.amount),
        "fee_tax": details.cancellation.as_ref().and_then(|c| c.fee.as_ref()).and_then(|f| f.tax),
        "fee_cutoff": details.cancellation.as_ref().and_then(|c| c.fee.as_ref()).and_then(|f| f.date_cut_off.as_ref()),
        "refund_cutoff": details.cancellation.as_ref().and_then(|c| c.refund.as_ref()).and_then(|r| r.date_cut_off.as_ref()),
        "change_cutoff": details.change.as_ref().and_then(|c| c.date_cut_off.as_ref()),
        "fee_display": details.cancellation.as_ref().and_then(|c| c.fee.as_ref()).and_then(|f| f.display.as_ref()).and_then(|d| d.amount.as_ref()),
        "payment_type": details.payment.as_ref().and_then(|p| p.config.as_ref()).and_then(|c| c.kind.as_ref()),
        "payment_amounts": {
            "reservation_charge": details.payment.as_ref().and_then(|p| p.amounts.as_ref()).and_then(|a| a.reservation_charge),
            "subtotal": details.payment.as_ref().and_then(|p| p.amounts.as_ref()).and_then(|a| a.subtotal),
            "resy_fee": details.payment.as_ref().and_then(|p| p.amounts.as_ref()).and_then(|a| a.resy_fee),
            "service_fee": details.payment.as_ref().and_then(|p| p.amounts.as_ref()).and_then(|a| a.service_fee),
            "tax": details.payment.as_ref().and_then(|p| p.amounts.as_ref()).and_then(|a| a.tax),
            "total": details.payment.as_ref().and_then(|p| p.amounts.as_ref()).and_then(|a| a.total),
        },
        "payment_methods": payment_methods,
        "policy_text": policy_text,
        "has_book_token": details.book_token.as_ref().and_then(|t| t.value.as_ref()).is_some(),
    })
}
