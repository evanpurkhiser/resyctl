use strum::IntoStaticStr;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    InvalidInput(#[from] InputError),

    #[error("missing auth token; run `resyctl auth login` first")]
    MissingAuthToken,

    #[error(transparent)]
    BookingPolicy(#[from] BookingPolicyError),

    #[error(transparent)]
    Api(#[from] ApiError),

    #[error(transparent)]
    Io(#[from] IoError),

    #[error("{0}")]
    Internal(String),
}

impl Error {
    /// Stable snake_case identifier for JSON consumers — drilled down to
    /// the leaf variant on the wrapped sub-enum so consumers can match on
    /// the exact failure (e.g. "cancellation_fee_present") instead of the
    /// category ("booking_policy").
    pub fn kind(&self) -> &'static str {
        match self {
            Error::InvalidInput(e) => e.into(),
            Error::MissingAuthToken => "missing_auth_token",
            Error::BookingPolicy(e) => e.into(),
            Error::Api(e) => e.into(),
            Error::Io(e) => e.into(),
            Error::Internal(_) => "internal",
        }
    }

    /// Process exit code grouped by error category.
    pub fn exit_code(&self) -> u8 {
        match self {
            Error::InvalidInput(_) | Error::MissingAuthToken => 5,
            Error::BookingPolicy(_) => 3,
            Error::Api(_) | Error::Io(_) | Error::Internal(_) => 4,
        }
    }
}

/// Input validation failures: bad CLI args, malformed user input, missing
/// required confirmations. Variants own their messages so command sites
/// just construct the variant.
#[derive(Debug, Error, IntoStaticStr)]
#[strum(serialize_all = "snake_case")]
pub enum InputError {
    #[error("pass only one of --month or --date")]
    AvailabilityCannotMixMonthAndDate,

    #[error("you must pass either --month or --date")]
    AvailabilityRequiresMonthOrDate,

    #[error("--month currently requires --days to return day-level availability")]
    AvailabilityMonthRequiresDays,

    #[error("--date is required for date availability mode")]
    AvailabilityDateModeRequiresDate,

    #[error("booking requires --yes (or use --dry-run)")]
    BookRequiresYes,

    #[error("cancel requires --yes (or use --dry-run)")]
    CancelRequiresYes,

    #[error("input cannot be empty")]
    EmptyPromptInput,

    #[error("password cannot be empty")]
    EmptyPassword,

    #[error("invalid month format: {value} (expected YYYY-MM)")]
    InvalidMonth { value: String },

    #[error("invalid date format: {value} (expected YYYY-MM-DD)")]
    InvalidDate { value: String },

    #[error("invalid time format: {value} (expected HH:MM)")]
    InvalidTime { value: String },

    #[error("invalid slot_id encoding")]
    InvalidSlotIdEncoding,

    #[error("invalid slot_id payload")]
    InvalidSlotIdPayload,
}

#[derive(Debug, Error, IntoStaticStr)]
#[strum(serialize_all = "snake_case")]
pub enum BookingPolicyError {
    #[error("booking blocked by policy: cancellation fee present; pass --allow-cancellation-fee to override")]
    CancellationFeePresent,

    #[error("booking blocked by policy: cancellation fee {actual} exceeds --max-cancellation-fee {max}")]
    CancellationFeeExceeded { actual: f64, max: f64 },

    #[error("booking blocked by policy: cutoff {hours}h is less than --max-cutoff-hours {max}")]
    CutoffTooClose { hours: i64, max: i64 },

    #[error("booking blocked by policy: cutoff unavailable for --max-cutoff-hours check")]
    CutoffUnavailable,
}

/// Errors raised when interacting with the Resy API: network failures,
/// non-success HTTP responses, and unexpected response shapes.
#[derive(Debug, Error, IntoStaticStr)]
#[strum(serialize_all = "snake_case")]
pub enum ApiError {
    #[error("request failed: {0}")]
    Request(#[source] reqwest::Error),

    #[error("auth request failed: {0}")]
    AuthRequest(#[source] reqwest::Error),

    #[error("api error {status}: {body}")]
    ErrorStatusCode {
        status: u16,
        body: serde_json::Value,
    },

    #[error("failed reading response body: {0}")]
    ReadBody(#[source] reqwest::Error),

    #[error("failed to deserialize API response body as JSON: {0}")]
    ParseBodyJson(#[source] serde_json::Error),

    #[error("failed to deserialize API response: {0}")]
    DeserializeResponse(#[source] serde_json::Error),

    #[error("auth response missing token")]
    AuthResponseMissingToken,

    #[error("re-auth response missing token")]
    ReauthResponseMissingToken,

    #[error("details response missing book_token.value")]
    MissingBookToken,

    #[error("failed to build HTTP client: {0}")]
    BuildClient(#[source] reqwest::Error),
}

/// Filesystem and stdin/stdout errors. Variants name the operation; the
/// path (when relevant) is part of the variant data.
#[derive(Debug, Error, IntoStaticStr)]
#[strum(serialize_all = "snake_case")]
pub enum IoError {
    #[error("could not resolve state directory")]
    StateDirUnresolved,

    #[error("failed reading {path}: {source}")]
    ReadFile {
        path: String,
        #[source]
        source: std::io::Error,
    },

    #[error("failed parsing {path}: {source}")]
    ParseStateFile {
        path: String,
        #[source]
        source: serde_json::Error,
    },

    #[error("failed creating {path}: {source}")]
    CreateDir {
        path: String,
        #[source]
        source: std::io::Error,
    },

    #[error("failed opening {path}: {source}")]
    OpenFile {
        path: String,
        #[source]
        source: std::io::Error,
    },

    #[error("failed writing {path}: {source}")]
    WriteFile {
        path: String,
        #[source]
        source: std::io::Error,
    },

    #[error("failed serializing state: {0}")]
    SerializeState(#[source] serde_json::Error),

    #[error("failed reading password: {0}")]
    PasswordPrompt(#[source] std::io::Error),

    #[error("failed writing prompt: {0}")]
    PromptWrite(#[source] std::io::Error),

    #[error("failed reading input: {0}")]
    PromptRead(#[source] std::io::Error),
}
