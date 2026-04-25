use chrono::{DateTime, NaiveDate, NaiveDateTime, NaiveTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Map;
use serde_json::Value;

use crate::types::{BookToken, ConfigId, ResyToken};

/// Resy returns slot start/end as `"YYYY-MM-DD HH:MM:SS"` (space-delimited,
/// no timezone), which doesn't match chrono's default NaiveDateTime serde
/// format. This module decodes/encodes that wire format.
mod naive_datetime_space {
    use chrono::NaiveDateTime;
    use serde::{Deserialize, Deserializer, Serializer, de::Error};

    const FORMAT: &str = "%Y-%m-%d %H:%M:%S";

    pub fn serialize<S: Serializer>(
        value: &Option<NaiveDateTime>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        match value {
            Some(dt) => serializer.serialize_str(&dt.format(FORMAT).to_string()),
            None => serializer.serialize_none(),
        }
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Option<NaiveDateTime>, D::Error> {
        match Option::<String>::deserialize(deserializer)? {
            Some(raw) => NaiveDateTime::parse_from_str(&raw, FORMAT)
                .map(Some)
                .map_err(Error::custom),
            None => Ok(None),
        }
    }
}

/// Resy uses 0/1 integers for boolean status flags. Decode them as `bool`
/// so callers don't have to remember the convention.
mod int_as_bool {
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(
        value: &Option<bool>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        match value {
            Some(b) => serializer.serialize_i64(if *b { 1 } else { 0 }),
            None => serializer.serialize_none(),
        }
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Option<bool>, D::Error> {
        Ok(Option::<i64>::deserialize(deserializer)?.map(|n| n != 0))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthPasswordResponse {
    pub token: Option<String>,
    pub payment_method_id: Option<i64>,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserResponse {
    pub id: Option<i64>,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    #[serde(rename = "em_address")]
    pub email: Option<String>,
    pub payment_method_id: Option<i64>,
    pub payment_methods: Option<Vec<PaymentMethod>>,
    pub num_bookings: Option<i64>,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResponse {
    pub search: SearchPayload,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchPayload {
    #[serde(default)]
    pub hits: Vec<SearchHit>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchHit {
    pub id: Option<SearchHitId>,
    pub name: Option<String>,
    pub locality: Option<String>,
    pub neighborhood: Option<String>,
    pub cuisine: Option<Vec<String>>,
    pub rating: Option<SearchHitRating>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchHitId {
    pub resy: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchHitRating {
    pub average: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FindResponse {
    pub results: Option<FindResults>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FindResults {
    #[serde(default)]
    pub venues: Vec<FindVenue>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FindVenue {
    #[serde(default)]
    pub slots: Vec<FindSlot>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FindSlot {
    pub config: Option<SlotConfig>,
    pub date: Option<SlotDate>,
    pub size: Option<SlotSize>,
    pub payment: Option<SlotPayment>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlotConfig {
    pub token: Option<ConfigId>,
    #[serde(rename = "type")]
    pub kind: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlotDate {
    #[serde(default, with = "naive_datetime_space")]
    pub start: Option<NaiveDateTime>,
    #[serde(default, with = "naive_datetime_space")]
    pub end: Option<NaiveDateTime>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlotSize {
    pub min: Option<i64>,
    pub max: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlotPayment {
    pub is_paid: Option<bool>,
    pub cancellation_fee: Option<f64>,
    pub deposit_fee: Option<f64>,
    pub secs_cancel_cut_off: Option<i64>,
    pub time_cancel_cut_off: Option<DateTime<Utc>>,
    pub secs_change_cut_off: Option<i64>,
    pub time_change_cut_off: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetailsResponse {
    pub book_token: Option<BookTokenInfo>,
    pub cancellation: Option<Cancellation>,
    pub change: Option<ChangePolicy>,
    pub payment: Option<Payment>,
    pub user: Option<DetailsUser>,
    pub config: Option<Value>,
    pub venue: Option<Value>,
    pub viewers: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BookTokenInfo {
    pub value: Option<BookToken>,
    pub date_expires: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Cancellation {
    pub display: Option<CancellationDisplay>,
    pub fee: Option<CancellationFee>,
    pub refund: Option<CancellationRefund>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CancellationDisplay {
    pub policy: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CancellationFee {
    pub amount: Option<f64>,
    pub tax: Option<f64>,
    pub date_cut_off: Option<DateTime<Utc>>,
    pub display: Option<CancellationFeeDisplay>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CancellationFeeDisplay {
    pub amount: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CancellationRefund {
    pub date_cut_off: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangePolicy {
    pub date_cut_off: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Payment {
    pub config: Option<PaymentConfig>,
    pub amounts: Option<PaymentAmounts>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentConfig {
    #[serde(rename = "type")]
    pub kind: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentAmounts {
    pub reservation_charge: Option<f64>,
    pub subtotal: Option<f64>,
    pub resy_fee: Option<f64>,
    pub service_fee: Option<f64>,
    pub tax: Option<f64>,
    pub total: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetailsUser {
    pub payment_methods: Option<Vec<PaymentMethod>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentMethod {
    pub id: Option<i64>,
    pub card_type: Option<String>,
    pub last_4: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BookResponse {
    pub reservation_id: Option<i64>,
    pub resy_token: Option<ResyToken>,
    pub specs: Option<Value>,
    pub venue: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReservationLookupResponse {
    #[serde(default)]
    pub reservations: Vec<ReservationItem>,
    pub metadata: Option<Value>,
    pub venues: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReservationItem {
    pub reservation_id: Option<i64>,
    pub resy_token: Option<ResyToken>,
    pub venue_id: Option<i64>,
    pub day: Option<NaiveDate>,
    pub time_slot: Option<NaiveTime>,
    pub num_seats: Option<i64>,
    pub status: Option<ReservationStatus>,
    pub venue: Option<ReservationVenue>,
    pub cancellation: Option<ReservationCancellation>,
    pub cancellation_policy: Option<Vec<String>>,
    pub payment_method: Option<PaymentMethod>,
    pub payment: Option<ReservationPayment>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReservationStatus {
    #[serde(default, with = "int_as_bool")]
    pub finished: Option<bool>,
    #[serde(default, with = "int_as_bool")]
    pub no_show: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReservationVenue {
    pub id: Option<i64>,
    pub name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReservationCancellation {
    pub allowed: Option<bool>,
    pub date_refund_cut_off: Option<DateTime<Utc>>,
    pub fee: Option<ReservationFee>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReservationFee {
    pub amount: Option<f64>,
    pub date_cut_off: Option<DateTime<Utc>>,
    pub display: Option<CancellationFeeDisplay>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReservationPayment {
    pub invoice: Option<PaymentAmounts>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CancelResponse {
    pub payment: Option<CancelPayment>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CancelPayment {
    pub transaction: Option<CancelTransaction>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CancelTransaction {
    pub refund: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VenueResponse {
    pub id: Option<SearchHitId>,
    pub name: Option<String>,
    pub url_slug: Option<String>,
    #[serde(rename = "type")]
    pub kind: Option<String>,
    pub location: Option<VenueLocation>,
    pub contact: Option<VenueContact>,
    pub links: Option<VenueLinks>,
    pub metadata: Option<VenueMetadata>,
    #[serde(default)]
    pub content: Vec<VenueContent>,
    #[serde(default)]
    pub social: Vec<VenueSocial>,
    pub price_range_id: Option<i64>,
    pub rater: Option<Value>,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VenueLocation {
    pub address_1: Option<String>,
    pub address_2: Option<String>,
    pub locality: Option<String>,
    pub region: Option<String>,
    pub postal_code: Option<String>,
    pub country: Option<String>,
    pub neighborhood: Option<String>,
    pub cross_street_1: Option<String>,
    pub cross_street_2: Option<String>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    /// City slug used in resy.com URLs (e.g. "new-york-ny").
    pub url_slug: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VenueContact {
    pub phone_number: Option<String>,
    pub url: Option<String>,
    pub menu_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VenueLinks {
    pub web: Option<String>,
    pub deep: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VenueMetadata {
    pub description: Option<String>,
    #[serde(default)]
    pub keywords: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VenueContent {
    pub name: Option<String>,
    pub title: Option<String>,
    pub body: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VenueSocial {
    pub name: Option<String>,
    pub value: Option<String>,
}
