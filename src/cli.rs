use clap::{Args, Parser, Subcommand};
use clap::ArgAction;

pub const DEFAULT_LAT: f64 = 40.7128;
pub const DEFAULT_LNG: f64 = -74.0060;

#[derive(Parser, Debug)]
#[command(name = "ressy", about = "Resy CLI (JSON output only)")]
pub struct Cli {
    #[arg(long, global = true)]
    pub auth_token: Option<String>,
    #[arg(long, global = true)]
    pub api_key: Option<String>,
    #[arg(long, global = true)]
    pub payment_method_id: Option<i64>,
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    Search(SearchArgs),
    Availability(AvailabilityArgs),
    Quote(QuoteArgs),
    Book(BookArgs),
    Reservations(ReservationsArgs),
    Cancel(CancelArgs),
    Auth(AuthArgs),
    Config(ConfigArgs),
}

#[derive(Args, Debug)]
pub struct SearchArgs {
    pub query: String,
    #[arg(long, default_value_t = 10)]
    pub limit: u32,
    #[arg(long, default_value_t = DEFAULT_LAT)]
    pub lat: f64,
    #[arg(long, default_value_t = DEFAULT_LNG)]
    pub lng: f64,
}

#[derive(Args, Debug)]
pub struct AvailabilityArgs {
    pub restaurant_id: i64,
    #[arg(long)]
    pub month: Option<String>,
    #[arg(long)]
    pub days: bool,
    #[arg(long)]
    pub date: Option<String>,
    #[arg(long, default_value_t = 2)]
    pub party_size: u8,
    #[arg(long)]
    pub seating: Option<String>,
    #[arg(long)]
    pub time_after: Option<String>,
    #[arg(long)]
    pub time_before: Option<String>,
    #[arg(long, default_value_t = DEFAULT_LAT)]
    pub lat: f64,
    #[arg(long, default_value_t = DEFAULT_LNG)]
    pub lng: f64,
}

#[derive(Args, Debug)]
pub struct QuoteArgs {
    pub slot_id: String,
}

#[derive(Args, Debug)]
pub struct BookArgs {
    pub slot_id: String,
    #[arg(long)]
    pub allow_fee: bool,
    #[arg(long)]
    pub max_fee: Option<f64>,
    #[arg(long)]
    pub max_cutoff_hours: Option<i64>,
    #[arg(long)]
    pub yes: bool,
    #[arg(long)]
    pub dry_run: bool,
}

#[derive(Args, Debug)]
pub struct ReservationsArgs {
    #[arg(long)]
    pub resy_token: String,
}

#[derive(Args, Debug)]
pub struct CancelArgs {
    #[arg(long)]
    pub resy_token: String,
    #[arg(long, action = ArgAction::Set, default_value_t = true)]
    pub refresh_token: bool,
    #[arg(long)]
    pub yes: bool,
    #[arg(long)]
    pub dry_run: bool,
}

#[derive(Args, Debug)]
pub struct ConfigArgs {}

#[derive(Args, Debug)]
pub struct AuthArgs {
    #[command(subcommand)]
    pub command: AuthCommand,
}

#[derive(Subcommand, Debug)]
pub enum AuthCommand {
    Status,
    Login(LoginArgs),
}

#[derive(Args, Debug)]
pub struct LoginArgs {
    #[arg(long)]
    pub email: String,
    #[arg(long)]
    pub password: Option<String>,
    #[arg(long)]
    pub password_file: Option<String>,
    #[arg(long)]
    pub write_secrets: bool,
}
