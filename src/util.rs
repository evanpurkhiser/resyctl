use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use chrono::Utc;
use serde::Serialize;
use serde_json::{Value, json};

use crate::error::AppError;
use crate::models::{DetailsResponse, FindResponse};
use crate::types::{SlotId, TimeArg};

pub fn parse_rfc3339_utc(value: &str) -> Option<chrono::DateTime<Utc>> {
    chrono::DateTime::parse_from_rfc3339(value)
        .ok()
        .map(|dt| dt.with_timezone(&Utc))
}

pub fn encode_slot_id(payload: &SlotId) -> Result<String, AppError> {
    let raw = serde_json::to_vec(payload)
        .map_err(|e| AppError::new(4, format!("failed to encode slot_id: {e}")))?;
    Ok(URL_SAFE_NO_PAD.encode(raw))
}

pub fn decode_slot_id(slot_id: &str) -> Result<SlotId, AppError> {
    let raw = URL_SAFE_NO_PAD
        .decode(slot_id.as_bytes())
        .map_err(|_| AppError::new(5, "invalid slot_id encoding"))?;
    serde_json::from_slice::<SlotId>(&raw).map_err(|_| AppError::new(5, "invalid slot_id payload"))
}

#[derive(Debug, Clone, Serialize)]
pub struct SlotPaymentSummary {
    pub is_paid: Option<bool>,
    pub cancellation_fee: Option<f64>,
    pub deposit_fee: Option<f64>,
    pub secs_cancel_cut_off: Option<i64>,
    pub time_cancel_cut_off: Option<String>,
    pub secs_change_cut_off: Option<i64>,
    pub time_change_cut_off: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AvailableSlot {
    pub slot_id: String,
    pub start: Option<String>,
    pub end: Option<String>,
    #[serde(rename = "type")]
    pub slot_type: Option<String>,
    pub party_min: Option<i64>,
    pub party_max: Option<i64>,
    pub is_paid: bool,
    pub payment: Option<SlotPaymentSummary>,
    pub raw: Value,
}

impl AvailableSlot {
    pub fn seating_contains(&self, seating_filter: &str) -> bool {
        self.slot_type
            .as_deref()
            .map(|slot_type| slot_type.to_lowercase().contains(seating_filter))
            .unwrap_or(false)
    }

    pub fn local_start_time(&self) -> Option<TimeArg> {
        self.start.as_deref().and_then(TimeArg::parse_slot_start)
    }
}

pub fn extract_slots(
    find: &FindResponse,
    venue_id: i64,
    day: &str,
    party_size: u8,
) -> Result<Vec<AvailableSlot>, AppError> {
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
            })?;

            let raw = to_json_value(slot)?;

            let payment = slot.payment.as_ref().map(|p| SlotPaymentSummary {
                is_paid: p.is_paid,
                cancellation_fee: p.cancellation_fee,
                deposit_fee: p.deposit_fee,
                secs_cancel_cut_off: p.secs_cancel_cut_off,
                time_cancel_cut_off: p.time_cancel_cut_off.clone(),
                secs_change_cut_off: p.secs_change_cut_off,
                time_change_cut_off: p.time_change_cut_off.clone(),
            });

            out.push(AvailableSlot {
                slot_id,
                start,
                end,
                slot_type,
                party_min: slot.size.as_ref().and_then(|s| s.min),
                party_max: slot.size.as_ref().and_then(|s| s.max),
                is_paid: slot
                    .payment
                    .as_ref()
                    .and_then(|p| p.is_paid)
                    .unwrap_or(false),
                payment,
                raw,
            });
        }
    }
    Ok(out)
}

#[derive(Debug, Clone, Serialize)]
pub struct QuoteSummary {
    pub book_token_expires: Option<String>,
    pub fee_amount: Option<f64>,
    pub fee_tax: Option<f64>,
    pub fee_cutoff: Option<String>,
    pub refund_cutoff: Option<String>,
    pub change_cutoff: Option<String>,
    pub fee_display: Option<String>,
    pub payment_type: Option<String>,
    pub payment_amounts: Value,
    pub payment_methods: Value,
    pub policy_text: Option<String>,
    pub has_book_token: bool,
}

impl QuoteSummary {
    pub fn fee_amount(&self) -> f64 {
        self.fee_amount.unwrap_or(0.0)
    }

    pub fn fee_cutoff_at(&self) -> Option<chrono::DateTime<Utc>> {
        self.fee_cutoff.as_deref().and_then(parse_rfc3339_utc)
    }
}

pub fn quote_summary(details: &DetailsResponse) -> Result<QuoteSummary, AppError> {
    let policy_text = details
        .cancellation
        .as_ref()
        .and_then(|c| c.display.as_ref())
        .and_then(|d| d.policy.as_ref())
        .map(|p| p.join("\n"))
        .filter(|s| !s.is_empty());

    let payment_methods = to_json_value(
        details
            .user
            .as_ref()
            .and_then(|u| u.payment_methods.as_ref())
            .cloned()
            .unwrap_or_default(),
    )?;

    Ok(QuoteSummary {
        book_token_expires: details
            .book_token
            .as_ref()
            .and_then(|t| t.date_expires.clone()),
        fee_amount: details
            .cancellation
            .as_ref()
            .and_then(|c| c.fee.as_ref())
            .and_then(|f| f.amount),
        fee_tax: details
            .cancellation
            .as_ref()
            .and_then(|c| c.fee.as_ref())
            .and_then(|f| f.tax),
        fee_cutoff: details
            .cancellation
            .as_ref()
            .and_then(|c| c.fee.as_ref())
            .and_then(|f| f.date_cut_off.clone()),
        refund_cutoff: details
            .cancellation
            .as_ref()
            .and_then(|c| c.refund.as_ref())
            .and_then(|r| r.date_cut_off.clone()),
        change_cutoff: details.change.as_ref().and_then(|c| c.date_cut_off.clone()),
        fee_display: details
            .cancellation
            .as_ref()
            .and_then(|c| c.fee.as_ref())
            .and_then(|f| f.display.as_ref())
            .and_then(|d| d.amount.clone()),
        payment_type: details
            .payment
            .as_ref()
            .and_then(|p| p.config.as_ref())
            .and_then(|c| c.kind.clone()),
        payment_amounts: json!({
            "reservation_charge": details.payment.as_ref().and_then(|p| p.amounts.as_ref()).and_then(|a| a.reservation_charge),
            "subtotal": details.payment.as_ref().and_then(|p| p.amounts.as_ref()).and_then(|a| a.subtotal),
            "resy_fee": details.payment.as_ref().and_then(|p| p.amounts.as_ref()).and_then(|a| a.resy_fee),
            "service_fee": details.payment.as_ref().and_then(|p| p.amounts.as_ref()).and_then(|a| a.service_fee),
            "tax": details.payment.as_ref().and_then(|p| p.amounts.as_ref()).and_then(|a| a.tax),
            "total": details.payment.as_ref().and_then(|p| p.amounts.as_ref()).and_then(|a| a.total),
        }),
        payment_methods,
        policy_text,
        has_book_token: details
            .book_token
            .as_ref()
            .and_then(|t| t.value.as_ref())
            .is_some(),
    })
}

pub fn to_json_value<T: Serialize>(value: T) -> Result<Value, AppError> {
    serde_json::to_value(value)
        .map_err(|e| AppError::new(4, format!("failed to serialize JSON value: {e}")))
}
