//! Time-based one-time passwords ([RFC 6238](https://www.rfc-editor.org/rfc/rfc6238)).
//!
//! UGOS authenticator apps use the common defaults: HMAC-SHA1, 6 digits, a
//! 30 second step. A secret may also be an `otpauth://totp/` URI, in which
//! case `digits` and `period` are taken from it. SHA-256, SHA-512 and HOTP
//! are rejected: this client has not seen UGOS use them.

use std::time::{SystemTime, UNIX_EPOCH};

use crate::error::{Result, UgosError};

/// A parsed authenticator secret.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Totp {
    /// Raw HMAC key, after base32 decoding.
    key: Vec<u8>,
    /// Decimal digits in the code. 6, 7 or 8.
    digits: u32,
    /// Step length in seconds.
    period: u64,
}

impl Totp {
    /// Digits this secret asks for.
    #[cfg(test)]
    #[must_use]
    pub const fn digits(&self) -> u32 {
        self.digits
    }

    /// Step length in seconds.
    #[cfg(test)]
    #[must_use]
    pub const fn period(&self) -> u64 {
        self.period
    }
}

/// ASCII form of the [RFC 6238](https://www.rfc-editor.org/rfc/rfc6238) SHA-1
/// test secret (`"12345678901234567890"`).
#[cfg(test)]
pub const RFC6238_SHA1_SECRET_BASE32: &str = "GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ";

/// Parse a base32 secret or an `otpauth://totp/` URI.
///
/// # Errors
///
/// Returns a description the caller can attach to [`UgosError::InvalidOtpSecret`].
pub fn parse_secret(input: &str) -> std::result::Result<Totp, String> {
    let input = input.trim();
    if input.is_empty() {
        return Err("secret is empty".into());
    }
    if let Some(rest) = strip_scheme(input, "otpauth") {
        return parse_otpauth(rest);
    }
    Ok(Totp {
        key: decode_base32(input)?,
        digits: 6,
        period: 30,
    })
}

/// Code for `unix_time`, using this secret's digits and period.
#[must_use]
pub fn code_at(totp: &Totp, unix_time: u64) -> String {
    let period = totp.period.max(1);
    hotp(&totp.key, unix_time / period, totp.digits)
}

/// Seconds since the Unix epoch.
///
/// # Errors
///
/// Returns [`UgosError::InvalidOtpSecret`] when the clock is before 1970,
/// which would make every code undefined.
pub fn unix_now() -> Result<u64> {
    #[cfg(test)]
    if let Some(forced) = forced_unix() {
        return Ok(forced);
    }
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_secs())
        .map_err(|_| UgosError::InvalidOtpSecret("system clock is before the unix epoch".into()))
}

/// Fix the clock seen by [`unix_now`]. `0` returns to the real clock.
#[cfg(test)]
pub fn force_unix_time(seconds: u64) {
    FORCED_UNIX.store(seconds, std::sync::atomic::Ordering::SeqCst);
}

#[cfg(test)]
static FORCED_UNIX: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

#[cfg(test)]
fn forced_unix() -> Option<u64> {
    match FORCED_UNIX.load(std::sync::atomic::Ordering::SeqCst) {
        0 => None,
        seconds => Some(seconds),
    }
}

fn strip_scheme<'a>(input: &'a str, scheme: &str) -> Option<&'a str> {
    let (head, rest) = input.split_once("://")?;
    head.eq_ignore_ascii_case(scheme).then_some(rest)
}

fn parse_otpauth(rest: &str) -> std::result::Result<Totp, String> {
    let (kind, after_kind) = rest
        .split_once('/')
        .ok_or_else(|| "otpauth URI has no type".to_owned())?;
    if !kind.eq_ignore_ascii_case("totp") {
        return Err("only otpauth totp URIs are supported".into());
    }
    let query = after_kind.split_once('?').map_or("", |(_, query)| query);

    let mut secret = None;
    let mut digits = 6u32;
    let mut period = 30u64;
    let mut algorithm = String::from("SHA1");
    for pair in query.split('&') {
        if pair.is_empty() {
            continue;
        }
        let (name, value) = pair.split_once('=').unwrap_or((pair, ""));
        let value = percent_decode(value);
        match name.to_ascii_lowercase().as_str() {
            "secret" => secret = Some(value),
            "digits" => {
                digits = value
                    .parse()
                    .map_err(|_| format!("digits is not a number: {value}"))?;
            }
            "period" => {
                period = value
                    .parse()
                    .map_err(|_| format!("period is not a number: {value}"))?;
            }
            "algorithm" => algorithm = value,
            _ => {}
        }
    }

    let algorithm_normal: String = algorithm.chars().filter(|c| *c != '-').collect();
    if !algorithm_normal.eq_ignore_ascii_case("SHA1") {
        return Err(format!(
            "only HMAC-SHA1 is supported (URI asked for {algorithm})"
        ));
    }
    if !(6..=8).contains(&digits) {
        return Err(format!("digits must be 6, 7 or 8 (URI asked for {digits})"));
    }
    if period == 0 {
        return Err("period must be greater than zero".into());
    }
    let secret = secret.ok_or_else(|| "otpauth URI has no secret".to_owned())?;
    Ok(Totp {
        key: decode_base32(&secret)?,
        digits,
        period,
    })
}

/// HMAC-SHA1, then the dynamic truncation from RFC 4226.
fn hotp(key: &[u8], counter: u64, digits: u32) -> String {
    let mac = hmac_sha1(key, &counter.to_be_bytes());
    let offset = usize::from(mac[19] & 0x0f);
    let binary = (u32::from(mac[offset] & 0x7f) << 24)
        | (u32::from(mac[offset + 1]) << 16)
        | (u32::from(mac[offset + 2]) << 8)
        | u32::from(mac[offset + 3]);
    let modulus = 10u32.checked_pow(digits).unwrap_or(1_000_000);
    let code = binary % modulus;
    let width = usize::try_from(digits).unwrap_or(6);
    format!("{code:0width$}")
}

fn hmac_sha1(key: &[u8], message: &[u8]) -> [u8; 20] {
    let mac_key = ring::hmac::Key::new(ring::hmac::HMAC_SHA1_FOR_LEGACY_USE_ONLY, key);
    let tag = ring::hmac::sign(&mac_key, message);
    let bytes = tag.as_ref();
    let mut out = [0u8; 20];
    if bytes.len() == out.len() {
        out.copy_from_slice(bytes);
    }
    out
}

fn decode_base32(input: &str) -> std::result::Result<Vec<u8>, String> {
    let mut acc: u32 = 0;
    let mut acc_bits: u32 = 0;
    let mut out = Vec::new();
    let mut saw_digit = false;
    for byte in input.bytes() {
        if byte.is_ascii_whitespace() || byte == b'=' {
            continue;
        }
        let Some(value) = base32_value(byte) else {
            return Err(format!("invalid base32 character U+{byte:04X}"));
        };
        saw_digit = true;
        acc = (acc << 5) | u32::from(value);
        acc_bits += 5;
        if acc_bits >= 8 {
            acc_bits -= 8;
            let shifted = (acc >> acc_bits) & 0xff;
            if let Ok(decoded) = u8::try_from(shifted) {
                out.push(decoded);
            }
            acc &= (1u32 << acc_bits) - 1;
        }
    }
    if !saw_digit || out.is_empty() {
        return Err("secret is empty".into());
    }
    Ok(out)
}

const fn base32_value(byte: u8) -> Option<u8> {
    match byte {
        b'A'..=b'Z' => Some(byte - b'A'),
        b'a'..=b'z' => Some(byte - b'a'),
        b'2'..=b'7' => Some(byte - b'2' + 26),
        _ => None,
    }
}

fn percent_decode(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' && index + 2 < bytes.len() {
            let hex = std::str::from_utf8(&bytes[index + 1..index + 3]).unwrap_or("");
            if let Ok(value) = u8::from_str_radix(hex, 16) {
                out.push(value);
                index += 3;
                continue;
            }
        }
        out.push(bytes[index]);
        index += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::{RFC6238_SHA1_SECRET_BASE32, Totp, code_at, parse_secret};

    /// RFC 6238 appendix B, SHA-1, 8 digits.
    const RFC6238_SHA1: &[(u64, &str)] = &[
        (59, "94287082"),
        (1_111_111_109, "07081804"),
        (1_111_111_111, "14050471"),
        (1_234_567_890, "89005924"),
        (2_000_000_000, "69279037"),
        (20_000_000_000, "65353130"),
    ];

    fn rfc_secret() -> Totp {
        Totp {
            key: b"12345678901234567890".to_vec(),
            digits: 8,
            period: 30,
        }
    }

    #[test]
    fn rfc6238_sha1_vectors() {
        let secret = rfc_secret();
        for (time, expected) in RFC6238_SHA1 {
            assert_eq!(code_at(&secret, *time), *expected, "T = {time}");
        }
    }

    #[test]
    fn base32_form_is_the_rfc_secret() {
        let parsed = parse_secret(RFC6238_SHA1_SECRET_BASE32).unwrap();
        assert_eq!(parsed.key, b"12345678901234567890");
        assert_eq!(parsed.digits(), 6);
        assert_eq!(parsed.period(), 30);
        // 8-digit value 94287082, reduced to the default 6 digits.
        assert_eq!(code_at(&parsed, 59), "287082");
    }

    #[test]
    fn base32_ignores_case_spaces_and_padding() {
        let padded = "gezd gnbv gy3t qojq gezd gnbv gy3t qojq====";
        let parsed = parse_secret(padded).unwrap();
        assert_eq!(parsed.key, b"12345678901234567890");
    }

    #[test]
    fn otpauth_uri_carries_digits_and_period() {
        let uri = format!(
            "otpauth://totp/UGOS:admin?secret={RFC6238_SHA1_SECRET_BASE32}&issuer=UGOS&algorithm=SHA1&digits=8&period=30"
        );
        let parsed = parse_secret(&uri).unwrap();
        assert_eq!(parsed.digits(), 8);
        assert_eq!(parsed.period(), 30);
        assert_eq!(code_at(&parsed, 59), "94287082");
    }

    #[test]
    fn otpauth_accepts_sha1_with_a_hyphen_and_any_case() {
        let uri =
            format!("OTPAuth://TOTP/label?algorithm=SHA-1&secret={RFC6238_SHA1_SECRET_BASE32}");
        assert!(parse_secret(&uri).is_ok());
    }

    #[test]
    fn the_same_step_repeats_and_the_next_step_changes() {
        let secret = parse_secret(RFC6238_SHA1_SECRET_BASE32).unwrap();
        // 1_700_000_010 is divisible by 30, so both samples share one counter.
        let first = code_at(&secret, 1_700_000_010);
        let still = code_at(&secret, 1_700_000_039);
        let next = code_at(&secret, 1_700_000_040);
        assert_eq!(first, still);
        assert_ne!(first, next);
    }

    #[test]
    fn other_algorithms_and_hotp_are_rejected() {
        let sha256 =
            format!("otpauth://totp/a?secret={RFC6238_SHA1_SECRET_BASE32}&algorithm=SHA256");
        assert!(parse_secret(&sha256).unwrap_err().contains("HMAC-SHA1"));
        let hotp = format!("otpauth://hotp/a?secret={RFC6238_SHA1_SECRET_BASE32}");
        assert!(parse_secret(&hotp).unwrap_err().contains("totp"));
    }

    #[test]
    fn a_uri_without_a_secret_or_with_a_bad_period_is_rejected() {
        assert!(parse_secret("otpauth://totp/a?digits=6").is_err());
        let zero = format!("otpauth://totp/a?secret={RFC6238_SHA1_SECRET_BASE32}&period=0");
        assert!(parse_secret(&zero).unwrap_err().contains("period"));
        let digits = format!("otpauth://totp/a?secret={RFC6238_SHA1_SECRET_BASE32}&digits=9");
        assert!(parse_secret(&digits).unwrap_err().contains("digits"));
    }

    #[test]
    fn empty_and_invalid_base32_are_rejected() {
        assert!(parse_secret("   ").is_err());
        assert!(parse_secret("!!!!!!!!").unwrap_err().contains("base32"));
    }
}
