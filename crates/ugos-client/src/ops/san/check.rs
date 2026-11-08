//! The input rules of the SAN Manager's forms.

use super::refused;
use crate::error::Result;

fn all(text: &str, ok: impl Fn(char) -> bool) -> bool {
    !text.is_empty() && text.chars().all(ok)
}

/// A LUN, group or target name: letters, digits, `_` and `-`, up to 32.
///
/// # Errors
///
/// Refuses anything else.
pub fn name(text: &str) -> Result<()> {
    let ok = all(text, |c| c.is_ascii_alphanumeric() || c == '_' || c == '-');
    if !ok || text.chars().count() > 32 {
        return Err(refused(
            "a name is letters, digits, _ and -, up to 32 characters",
        ));
    }
    Ok(())
}

/// A description: up to 32 characters without the characters the web UI
/// strips.
///
/// # Errors
///
/// Refuses anything else.
pub fn description(text: &str) -> Result<()> {
    const FORBIDDEN: &str = "!,`:'\"/\\<>?#@$%*;{}[]()|";
    if text.chars().count() > 32 {
        return Err(refused("a description is up to 32 characters"));
    }
    if let Some(bad) = text.chars().find(|c| FORBIDDEN.contains(*c) || *c == '\n') {
        return Err(refused(format!("a description may not hold {bad:?}")));
    }
    Ok(())
}

/// The IQN of a target: letters, digits, `:`, `.` and `-`, up to 128.
///
/// # Errors
///
/// Refuses anything else.
pub fn target_iqn(text: &str) -> Result<()> {
    let ok = all(text, |c| c.is_ascii_alphanumeric() || ":.-".contains(c));
    if !ok || text.chars().count() > 128 {
        return Err(refused(
            "a target IQN is letters, digits, : . and -, up to 128 characters",
        ));
    }
    Ok(())
}

/// An initiator IQN of a group: letters, digits, `.`, `:` and `-`, up to 223.
///
/// # Errors
///
/// Refuses anything else.
pub fn initiator_iqn(text: &str) -> Result<()> {
    let ok = all(text, |c| c.is_ascii_alphanumeric() || ".:-".contains(c));
    if !ok || text.chars().count() > 223 {
        return Err(refused(
            "an initiator IQN is letters, digits, . : and -, up to 223 characters",
        ));
    }
    Ok(())
}

/// A CHAP user: letters and digits, up to 12.
///
/// # Errors
///
/// Refuses anything else.
pub fn chap_user(text: &str) -> Result<()> {
    if !all(text, |c| c.is_ascii_alphanumeric()) || text.chars().count() > 12 {
        return Err(refused("a CHAP user is letters and digits, up to 12"));
    }
    Ok(())
}

/// A CHAP password: letters and digits, 12 to 16.
///
/// # Errors
///
/// Refuses anything else.
pub fn chap_password(text: &str) -> Result<()> {
    let n = text.chars().count();
    if !all(text, |c| c.is_ascii_alphanumeric()) || !(12..=16).contains(&n) {
        return Err(refused("a CHAP password is letters and digits, 12 to 16"));
    }
    Ok(())
}

/// The start of every IQN: letters, digits, `-` and `.`, up to 95, lower case.
///
/// # Errors
///
/// Refuses anything else.
pub fn iqn_prefix(text: &str) -> Result<String> {
    let ok = all(text, |c| c.is_ascii_alphanumeric() || c == '-' || c == '.');
    if !ok || text.chars().count() > 95 {
        return Err(refused(
            "an IQN prefix is letters, digits, - and ., up to 95 characters",
        ));
    }
    Ok(text.to_ascii_lowercase())
}

/// A size as `10`, `10G`, `10GB`, `2T` or `2TB`, in whole units of GB or TB.
///
/// # Errors
///
/// Refuses a fraction, a size of 0 and any other unit.
pub fn size(text: &str) -> Result<(i64, &'static str)> {
    let t = text.trim().to_ascii_uppercase();
    let (digits, unit) = t
        .strip_suffix("TB")
        .or_else(|| t.strip_suffix('T'))
        .map_or_else(
            || {
                let d = t
                    .strip_suffix("GB")
                    .or_else(|| t.strip_suffix('G'))
                    .unwrap_or(&t);
                (d, "GB")
            },
            |d| (d, "TB"),
        );
    let value: i64 = digits
        .trim()
        .parse()
        .map_err(|_| refused("a size is a whole number of GB or TB, like 100G or 2T"))?;
    if value <= 0 {
        return Err(refused("a size is above 0"));
    }
    Ok((value, unit))
}

/// The bytes in a size.
#[must_use]
pub const fn bytes(value: i64, unit: &str) -> i64 {
    if unit.as_bytes()[0] == b'T' {
        value.saturating_mul(1 << 40)
    } else {
        value.saturating_mul(1 << 30)
    }
}
