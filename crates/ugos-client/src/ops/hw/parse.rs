//! Reading the short forms people type: `20m`, `weekdays`, a UTC time.

use super::power::Sleep;
use super::ups::Wait;
use crate::error::{Result, UgosError};

const fn refused(message: String) -> UgosError {
    UgosError::Refused(message)
}

/// A number and its unit letter, like `20m` or `2h`.
fn number_and_unit(text: &str) -> Option<(i64, char)> {
    let unit = text.chars().last()?;
    let digits = &text[..text.len() - unit.len_utf8()];
    digits.parse().ok().map(|n| (n, unit))
}

/// `never`, `10m`, `1h` and so on.
///
/// # Errors
///
/// Returns [`UgosError::Refused`] for another form.
pub fn sleep(text: &str) -> Result<Sleep> {
    let bad = || refused(format!("'{text}' is never, or like 20m or 2h"));
    if text == "never" {
        return Ok(Sleep::Never);
    }
    match number_and_unit(text) {
        Some((n, 'm')) => Ok(Sleep::Minutes(n)),
        Some((n, 'h')) => Ok(Sleep::Hours(n)),
        _ => Err(bad()),
    }
}

/// `battery-low`, `now`, `5m` or `2h`.
///
/// # Errors
///
/// Returns [`UgosError::Refused`] for another form.
pub fn wait(text: &str) -> Result<Wait> {
    match text {
        "battery-low" => Ok(Wait::BatteryLow),
        "now" => Ok(Wait::Immediately),
        _ => match number_and_unit(text) {
            Some((n, 'm')) => Ok(Wait::Custom(n, 1)),
            Some((n, 'h')) => Ok(Wait::Custom(n, 2)),
            _ => Err(refused(format!(
                "'{text}' is battery-low, now, or like 5m or 2h"
            ))),
        },
    }
}

/// `1,2,3`, `daily`, `weekdays` or `weekend`.
///
/// # Errors
///
/// Returns [`UgosError::Refused`] for something that is no weekday list.
pub fn days(text: &str) -> Result<Vec<i64>> {
    match text {
        "daily" => Ok((1..=7).collect()),
        "weekdays" => Ok((1..=5).collect()),
        "weekend" => Ok(vec![6, 7]),
        _ => text
            .split(',')
            .map(|d| {
                d.trim()
                    .parse()
                    .map_err(|_| refused(format!("'{d}' is not a weekday number")))
            })
            .collect(),
    }
}

/// Comma separated numbers.
///
/// # Errors
///
/// Returns [`UgosError::Refused`] for a part that is no number.
pub fn numbers(text: &str) -> Result<Vec<i64>> {
    text.split(',')
        .map(|part| {
            part.trim()
                .parse()
                .map_err(|_| refused(format!("'{part}' is not a number")))
        })
        .collect()
}

fn triple(text: &str, separator: char) -> Option<[i64; 3]> {
    let parts: Vec<i64> = text
        .split(separator)
        .map(str::parse)
        .collect::<std::result::Result<_, _>>()
        .ok()?;
    <[i64; 3]>::try_from(parts).ok()
}

/// `YYYY-MM-DD HH:MM:SS` as Unix time, read as UTC.
///
/// # Errors
///
/// Returns [`UgosError::Refused`] for text that is not such a time.
pub fn unix_utc(text: &str) -> Result<i64> {
    let bad = || refused(format!("'{text}' is not YYYY-MM-DD HH:MM:SS"));
    let (date, clock) = text.trim().split_once(' ').ok_or_else(bad)?;
    let [year, month, day] = triple(date, '-').ok_or_else(bad)?;
    let [hour, minute, second] = triple(clock, ':').ok_or_else(bad)?;
    if !(1..=12).contains(&month)
        || !(1..=31).contains(&day)
        || hour > 23
        || minute > 59
        || second > 59
    {
        return Err(bad());
    }
    let (year, shifted) = if month <= 2 {
        (year - 1, month + 9)
    } else {
        (year, month - 3)
    };
    let era = year.div_euclid(400);
    let year_of_era = year - era * 400;
    let day_of_year = (153 * shifted + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    Ok((era * 146_097 + day_of_era - 719_468) * 86_400 + hour * 3600 + minute * 60 + second)
}
