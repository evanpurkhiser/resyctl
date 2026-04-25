use std::fmt;
use std::str::FromStr;

use chrono::{Datelike, NaiveDate, NaiveDateTime, NaiveTime};
use serde::{Deserialize, Serialize};

use crate::error::AppError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MonthArg {
    first_day: NaiveDate,
}

impl MonthArg {
    pub fn parse(value: &str) -> Result<Self, AppError> {
        Self::from_str(value)
    }

    pub fn days(self) -> impl Iterator<Item = NaiveDate> {
        let first = self.first_day;
        std::iter::successors(Some(first), move |day| {
            let next = day.succ_opt()?;
            (next.month() == first.month()).then_some(next)
        })
    }
}

impl fmt::Display for MonthArg {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.first_day.format("%Y-%m"))
    }
}

impl FromStr for MonthArg {
    type Err = AppError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let first_day =
            NaiveDate::parse_from_str(&format!("{value}-01"), "%Y-%m-%d").map_err(|_| {
                AppError::new(
                    5,
                    format!("invalid month format: {value} (expected YYYY-MM)"),
                )
            })?;
        Ok(Self { first_day })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DateArg(pub NaiveDate);

impl DateArg {
    pub fn parse(value: &str) -> Result<Self, AppError> {
        Self::from_str(value)
    }
}

impl fmt::Display for DateArg {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0.format("%Y-%m-%d"))
    }
}

impl FromStr for DateArg {
    type Err = AppError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        NaiveDate::parse_from_str(value, "%Y-%m-%d")
            .map(Self)
            .map_err(|_| {
                AppError::new(
                    5,
                    format!("invalid date format: {value} (expected YYYY-MM-DD)"),
                )
            })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct TimeArg(pub NaiveTime);

impl TimeArg {
    pub fn parse(value: &str) -> Result<Self, AppError> {
        Self::from_str(value)
    }
}

impl fmt::Display for TimeArg {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0.format("%H:%M"))
    }
}

impl FromStr for TimeArg {
    type Err = AppError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        NaiveTime::parse_from_str(value, "%H:%M")
            .map(Self)
            .map_err(|_| AppError::new(5, format!("invalid time format: {value} (expected HH:MM)")))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlotId {
    pub config_id: ConfigId,
    pub day: NaiveDate,
    pub party_size: u8,
    pub venue_id: i64,
    pub start: Option<NaiveDateTime>,
    pub slot_type: Option<String>,
}

macro_rules! string_newtype {
    ($name:ident) => {
        #[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(pub String);

        impl $name {
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl AsRef<str> for $name {
            fn as_ref(&self) -> &str {
                &self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&self.0)
            }
        }

        impl From<String> for $name {
            fn from(value: String) -> Self {
                Self(value)
            }
        }

        impl FromStr for $name {
            type Err = std::convert::Infallible;

            fn from_str(value: &str) -> Result<Self, Self::Err> {
                Ok(Self(value.to_string()))
            }
        }
    };
}

string_newtype!(ResyToken);
string_newtype!(BookToken);
string_newtype!(ConfigId);

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;

    use super::{DateArg, MonthArg, TimeArg};

    #[test]
    fn parses_month_and_iterates_days() {
        let month = "2026-02".parse::<MonthArg>().expect("valid month");
        let days: Vec<_> = month.days().collect();

        assert_eq!(
            days.first(),
            Some(&NaiveDate::from_ymd_opt(2026, 2, 1).unwrap())
        );
        assert_eq!(
            days.last(),
            Some(&NaiveDate::from_ymd_opt(2026, 2, 28).unwrap())
        );
        assert_eq!(month.to_string(), "2026-02");
    }

    #[test]
    fn parses_date_and_time_args() {
        let date = "2026-04-26".parse::<DateArg>().expect("valid date");
        let time = "18:30".parse::<TimeArg>().expect("valid time");

        assert_eq!(date.to_string(), "2026-04-26");
        assert_eq!(time.to_string(), "18:30");
    }

}
