use chrono::{DateTime, NaiveDate, NaiveTime, Utc};
use serde::Serialize;
use serde_json::{Value, json};

use crate::api::ResyClient;
use crate::cli::ReservationsArgs;
use crate::error::Error;
use crate::models::{PaymentMethod, ReservationItem, ReservationLookupResponse};
use crate::types::ResyToken;
use crate::util::to_json_value;

#[derive(Debug, Serialize)]
struct NormalizedReservation {
    reservation_id: Option<i64>,
    resy_token: Option<ResyToken>,
    day: Option<NaiveDate>,
    time_slot: Option<NaiveTime>,
    num_seats: Option<i64>,
    status: NormalizedStatus,
    venue: NormalizedVenue,
    cancellation: NormalizedCancellation,
    payment: NormalizedPayment,
    raw: Value,
}

#[derive(Debug, Serialize)]
struct NormalizedStatus {
    finished: Option<bool>,
    no_show: Option<bool>,
}

#[derive(Debug, Serialize)]
struct NormalizedVenue {
    id: Option<i64>,
    name: Option<String>,
}

#[derive(Debug, Serialize)]
struct NormalizedCancellation {
    allowed: Option<bool>,
    cancellation_fee_amount: Option<f64>,
    cancellation_fee_display: Option<String>,
    cancellation_fee_cutoff: Option<DateTime<Utc>>,
    refund_cutoff: Option<DateTime<Utc>>,
    policy: Option<Vec<String>>,
}

#[derive(Debug, Serialize)]
struct NormalizedPayment {
    payment_method: Option<PaymentMethod>,
    invoice: NormalizedInvoice,
}

#[derive(Debug, Serialize)]
struct NormalizedInvoice {
    subtotal: Option<f64>,
    tax: Option<f64>,
    service_fee: Option<f64>,
    resy_fee: Option<f64>,
    total: Option<f64>,
}

impl NormalizedReservation {
    fn from_item(item: &ReservationItem, venues: Option<&Value>) -> Result<Self, Error> {
        let venue_id = item.venue.as_ref().and_then(|v| v.id).or(item.venue_id);
        let venue_name = item
            .venue
            .as_ref()
            .and_then(|v| v.name.clone())
            .or_else(|| venue_name_from_lookup(venues, venue_id));

        Ok(Self {
            reservation_id: item.reservation_id,
            resy_token: item.resy_token.clone(),
            day: item.day.clone(),
            time_slot: item.time_slot.clone(),
            num_seats: item.num_seats,
            status: NormalizedStatus {
                finished: item.status.as_ref().and_then(|s| s.finished),
                no_show: item.status.as_ref().and_then(|s| s.no_show),
            },
            venue: NormalizedVenue {
                id: venue_id,
                name: venue_name,
            },
            cancellation: NormalizedCancellation {
                allowed: item.cancellation.as_ref().and_then(|c| c.allowed),
                cancellation_fee_amount: item
                    .cancellation
                    .as_ref()
                    .and_then(|c| c.fee.as_ref())
                    .and_then(|f| f.amount),
                cancellation_fee_display: item
                    .cancellation
                    .as_ref()
                    .and_then(|c| c.fee.as_ref())
                    .and_then(|f| f.display.as_ref())
                    .and_then(|d| d.amount.clone()),
                cancellation_fee_cutoff: item
                    .cancellation
                    .as_ref()
                    .and_then(|c| c.fee.as_ref())
                    .and_then(|f| f.date_cut_off.clone()),
                refund_cutoff: item
                    .cancellation
                    .as_ref()
                    .and_then(|c| c.date_refund_cut_off.clone()),
                policy: item.cancellation_policy.clone(),
            },
            payment: NormalizedPayment {
                payment_method: item.payment_method.clone(),
                invoice: NormalizedInvoice {
                    subtotal: item
                        .payment
                        .as_ref()
                        .and_then(|p| p.invoice.as_ref())
                        .and_then(|i| i.subtotal),
                    tax: item
                        .payment
                        .as_ref()
                        .and_then(|p| p.invoice.as_ref())
                        .and_then(|i| i.tax),
                    service_fee: item
                        .payment
                        .as_ref()
                        .and_then(|p| p.invoice.as_ref())
                        .and_then(|i| i.service_fee),
                    resy_fee: item
                        .payment
                        .as_ref()
                        .and_then(|p| p.invoice.as_ref())
                        .and_then(|i| i.resy_fee),
                    total: item
                        .payment
                        .as_ref()
                        .and_then(|p| p.invoice.as_ref())
                        .and_then(|i| i.total),
                },
            },
            raw: to_json_value(item)?,
        })
    }

    fn sort_key(&self) -> (Option<NaiveDate>, Option<NaiveTime>) {
        (self.day, self.time_slot)
    }
}

fn venue_name_from_lookup(venues: Option<&Value>, venue_id: Option<i64>) -> Option<String> {
    let venue_id = venue_id?;
    let key = venue_id.to_string();
    venues
        .and_then(|v| v.get(&key))
        .and_then(|v| v.get("name"))
        .and_then(Value::as_str)
        .map(str::to_string)
}

fn is_upcoming_reservation(item: &ReservationItem, today: NaiveDate) -> bool {
    let is_today_or_future = item.day.map(|day| day >= today).unwrap_or(false);

    let not_finished = item
        .status
        .as_ref()
        .and_then(|s| s.finished)
        .map(|finished| !finished)
        .unwrap_or(true);

    let not_no_show = item
        .status
        .as_ref()
        .and_then(|s| s.no_show)
        .map(|no_show| !no_show)
        .unwrap_or(true);

    is_today_or_future && not_finished && not_no_show
}

pub async fn run(client: &ResyClient, args: ReservationsArgs) -> Result<Value, Error> {
    let raw = client
        .reservations(args.resy_token.as_ref(), args.limit, args.offset)
        .await?;

    let today = Utc::now().date_naive();
    let apply_upcoming_filter = !args.all && args.upcoming;

    let mut normalized: Vec<NormalizedReservation> = raw
        .reservations
        .iter()
        .filter(|item| !apply_upcoming_filter || is_upcoming_reservation(item, today))
        .map(|item| NormalizedReservation::from_item(item, raw.venues.as_ref()))
        .collect::<Result<_, Error>>()?;

    normalized.sort_by(|left, right| left.sort_key().cmp(&right.sort_key()));

    let raw_value = to_json_value(&raw)?;

    let ReservationLookupResponse {
        metadata,
        reservations: _,
        venues: _,
    } = raw;

    Ok(json!({
        "ok": true,
        "input": {
            "resy_token_present": args.resy_token.as_ref().map(|s| !s.as_str().is_empty()).unwrap_or(false),
            "upcoming": apply_upcoming_filter,
            "all": args.all,
            "limit": args.limit,
            "offset": args.offset,
        },
        "count": normalized.len(),
        "reservations": normalized,
        "metadata": metadata,
        "raw": raw_value,
    }))
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;
    use serde_json::json;

    use super::{is_upcoming_reservation, venue_name_from_lookup};
    use crate::models::ReservationItem;

    #[test]
    fn upcoming_filter_requires_future_and_not_finished() {
        let item: ReservationItem = serde_json::from_value(json!({
            "day": "2026-04-30",
            "status": {"finished": 0, "no_show": 0}
        }))
        .expect("valid reservation item");
        let finished: ReservationItem = serde_json::from_value(json!({
            "day": "2026-04-30",
            "status": {"finished": 1, "no_show": 0}
        }))
        .expect("valid reservation item");

        assert!(is_upcoming_reservation(
            &item,
            NaiveDate::from_ymd_opt(2026, 4, 24).expect("valid date")
        ));
        assert!(!is_upcoming_reservation(
            &finished,
            NaiveDate::from_ymd_opt(2026, 4, 24).expect("valid date")
        ));
    }

    #[test]
    fn venue_name_falls_back_to_lookup() {
        let venues = json!({
            "84214": {"name": "Ishq"}
        });

        assert_eq!(
            venue_name_from_lookup(Some(&venues), Some(84214)),
            Some("Ishq".to_string())
        );
        assert_eq!(venue_name_from_lookup(Some(&venues), Some(1)), None);
    }
}
