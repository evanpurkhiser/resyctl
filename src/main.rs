use std::env;
use std::fs;
use std::path::Path;
use std::process::ExitCode;

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use chrono::{Datelike, NaiveDate, Utc};
use clap::{Args, Parser, Subcommand};
use reqwest::header::{AUTHORIZATION, CONTENT_TYPE, HeaderMap, HeaderValue};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

const DEFAULT_API_KEY: &str = "AIcdK2rLXG6TYwJseSbmrBAy3RP81ocd";
const DEFAULT_LAT: f64 = 40.7128;
const DEFAULT_LNG: f64 = -74.0060;

#[derive(Parser, Debug)]
#[command(name = "ressy", about = "Resy CLI (JSON output only)")]
struct Cli {
    #[arg(long, global = true)]
    auth_token: Option<String>,
    #[arg(long, global = true)]
    api_key: Option<String>,
    #[arg(long, global = true)]
    payment_method_id: Option<i64>,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    Search(SearchArgs),
    Availability(AvailabilityArgs),
    Quote(QuoteArgs),
    Book(BookArgs),
}

#[derive(Args, Debug)]
struct SearchArgs {
    query: String,
    #[arg(long, default_value_t = 10)]
    limit: u32,
    #[arg(long, default_value_t = DEFAULT_LAT)]
    lat: f64,
    #[arg(long, default_value_t = DEFAULT_LNG)]
    lng: f64,
}

#[derive(Args, Debug)]
struct AvailabilityArgs {
    restaurant_id: i64,
    #[arg(long)]
    month: Option<String>,
    #[arg(long)]
    days: bool,
    #[arg(long)]
    date: Option<String>,
    #[arg(long, default_value_t = 2)]
    party_size: u8,
    #[arg(long)]
    seating: Option<String>,
    #[arg(long)]
    time_after: Option<String>,
    #[arg(long)]
    time_before: Option<String>,
    #[arg(long, default_value_t = DEFAULT_LAT)]
    lat: f64,
    #[arg(long, default_value_t = DEFAULT_LNG)]
    lng: f64,
}

#[derive(Args, Debug)]
struct QuoteArgs {
    slot_id: String,
}

#[derive(Args, Debug)]
struct BookArgs {
    slot_id: String,
    #[arg(long)]
    allow_fee: bool,
    #[arg(long)]
    max_fee: Option<f64>,
    #[arg(long)]
    max_cutoff_hours: Option<i64>,
    #[arg(long)]
    yes: bool,
    #[arg(long)]
    dry_run: bool,
}

#[derive(Debug)]
struct AppError {
    code: i32,
    message: String,
}

impl AppError {
    fn new(code: i32, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SlotId {
    config_id: String,
    day: String,
    party_size: u8,
    venue_id: i64,
    start: Option<String>,
    slot_type: Option<String>,
}

struct ResyClient {
    http: reqwest::Client,
}

impl ResyClient {
    fn new(api_key: &str, auth_token: &str) -> Result<Self, AppError> {
        let mut headers = HeaderMap::new();
        let auth = format!("ResyAPI api_key=\"{}\"", api_key);
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&auth)
                .map_err(|_| AppError::new(5, "invalid API key for header"))?,
        );
        headers.insert(
            "x-resy-universal-auth",
            HeaderValue::from_str(auth_token)
                .map_err(|_| AppError::new(5, "invalid auth token for header"))?,
        );
        headers.insert(
            "x-resy-auth-token",
            HeaderValue::from_str(auth_token)
                .map_err(|_| AppError::new(5, "invalid auth token for header"))?,
        );

        let http = reqwest::Client::builder()
            .default_headers(headers)
            .user_agent("ressy-cli/0.1.0")
            .build()
            .map_err(|e| AppError::new(4, format!("failed to build HTTP client: {e}")))?;

        Ok(Self { http })
    }

    async fn search(&self, query: &str, limit: u32, lat: f64, lng: f64) -> Result<Value, AppError> {
        let body = json!({
            "query": query,
            "per_page": limit,
            "types": ["venue"],
            "geo": { "latitude": lat, "longitude": lng }
        });
        self.post_json("https://api.resy.com/3/venuesearch/search", body)
            .await
    }

    async fn find(
        &self,
        venue_id: i64,
        day: &str,
        party_size: u8,
        lat: f64,
        lng: f64,
    ) -> Result<Value, AppError> {
        let response = self
            .http
            .get("https://api.resy.com/4/find")
            .query(&[
                ("venue_id", venue_id.to_string()),
                ("day", day.to_string()),
                ("party_size", party_size.to_string()),
                ("lat", lat.to_string()),
                ("long", lng.to_string()),
            ])
            .send()
            .await
            .map_err(|e| AppError::new(4, format!("find request failed: {e}")))?;

        read_json_response(response).await
    }

    async fn details(&self, slot: &SlotId) -> Result<Value, AppError> {
        let body = json!({
            "commit": 1,
            "config_id": slot.config_id,
            "day": slot.day,
            "party_size": slot.party_size,
        });
        self.post_json("https://api.resy.com/3/details", body).await
    }

    async fn book(&self, book_token: &str, payment_method_id: Option<i64>) -> Result<Value, AppError> {
        let mut form = vec![("book_token", book_token.to_string())];
        if let Some(id) = payment_method_id {
            let payment = json!({ "id": id }).to_string();
            form.push(("struct_payment_method", payment));
        }

        let response = self
            .http
            .post("https://api.resy.com/3/book")
            .header(CONTENT_TYPE, "application/x-www-form-urlencoded")
            .form(&form)
            .send()
            .await
            .map_err(|e| AppError::new(4, format!("book request failed: {e}")))?;

        read_json_response(response).await
    }

    async fn post_json(&self, url: &str, body: Value) -> Result<Value, AppError> {
        let response = self
            .http
            .post(url)
            .header(CONTENT_TYPE, "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| AppError::new(4, format!("request failed: {e}")))?;

        read_json_response(response).await
    }
}

#[tokio::main]
async fn main() -> ExitCode {
    let cli = Cli::parse();
    let result = run(cli).await;
    match result {
        Ok(output) => {
            if let Err(e) = print_json(&output) {
                let err = json!({"ok": false, "code": 4, "error": format!("failed to print JSON: {}", e.message)});
                let _ = print_json(&err);
                ExitCode::from(4)
            } else {
                ExitCode::SUCCESS
            }
        }
        Err(e) => {
            let err = json!({"ok": false, "code": e.code, "error": e.message});
            let _ = print_json(&err);
            ExitCode::from(e.code as u8)
        }
    }
}

async fn run(cli: Cli) -> Result<Value, AppError> {
    let auth_token = resolve_auth_token(cli.auth_token)?;
    let api_key = resolve_api_key(cli.api_key);
    let payment_method_id = resolve_payment_method_id(cli.payment_method_id);

    let client = ResyClient::new(&api_key, &auth_token)?;

    match cli.command {
        Command::Search(args) => run_search(&client, args).await,
        Command::Availability(args) => run_availability(&client, args).await,
        Command::Quote(args) => run_quote(&client, args).await,
        Command::Book(args) => run_book(&client, args, payment_method_id).await,
    }
}

async fn run_search(client: &ResyClient, args: SearchArgs) -> Result<Value, AppError> {
    let raw = client.search(&args.query, args.limit, args.lat, args.lng).await?;
    let hits = raw
        .get("search")
        .and_then(|v| v.get("hits"))
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    let venues: Vec<Value> = hits
        .iter()
        .map(|hit| {
            json!({
                "id": hit.pointer("/id/resy").and_then(Value::as_i64),
                "name": hit.get("name").and_then(Value::as_str),
                "locality": hit.get("locality").and_then(Value::as_str),
                "neighborhood": hit.get("neighborhood").and_then(Value::as_str),
                "cuisine": hit.get("cuisine").cloned(),
                "rating": hit.pointer("/rating/average").and_then(Value::as_f64),
                "raw": hit,
            })
        })
        .collect();

    Ok(json!({
        "ok": true,
        "query": args.query,
        "count": venues.len(),
        "venues": venues,
    }))
}

async fn run_availability(client: &ResyClient, args: AvailabilityArgs) -> Result<Value, AppError> {
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

async fn run_quote(client: &ResyClient, args: QuoteArgs) -> Result<Value, AppError> {
    let slot = decode_slot_id(&args.slot_id)?;
    let details = client.details(&slot).await?;
    let summary = quote_summary(&details);

    Ok(json!({
        "ok": true,
        "slot_id": args.slot_id,
        "slot": slot,
        "quote": summary,
        "raw": details,
    }))
}

async fn run_book(
    client: &ResyClient,
    args: BookArgs,
    cli_payment_method_id: Option<i64>,
) -> Result<Value, AppError> {
    let slot = decode_slot_id(&args.slot_id)?;
    let details = client.details(&slot).await?;
    let summary = quote_summary(&details);

    let fee_amount = summary
        .get("fee_amount")
        .and_then(Value::as_f64)
        .unwrap_or(0.0);

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
            format!(
                "booking blocked by policy: fee {fee_amount} exceeds --max-fee {max_fee}"
            ),
        ));
    }

    if let Some(max_cutoff_hours) = args.max_cutoff_hours {
        let fee_cutoff = summary.get("fee_cutoff").and_then(Value::as_str);
        let now = Utc::now();
        let hours_until_cutoff = fee_cutoff
            .and_then(parse_rfc3339_utc)
            .map(|ts| (ts - now).num_hours());

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

    let book_token = details
        .pointer("/book_token/value")
        .and_then(Value::as_str)
        .ok_or_else(|| AppError::new(4, "details response missing book_token.value"))?;

    let payment_method_id = cli_payment_method_id.or_else(|| {
        details
            .pointer("/user/payment_methods")
            .and_then(Value::as_array)
            .and_then(|arr| arr.first())
            .and_then(|v| v.get("id"))
            .and_then(Value::as_i64)
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

    let booking_result = client.book(book_token, payment_method_id).await?;

    Ok(json!({
        "ok": true,
        "booked": true,
        "slot": slot,
        "quote": summary,
        "payment_method_id": payment_method_id,
        "result": booking_result,
    }))
}

async fn read_json_response(response: reqwest::Response) -> Result<Value, AppError> {
    let status = response.status();
    let body = response
        .text()
        .await
        .map_err(|e| AppError::new(4, format!("failed reading response body: {e}")))?;

    let parsed = serde_json::from_str::<Value>(&body)
        .unwrap_or_else(|_| json!({"raw": body, "parse_error": true}));

    if !status.is_success() {
        return Err(AppError::new(
            4,
            format!("api error {}: {}", status.as_u16(), parsed),
        ));
    }

    Ok(parsed)
}

fn resolve_auth_token(flag: Option<String>) -> Result<String, AppError> {
    if let Some(v) = flag
        && !v.trim().is_empty()
    {
        return Ok(v.trim().to_string());
    }
    for key in ["RESSY_AUTH_TOKEN", "RESY_AUTH_TOKEN", "X_RESY_UNIVERSAL_AUTH"] {
        if let Ok(v) = env::var(key)
            && !v.trim().is_empty()
        {
            return Ok(v.trim().to_string());
        }
    }

    let default_path = Path::new("secrets/resy_auth_token");
    if default_path.exists() {
        let token = fs::read_to_string(default_path)
            .map_err(|e| AppError::new(4, format!("failed reading secrets/resy_auth_token: {e}")))?;
        if !token.trim().is_empty() {
            return Ok(token.trim().to_string());
        }
    }

    Err(AppError::new(
        5,
        "missing auth token; set --auth-token, RESSY_AUTH_TOKEN, or secrets/resy_auth_token",
    ))
}

fn resolve_api_key(flag: Option<String>) -> String {
    if let Some(v) = flag
        && !v.trim().is_empty()
    {
        return v.trim().to_string();
    }
    for key in ["RESSY_API_KEY", "RESY_API_KEY"] {
        if let Ok(v) = env::var(key)
            && !v.trim().is_empty()
        {
            return v.trim().to_string();
        }
    }
    DEFAULT_API_KEY.to_string()
}

fn resolve_payment_method_id(flag: Option<i64>) -> Option<i64> {
    if flag.is_some() {
        return flag;
    }
    for key in ["RESSY_PAYMENT_METHOD_ID", "RESY_PAYMENT_METHOD_ID"] {
        if let Ok(v) = env::var(key)
            && let Ok(parsed) = v.trim().parse::<i64>()
        {
            return Some(parsed);
        }
    }

    let default_path = Path::new("secrets/resy_payment_method_id");
    if default_path.exists()
        && let Ok(v) = fs::read_to_string(default_path)
        && let Ok(parsed) = v.trim().parse::<i64>()
    {
        return Some(parsed);
    }
    None
}

fn validate_date(date: &str) -> Result<(), AppError> {
    NaiveDate::parse_from_str(date, "%Y-%m-%d")
        .map(|_| ())
        .map_err(|_| AppError::new(5, format!("invalid date format: {date} (expected YYYY-MM-DD)")))
}

fn dates_in_month(month: &str) -> Result<Vec<NaiveDate>, AppError> {
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

fn start_to_hhmm(start: &str) -> Option<&str> {
    start.split_whitespace().nth(1).and_then(|t| t.get(0..5))
}

fn parse_rfc3339_utc(value: &str) -> Option<chrono::DateTime<Utc>> {
    chrono::DateTime::parse_from_rfc3339(value)
        .ok()
        .map(|dt| dt.with_timezone(&Utc))
}

fn extract_slots(find: &Value, venue_id: i64, day: &str, party_size: u8) -> Vec<Value> {
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

fn quote_summary(details: &Value) -> Value {
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
        "fee_display": details.pointer("/cancellation/fee/display/amount").and_then(Value::as_str),
        "fee_cutoff": details.pointer("/cancellation/fee/date_cut_off").and_then(Value::as_str),
        "refund_cutoff": details.pointer("/cancellation/refund/date_cut_off").and_then(Value::as_str),
        "payment_type": details.pointer("/payment/config/type").and_then(Value::as_str),
        "policy_text": policy_text,
        "has_book_token": details.pointer("/book_token/value").and_then(Value::as_str).is_some(),
    })
}

fn encode_slot_id(payload: &SlotId) -> String {
    let raw = serde_json::to_vec(payload).unwrap_or_else(|_| b"{}".to_vec());
    URL_SAFE_NO_PAD.encode(raw)
}

fn decode_slot_id(slot_id: &str) -> Result<SlotId, AppError> {
    let raw = URL_SAFE_NO_PAD
        .decode(slot_id.as_bytes())
        .map_err(|_| AppError::new(5, "invalid slot_id encoding"))?;
    serde_json::from_slice::<SlotId>(&raw)
        .map_err(|_| AppError::new(5, "invalid slot_id payload"))
}

fn print_json(value: &Value) -> Result<(), AppError> {
    let output = serde_json::to_string_pretty(value)
        .map_err(|e| AppError::new(4, format!("failed to serialize output JSON: {e}")))?;
    println!("{output}");
    Ok(())
}
