//! The expiry of an account: never, disabled, or a date.

use crate::error::{Result, UgosError};
use crate::types::users::{DISABLED, EXPIRES_NEVER};

/// Seconds in a day.
const DAY: i64 = 86_400;

/// The latest `expire_time` the web UI's date picker allows, in seconds.
const LATEST: i64 = 4_133_894_400;

/// An expiry as UGOS holds it: the time in seconds, and the date as text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Expiry {
    /// Unix time in seconds, or `-1` for never and `1` for disabled.
    pub time: i64,
    /// The date as `YYYY-MM-DD`, empty unless an expiry date is set.
    pub date: String,
}

impl Expiry {
    /// An account that is on and never expires.
    pub const NEVER: Self = Self {
        time: EXPIRES_NEVER,
        date: String::new(),
    };

    /// An account that is switched off.
    pub const DISABLED: Self = Self {
        time: DISABLED,
        date: String::new(),
    };

    /// Whether this is the switched-off state.
    #[must_use]
    pub const fn is_disabled(&self) -> bool {
        self.time == DISABLED
    }
}

/// `--expires YYYY-MM-DD`, as the web UI's date picker sends it: the time is
/// midnight UTC of that day, and the date goes along as text.
///
/// `today` is the current day, counted from 1970-01-01. A date before it is
/// refused, as the picker refuses it.
///
/// # Errors
///
/// Returns [`UgosError::Refused`] for text that is not a date, a date in the
/// past, or one beyond what UGOS allows.
pub fn parse_date(text: &str, today: i64) -> Result<Expiry> {
    let bad = || UgosError::Refused(format!("'{text}' is not a date; write YYYY-MM-DD"));
    let mut parts = text.trim().splitn(3, '-');
    let mut next = |width: usize| -> Result<i64> {
        let part = parts.next().ok_or_else(bad)?;
        if part.len() != width || !part.bytes().all(|b| b.is_ascii_digit()) {
            return Err(bad());
        }
        part.parse().map_err(|_| bad())
    };
    let (year, month, day) = (next(4)?, next(2)?, next(2)?);
    if !(1..=12).contains(&month) || day < 1 || day > days_in_month(year, month) {
        return Err(bad());
    }
    let days = days_from_civil(year, month, day);
    if days < today {
        return Err(UgosError::Refused(format!("{text} is in the past")));
    }
    let time = days * DAY;
    if time > LATEST {
        return Err(UgosError::Refused(format!(
            "{text} is further out than UGOS allows"
        )));
    }
    Ok(Expiry {
        time,
        date: format!("{year:04}-{month:02}-{day:02}"),
    })
}

/// The current day, counted from 1970-01-01.
#[must_use]
pub fn today() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |since| {
            i64::try_from(since.as_secs() / 86_400).unwrap_or(0)
        })
}

const fn is_leap(year: i64) -> bool {
    year % 4 == 0 && (year % 100 != 0 || year % 400 == 0)
}

const fn days_in_month(year: i64, month: i64) -> i64 {
    match month {
        2 if is_leap(year) => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

/// Days from 1970-01-01 to the given date (Howard Hinnant's algorithm).
const fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let year = if month <= 2 { year - 1 } else { year };
    let era = year.div_euclid(400);
    let year_of_era = year - era * 400;
    let shifted = if month > 2 { month - 3 } else { month + 9 };
    let day_of_year = (153 * shifted + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn a_date_is_midnight_utc_and_keeps_its_text() {
        let expiry = parse_date("2027-01-31", 0).unwrap();
        assert_eq!(expiry.time, 1_801_353_600);
        assert_eq!(expiry.date, "2027-01-31");
        assert_eq!(parse_date("1970-01-01", 0).unwrap().time, 0);
        assert_eq!(parse_date("2000-03-01", 0).unwrap().time, 951_868_800);
    }

    #[test]
    fn leap_days_are_counted() {
        assert!(parse_date("2028-02-29", 0).is_ok());
        assert!(parse_date("2027-02-29", 0).is_err());
        assert!(parse_date("2100-02-29", 0).is_err());
        assert!(parse_date("2000-02-29", 0).is_ok());
    }

    #[test]
    fn things_that_are_not_dates_are_refused() {
        for bad in [
            "",
            "2027",
            "2027-1-31",
            "27-01-31",
            "2027-13-01",
            "2027-00-10",
            "2027-01-32",
            "2027/01/31",
            "tomorrow",
            "2027-01-31-01",
        ] {
            assert!(parse_date(bad, 0).is_err(), "{bad:?}");
        }
    }

    #[test]
    fn the_past_and_the_far_future_are_refused() {
        let today = days_from_civil(2026, 11, 7);
        assert!(parse_date("2026-11-06", today).is_err());
        assert!(parse_date("2026-11-07", today).is_ok());
        assert!(parse_date("2300-01-01", today).is_err());
    }

    #[test]
    fn today_is_after_the_start_of_this_work() {
        assert!(today() > days_from_civil(2026, 11, 7) - 1);
    }
}
