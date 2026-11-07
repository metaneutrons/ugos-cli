//! The settings of the file services, by alias.
//!
//! An alias is the name a person types (`workgroup`, `encryption`). Each
//! maps to the field UGOS keeps it in and says what a value may be, as far as
//! the web UI checks it. Values come as text and leave as JSON.

use std::net::{Ipv4Addr, Ipv6Addr};

use serde_json::Value;

use super::{Config, refused};
use crate::error::Result;

/// What a value may be.
#[derive(Debug, Clone, Copy)]
pub enum Kind {
    /// `on` or `off`.
    Bool,
    /// A whole number in a range.
    Int(i64, i64),
    /// Text of at most this many characters.
    Text(usize),
    /// One of these names, stored as the number next to it.
    Code(&'static [(&'static str, i64)]),
    /// One of these words, stored as text.
    Word(&'static [&'static str]),
    /// `smb1`, `smb2` or both, stored as a list of numbers.
    Protocols,
    /// IP addresses, comma separated, all of one family, or nothing.
    Addresses,
    /// A name list in slashes, like `/a/b/`.
    Veto,
    /// A workgroup name.
    Workgroup,
}

/// A setting.
#[derive(Debug, Clone, Copy)]
pub struct Field {
    /// The name to type.
    pub alias: &'static str,
    /// The field in the configuration.
    pub key: &'static str,
    /// What it takes.
    pub kind: Kind,
    /// One line about it.
    pub help: &'static str,
}

impl Field {
    /// A switch.
    #[must_use]
    pub const fn flag(alias: &'static str, key: &'static str, help: &'static str) -> Self {
        Self {
            alias,
            key,
            kind: Kind::Bool,
            help,
        }
    }

    /// A setting of any kind.
    #[must_use]
    pub const fn of(
        alias: &'static str,
        key: &'static str,
        kind: Kind,
        help: &'static str,
    ) -> Self {
        Self {
            alias,
            key,
            kind,
            help,
        }
    }
}

/// Read `on` or `off` and its common spellings.
///
/// # Errors
///
/// Refuses anything else.
pub fn parse_bool(text: &str) -> Result<bool> {
    match text.trim().to_ascii_lowercase().as_str() {
        "on" | "true" | "yes" | "1" => Ok(true),
        "off" | "false" | "no" | "0" => Ok(false),
        other => Err(refused(format!("{other:?} is not on or off"))),
    }
}

fn int(text: &str, low: i64, high: i64, alias: &str) -> Result<Value> {
    let value: i64 = text
        .trim()
        .parse()
        .map_err(|_| refused(format!("{alias} is a whole number")))?;
    if !(low..=high).contains(&value) {
        return Err(refused(format!("{alias} is {low} to {high}")));
    }
    Ok(Value::from(value))
}

fn addresses(text: &str) -> Result<Value> {
    let text = text.trim();
    if text.is_empty() {
        return Ok(Value::String(String::new()));
    }
    let items: Vec<&str> = text.split(',').map(str::trim).collect();
    let v4 = items.iter().all(|i| i.parse::<Ipv4Addr>().is_ok());
    let v6 = items.iter().all(|i| i.parse::<Ipv6Addr>().is_ok());
    if !(v4 || v6) {
        return Err(refused(
            "the WINS server is IP addresses, all IPv4 or all IPv6, comma separated",
        ));
    }
    Ok(Value::String(items.join(",")))
}

fn workgroup(text: &str) -> Result<Value> {
    const FORBIDDEN: &str = "[]\";:<>+={}\\/*|?,-，。？｜《》；—“”";
    let text = text.trim();
    if text.is_empty() || text.chars().count() > 15 {
        return Err(refused("the workgroup is 1 to 15 characters"));
    }
    if let Some(bad) = text.chars().find(|c| FORBIDDEN.contains(*c)) {
        return Err(refused(format!("the workgroup may not hold {bad:?}")));
    }
    Ok(Value::String(text.into()))
}

fn veto(text: &str) -> Result<Value> {
    let text = text.trim();
    if text.is_empty() {
        return Ok(Value::String(String::new()));
    }
    if text == "//" || !(text.starts_with('/') && text.ends_with('/') && text.len() > 2) {
        return Err(refused(
            "the hidden file names start and end with a slash, like /name/other/",
        ));
    }
    Ok(Value::String(text.into()))
}

fn protocols(text: &str) -> Result<Value> {
    let mut codes = Vec::new();
    for word in text.split(',').map(|w| w.trim().to_ascii_lowercase()) {
        let code = match word.as_str() {
            "smb1" | "1" => 1,
            "smb2" | "smb3" | "2" => 2,
            other => {
                return Err(refused(format!(
                    "{other:?} is not a protocol: smb1 or smb2"
                )));
            }
        };
        if !codes.contains(&code) {
            codes.push(code);
        }
    }
    codes.sort_unstable();
    if codes.is_empty() {
        return Err(refused("at least one protocol is needed"));
    }
    Ok(Value::from(codes))
}

/// Check one value and make its JSON.
///
/// # Errors
///
/// Refuses what the web UI refuses.
pub fn parse_value(field: &Field, text: &str) -> Result<Value> {
    let alias = field.alias;
    match field.kind {
        Kind::Bool => Ok(Value::Bool(parse_bool(text)?)),
        Kind::Int(low, high) => int(text, low, high, alias),
        Kind::Text(max) => {
            if text.chars().count() > max {
                return Err(refused(format!("{alias} is at most {max} characters")));
            }
            Ok(Value::String(text.into()))
        }
        Kind::Code(names) => {
            let word = text.trim().to_ascii_lowercase();
            names
                .iter()
                .find(|(name, _)| *name == word)
                .map(|(_, code)| Value::from(*code))
                .ok_or_else(|| {
                    let list: Vec<&str> = names.iter().map(|(n, _)| *n).collect();
                    refused(format!("{alias} is one of {}", list.join(", ")))
                })
        }
        Kind::Word(words) => words
            .iter()
            .find(|w| w.eq_ignore_ascii_case(text.trim()))
            .map(|w| Value::String((*w).into()))
            .ok_or_else(|| refused(format!("{alias} is one of {}", words.join(", ")))),
        Kind::Protocols => protocols(text),
        Kind::Addresses => addresses(text),
        Kind::Veto => veto(text),
        Kind::Workgroup => workgroup(text),
    }
}

/// The configuration fields a list of settings changes.
///
/// # Errors
///
/// Refuses an unknown alias, a value that does not fit, and a setting named
/// twice.
pub fn patch(fields: &[Field], settings: &[(String, String)]) -> Result<Config> {
    let mut out = Config::new();
    for (alias, text) in settings {
        let field = fields.iter().find(|f| f.alias == alias).ok_or_else(|| {
            let list: Vec<&str> = fields.iter().map(|f| f.alias).collect();
            refused(format!(
                "{alias:?} is no setting; there are {}",
                list.join(", ")
            ))
        })?;
        if out
            .insert(field.key.into(), parse_value(field, text)?)
            .is_some()
        {
            return Err(refused(format!("{alias} is given twice")));
        }
    }
    Ok(out)
}

/// Split `alias=value` as typed on a command line.
///
/// # Errors
///
/// Refuses text without an equals sign.
pub fn split_setting(text: &str) -> Result<(String, String)> {
    text.split_once('=')
        .map(|(a, v)| (a.trim().to_owned(), v.to_owned()))
        .ok_or_else(|| refused(format!("{text:?} is not alias=value")))
}
