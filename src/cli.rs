use clap::{Args, Parser, Subcommand};

use crate::types::{DateArg, MonthArg, ResyToken, TimeArg};

pub const DEFAULT_LAT: f64 = 40.7128;
pub const DEFAULT_LNG: f64 = -74.0060;

#[derive(Parser, Debug)]
#[command(
    name = "resyctl",
    about = "A Resy CLI focused on automation and agent use"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Search venues by name text.
    Search(SearchArgs),
    /// Check venue availability for a specific date or month.
    Availability(AvailabilityArgs),
    /// Quote a slot to inspect cancellation and payment policy details.
    Quote(QuoteArgs),
    /// Attempt to book a slot, with policy guardrails.
    Book(BookArgs),
    /// List reservations from the account.
    Reservations(ReservationsArgs),
    /// List available payment methods for booking.
    PaymentMethods(PaymentMethodsArgs),
    /// Cancel a reservation by resy token.
    Cancel(CancelArgs),
    /// Authenticate and inspect authentication status.
    Auth(AuthArgs),
    /// Show effective configuration and where values are loaded from.
    Config(ConfigArgs),
}

#[derive(Args, Debug)]
pub struct SearchArgs {
    /// Venue name or query text to search for.
    pub query: String,
    /// Maximum number of search hits to request.
    #[arg(long, default_value_t = 10)]
    pub limit: u32,
    /// Latitude used for geo-biased search ranking.
    #[arg(long, default_value_t = DEFAULT_LAT)]
    pub lat: f64,
    /// Longitude used for geo-biased search ranking.
    #[arg(long, default_value_t = DEFAULT_LNG)]
    pub lng: f64,
}

#[derive(Args, Debug)]
pub struct AvailabilityArgs {
    /// Resy venue id.
    pub restaurant_id: i64,
    /// Month to scan in YYYY-MM format.
    #[arg(long, value_parser = MonthArg::parse)]
    pub month: Option<MonthArg>,
    /// In month mode, return only days that have at least one slot.
    #[arg(long)]
    pub days: bool,
    /// Specific date to query in YYYY-MM-DD format.
    #[arg(long, value_parser = DateArg::parse)]
    pub date: Option<DateArg>,
    /// Party size used in availability lookup.
    #[arg(long, default_value_t = 2)]
    pub party_size: u8,
    /// Filter slots by seating/type text (e.g. bar, patio, table).
    #[arg(long)]
    pub seating: Option<String>,
    /// Filter to slots at or after this local time (HH:MM).
    #[arg(long, value_parser = TimeArg::parse)]
    pub time_after: Option<TimeArg>,
    /// Filter to slots at or before this local time (HH:MM).
    #[arg(long, value_parser = TimeArg::parse)]
    pub time_before: Option<TimeArg>,
    /// Latitude used for availability requests.
    #[arg(long, default_value_t = DEFAULT_LAT)]
    pub lat: f64,
    /// Longitude used for availability requests.
    #[arg(long, default_value_t = DEFAULT_LNG)]
    pub lng: f64,
}

#[derive(Args, Debug)]
pub struct QuoteArgs {
    /// Opaque slot id returned by the availability command.
    pub slot_id: String,
}

#[derive(Args, Debug)]
pub struct BookArgs {
    /// Opaque slot id returned by the availability command.
    pub slot_id: String,
    /// Allow booking even if a cancellation fee applies.
    #[arg(long)]
    pub allow_fee: bool,
    /// Maximum allowed cancellation fee in account currency.
    #[arg(long)]
    pub max_fee: Option<f64>,
    /// Override payment method id used for this booking request.
    #[arg(long)]
    pub payment_method_id: Option<i64>,
    /// Minimum hours before fee cutoff required to proceed.
    #[arg(long)]
    pub max_cutoff_hours: Option<i64>,
    /// Confirm live booking (required unless using dry-run).
    #[arg(long)]
    pub yes: bool,
    /// Simulate booking flow without creating a reservation.
    #[arg(long)]
    pub dry_run: bool,
}

#[derive(Args, Debug)]
pub struct ReservationsArgs {
    /// Optional reservation token for targeted lookup.
    pub resy_token: Option<ResyToken>,
    /// Return only upcoming reservations (default behavior).
    #[arg(long, default_value_t = true)]
    pub upcoming: bool,
    /// Return all reservations without upcoming-only filtering.
    #[arg(long)]
    pub all: bool,
    /// Max reservations to request from the API.
    #[arg(long)]
    pub limit: Option<u32>,
    /// Pagination offset for reservation listing.
    #[arg(long)]
    pub offset: Option<u32>,
}

#[derive(Args, Debug)]
pub struct CancelArgs {
    /// Reservation token to cancel.
    pub resy_token: ResyToken,
    /// Confirm live cancellation (required unless using dry-run).
    #[arg(long)]
    pub yes: bool,
    /// Simulate cancellation flow without canceling reservation.
    #[arg(long)]
    pub dry_run: bool,
}

#[derive(Args, Debug)]
pub struct PaymentMethodsArgs {}

#[derive(Args, Debug)]
pub struct ConfigArgs {}

#[derive(Args, Debug)]
pub struct AuthArgs {
    #[command(subcommand)]
    pub command: AuthCommand,
}

#[derive(Subcommand, Debug)]
pub enum AuthCommand {
    /// Validate current auth token and return account summary.
    Status,
    /// Log in with email/password and persist credentials to state.
    Login(LoginArgs),
}

#[derive(Args, Debug)]
pub struct LoginArgs {
    /// Email address for Resy login. Prompted if omitted.
    #[arg(long)]
    pub email: Option<String>,
    /// Password for Resy login. Prompted (hidden) if omitted.
    #[arg(long)]
    pub password: Option<String>,
}
