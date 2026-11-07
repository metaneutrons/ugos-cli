//! Time, time servers, language, host name and the finder switches.

use serde_json::{Value, json};

use crate::client::UgosClient;
use crate::error::{Result, UgosError};
use crate::types::hw::{Finder, Language, LanguageList, ServerList, TimeConfig};

fn refused(message: impl Into<String>) -> UgosError {
    UgosError::Refused(message.into())
}

/// The time settings.
///
/// # Errors
///
/// Returns the appropriate [`UgosError`] on failure.
pub async fn time(client: &UgosClient) -> Result<TimeConfig> {
    client.get("time/config").await
}

/// How to set the clock.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClockMode {
    /// Keep it from this time server.
    Server(String),
    /// Set it to this Unix time and leave it.
    Manual(i64),
}

/// What to change about time.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TimeChange {
    /// A time zone name from the list.
    pub timezone: Option<String>,
    /// How the clock is kept.
    pub mode: Option<ClockMode>,
}

/// The time servers.
///
/// # Errors
///
/// Returns the appropriate [`UgosError`] on failure.
pub async fn servers(client: &UgosClient) -> Result<Vec<String>> {
    let list: ServerList = client.get("time/server").await?;
    Ok(list.result)
}

/// The body of `time/config`: the settings as read, with the changes.
fn time_body(next: &TimeConfig) -> Value {
    let mut body = json!({
        "timezone": next.timezone, "time_set_type": next.time_set_type,
        "time": if next.time_set_type == 1 { next.time } else { 0 },
        "time_server": next.time_server,
        "date_format": next.date_format, "time_format": next.time_format,
    });
    if next.time_set_type == 1 {
        body["time_show"] = json!(next.time_show);
    }
    body
}

/// Change time and read it back.
///
/// # Errors
///
/// Returns [`UgosError::Refused`] for nothing to change, an unknown time zone
/// or a time more than five years from now, and [`UgosError::NeedsForce`] when
/// UGOS says the cloud certificate would not be valid at the time chosen and
/// `force` is not set.
pub async fn set_time(client: &UgosClient, change: &TimeChange, force: bool) -> Result<TimeConfig> {
    let mut next = time(client).await?;
    if change.timezone.is_none() && change.mode.is_none() {
        return Err(refused("nothing to change"));
    }
    if let Some(zone) = &change.timezone {
        if !next.zones.iter().any(|z| z.name == *zone) {
            return Err(refused(format!("'{zone}' is not a time zone of this NAS")));
        }
        next.timezone.clone_from(zone);
    }
    match &change.mode {
        Some(ClockMode::Server(host)) => {
            if !servers(client).await?.contains(host) {
                return Err(refused(format!(
                    "'{host}' is not in the list of time servers"
                )));
            }
            next.time_set_type = 0;
            next.time_server.clone_from(host);
        }
        Some(ClockMode::Manual(unix)) => {
            check_manual(client, *unix, force).await?;
            next.time_set_type = 1;
            next.time = *unix;
            next.time_show = shown(*unix, next.zones.iter().find(|z| z.name == next.timezone));
        }
        None => {}
    }
    let _: Value = client.post("time/config", &time_body(&next)).await?;
    time(client).await
}

fn shown(unix: i64, _zone: Option<&crate::types::hw::Zone>) -> String {
    let days = unix.div_euclid(86_400);
    let secs = unix.rem_euclid(86_400);
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!(
        "{year:04}-{month:02}-{day:02} {:02}:{:02}:{:02}",
        secs / 3600,
        secs % 3600 / 60,
        secs % 60
    )
}

async fn check_manual(client: &UgosClient, unix: i64, force: bool) -> Result<()> {
    let now = time(client).await?.time;
    let five_years = 5 * 366 * 86_400;
    if (unix - now).abs() > five_years {
        return Err(refused("a manual time is within five years of now"));
    }
    let verified: Result<Value> = client
        .get_with_params(
            "time/verify/cloud/certificate",
            &[("time", &unix.to_string())],
        )
        .await;
    if verified.is_err() && !force {
        return Err(UgosError::NeedsForce(
            "at that time the certificate of the NAS cloud may not be valid, and cloud access can break".into(),
        ));
    }
    Ok(())
}

/// Add a time server.
///
/// # Errors
///
/// Returns [`UgosError::Refused`] for an empty name or one in the list.
pub async fn add_server(client: &UgosClient, host: &str) -> Result<Vec<String>> {
    if host.trim().is_empty() {
        return Err(refused("the time server is empty"));
    }
    if servers(client).await?.iter().any(|s| s == host) {
        return Err(refused(format!("'{host}' is in the list already")));
    }
    let _: Value = client
        .post("time/server", &json!({ "time_server": host }))
        .await?;
    servers(client).await
}

/// Remove a time server. `pool.ntp.org` stays.
///
/// # Errors
///
/// Returns [`UgosError::Refused`] for the default server or one not in the list.
pub async fn remove_server(client: &UgosClient, host: &str) -> Result<Vec<String>> {
    if host == "pool.ntp.org" {
        return Err(refused("pool.ntp.org is the default and stays"));
    }
    if !servers(client).await?.iter().any(|s| s == host) {
        return Err(refused(format!("'{host}' is not in the list")));
    }
    let _: Value = client
        .post("time/server/delete", &json!({ "time_server": host }))
        .await?;
    servers(client).await
}

// ── Language ────────────────────────────────────────────────────────

/// The languages the NAS offers.
///
/// # Errors
///
/// Returns the appropriate [`UgosError`] on failure.
pub async fn languages(client: &UgosClient) -> Result<Vec<Language>> {
    let list: LanguageList = client.get("language/list").await?;
    Ok(list.result)
}

/// The default language of the NAS, a key or `auto`.
///
/// # Errors
///
/// Returns the appropriate [`UgosError`] on failure.
pub async fn language(client: &UgosClient) -> Result<String> {
    let reply: Value = client.get("language/global").await?;
    Ok(reply
        .get("result")
        .and_then(Value::as_str)
        .unwrap_or("auto")
        .to_owned())
}

/// Set the default language of the NAS to a key from the list, or `auto`.
///
/// # Errors
///
/// Returns [`UgosError::Refused`] for a key not in the list.
pub async fn set_language(client: &UgosClient, key: &str) -> Result<String> {
    if key != "auto" && !languages(client).await?.iter().any(|l| l.key == key) {
        return Err(refused(format!("'{key}' is not a language of this NAS")));
    }
    let _: Value = client
        .post("language/global", &json!({ "language": key }))
        .await?;
    language(client).await
}

// ── Host name and finder ────────────────────────────────────────────

/// Check a host name as the web UI does: letters, digits and hyphens, up to
/// 15, not starting with a digit, not starting or ending with a hyphen.
///
/// # Errors
///
/// Returns [`UgosError::Refused`] with the reason.
pub fn check_hostname(name: &str) -> Result<()> {
    if name.is_empty() || name.len() > 15 {
        return Err(refused("a host name is 1 to 15 characters"));
    }
    if !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
        return Err(refused("a host name has letters, digits and hyphens only"));
    }
    if name.starts_with(|c: char| c.is_ascii_digit())
        || name.starts_with('-')
        || name.ends_with('-')
    {
        return Err(refused(
            "a host name does not start with a digit or start or end with a hyphen",
        ));
    }
    Ok(())
}

/// Set the host name and read it back from the machine info.
///
/// # Errors
///
/// Returns [`UgosError::Refused`] for a name the web UI would refuse, or one
/// that did not take.
pub async fn set_hostname(client: &UgosClient, name: &str) -> Result<String> {
    check_hostname(name)?;
    let _: Value = client
        .post("sysinfo/machine/hostname", &json!({ "hostname": name }))
        .await?;
    let info: Value = client.get("sysinfo/machine/common").await?;
    let now = info["common"]["nas_name"]
        .as_str()
        .unwrap_or_default()
        .to_owned();
    if now.eq_ignore_ascii_case(name) {
        Ok(now)
    } else {
        Err(refused(format!(
            "UGOS accepted the request but the name is '{now}'"
        )))
    }
}

/// The finder and improvement-plan switches.
///
/// # Errors
///
/// Returns the appropriate [`UgosError`] on failure.
pub async fn finder(client: &UgosClient) -> Result<Finder> {
    client.get("sysinfo/finder/get").await
}

/// Set the finder switches and read them back.
///
/// # Errors
///
/// Returns [`UgosError::Refused`] for nothing to change or a change UGOS did
/// not keep.
pub async fn set_finder(
    client: &UgosClient,
    improvement: Option<bool>,
    web_find: Option<bool>,
) -> Result<Finder> {
    if improvement.is_none() && web_find.is_none() {
        return Err(refused("nothing to change"));
    }
    let mut next = finder(client).await?;
    if let Some(on) = improvement {
        next.improvement_enable = i64::from(on);
    }
    if let Some(on) = web_find {
        next.web_find = i64::from(on);
    }
    let _: Value = client.post("sysinfo/finder/set", &next).await?;
    let now = finder(client).await?;
    if now == next {
        Ok(now)
    } else {
        Err(refused(
            "UGOS accepted the request but the switches did not change",
        ))
    }
}
