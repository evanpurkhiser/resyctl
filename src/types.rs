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

    pub fn parse_slot_start(value: &str) -> Option<Self> {
        NaiveDateTime::parse_from_str(value, "%Y-%m-%d %H:%M:%S")
            .map(|dt| Self(dt.time()))
            .or_else(|_| chrono::DateTime::parse_from_rfc3339(value).map(|dt| Self(dt.time())))
            .or_else(|_| NaiveTime::parse_from_str(value, "%H:%M").map(Self))
            .ok()
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
    pub config_id: String,
    pub day: String,
    pub party_size: u8,
    pub venue_id: i64,
    pub start: Option<String>,
    pub slot_type: Option<String>,
}

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

    #[test]
    fn parses_slot_start_time_from_multiple_formats() {
        let from_find = TimeArg::parse_slot_start("2026-04-26 18:30:00").expect("find style");
        let from_rfc3339 =
            TimeArg::parse_slot_start("2026-04-26T18:30:00-04:00").expect("rfc3339 style");

        assert_eq!(from_find.to_string(), "18:30");
        assert_eq!(from_rfc3339.to_string(), "18:30");
    }
}
