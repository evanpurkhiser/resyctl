use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use chrono::{Datelike, NaiveDate, Utc};
use serde_json::{Value, json};

use crate::error::AppError;
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

pub fn extract_slots(find: &Value, venue_id: i64, day: &str, party_size: u8) -> Vec<Value> {
    let venues = find
        .pointer("/results/venues")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    let mut out = Vec::new();
    for venue in venues {
        let slots = venue
            .get("slots")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        for slot in slots {
            let config_id = slot
                .pointer("/config/token")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string();
            if config_id.is_empty() {
                continue;
            }

            let slot_type = slot
                .pointer("/config/type")
                .and_then(Value::as_str)
                .map(|s| s.to_string());
            let start = slot
                .pointer("/date/start")
                .and_then(Value::as_str)
                .map(|s| s.to_string());
            let end = slot
                .pointer("/date/end")
                .and_then(Value::as_str)
                .map(|s| s.to_string());
            let slot_id = encode_slot_id(&SlotId {
                config_id,
                day: day.to_string(),
                party_size,
                venue_id,
                start: start.clone(),
                slot_type: slot_type.clone(),
            });

            out.push(json!({
                "slot_id": slot_id,
                "start": start,
                "end": end,
                "type": slot_type,
                "party_min": slot.pointer("/size/min").and_then(Value::as_i64),
                "party_max": slot.pointer("/size/max").and_then(Value::as_i64),
                "is_paid": slot.pointer("/payment/is_paid").and_then(Value::as_bool).unwrap_or(false),
                "raw": slot,
            }));
        }
    }
    out
}

pub fn quote_summary(details: &Value) -> Value {
    let policy_text = details
        .pointer("/cancellation/display/policy")
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .filter_map(Value::as_str)
                .collect::<Vec<_>>()
                .join("\n")
        })
        .filter(|s| !s.is_empty());

    json!({
        "book_token_expires": details.pointer("/book_token/date_expires").and_then(Value::as_str),
        "fee_amount": details.pointer("/cancellation/fee/amount").and_then(Value::as_f64),
        "fee_cutoff": details.pointer("/cancellation/fee/date_cut_off").and_then(Value::as_str),
        "refund_cutoff": details.pointer("/cancellation/refund/date_cut_off").and_then(Value::as_str),
        "fee_display": details.pointer("/cancellation/fee/display/amount").and_then(Value::as_str),
        "payment_type": details.pointer("/payment/config/type").and_then(Value::as_str),
        "policy_text": policy_text,
        "has_book_token": details.pointer("/book_token/value").and_then(Value::as_str).is_some(),
    })
}
