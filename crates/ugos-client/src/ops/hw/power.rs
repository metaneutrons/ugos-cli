//! Sleep, power on after a power loss, Wake on LAN, and the power schedule.

use serde_json::{Value, json};

use crate::client::UgosClient;
use crate::error::{Result, UgosError};
use crate::types::hw::{Power, PowerSwitch};

fn refused(message: impl Into<String>) -> UgosError {
    UgosError::Refused(message.into())
}

/// How long disks wait before they sleep: minutes or hours.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sleep {
    /// Disks do not sleep.
    Never,
    /// 10, 20 or 30 minutes.
    Minutes(i64),
    /// 1, 2 or 3 hours.
    Hours(i64),
}

/// What to change. `None` leaves a setting alone.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PowerChange {
    /// Power on again after a power loss.
    pub power_boot: Option<bool>,
    /// Wake on LAN.
    pub wake_on: Option<bool>,
    /// When the internal disks sleep.
    pub sleep: Option<Sleep>,
    /// `1` wake on login, `2` wake on read or write, for the internal disks.
    pub strategy: Option<i64>,
    /// When USB disks sleep.
    pub usb_sleep: Option<Sleep>,
    /// Log the sleeping.
    pub sleep_log: Option<bool>,
    /// Switch the power schedule on or off.
    pub schedule: Option<bool>,
}

/// The power settings.
///
/// # Errors
///
/// Returns the appropriate [`UgosError`] on failure.
pub async fn power(client: &UgosClient) -> Result<Power> {
    client.get("hardware/power/config").await
}

fn check_sleep(sleep: Sleep) -> Result<(bool, String, i64)> {
    match sleep {
        Sleep::Never => Ok((false, "M".into(), 10)),
        Sleep::Minutes(m) if [10, 20, 30].contains(&m) => Ok((true, "M".into(), m)),
        Sleep::Hours(h) if [1, 2, 3].contains(&h) => Ok((true, "H".into(), h)),
        _ => Err(refused(
            "disks sleep after 10, 20 or 30 minutes, or 1, 2 or 3 hours",
        )),
    }
}

/// Apply a change to a copy.
///
/// # Errors
///
/// Returns [`UgosError::Refused`] for a value the web UI would refuse.
pub fn apply(power: &Power, change: &PowerChange) -> Result<Power> {
    let mut next = power.clone();
    if let Some(on) = change.power_boot {
        next.power_boot = on;
    }
    if let Some(on) = change.wake_on {
        next.wake_on = on;
        if on {
            next.power_boot = true;
        }
    }
    if let Some(sleep) = change.sleep {
        (
            next.hard_drive_flag,
            next.hard_drive_unit,
            next.hard_drive_time,
        ) = check_sleep(sleep)?;
    }
    if let Some(sleep) = change.usb_sleep {
        (
            next.usb_hard_drive_flag,
            next.usb_hard_drive_unit,
            next.usb_hard_drive_time,
        ) = check_sleep(sleep)?;
    }
    if let Some(strategy) = change.strategy {
        if ![1, 2].contains(&strategy) {
            return Err(refused(
                "the wake strategy is 1 (login) or 2 (read or write)",
            ));
        }
        next.hard_drive_strategy = strategy;
        next.usb_hard_drive_strategy = strategy;
    }
    if let Some(on) = change.sleep_log {
        next.enable_hard_drive_sleep_log = on;
    }
    if let Some(on) = change.schedule {
        next.enable_scheduled_power = on;
    }
    Ok(next)
}

/// The body of `hardware/power/start`, in the shape the web UI sends: only
/// the keys it names, with the lists of pools and disks that may not sleep
/// as they were.
#[must_use]
pub fn body(power: &Power) -> Value {
    let scope = |key: &str| {
        let inner = power.extra.get(key);
        let list = |name: &str| {
            inner
                .and_then(|v| v.get(name))
                .filter(|v| v.is_array())
                .cloned()
                .unwrap_or_else(|| json!([]))
        };
        json!({
            "excluded_pool_names": list("excluded_pool_names"),
            "excluded_disk_devs": list("excluded_disk_devs"),
        })
    };
    json!({
        "power_boot": power.power_boot, "wake_on": power.wake_on,
        "hard_drive_flag": power.hard_drive_flag, "hard_drive_unit": power.hard_drive_unit,
        "hard_drive_time": power.hard_drive_time, "hard_drive_strategy": power.hard_drive_strategy,
        "hard_drive_scope": scope("hard_drive_scope"),
        "usb_hard_drive_flag": power.usb_hard_drive_flag,
        "usb_hard_drive_unit": power.usb_hard_drive_unit,
        "usb_hard_drive_time": power.usb_hard_drive_time,
        "usb_hard_drive_strategy": power.usb_hard_drive_strategy,
        "usb_hard_drive_scope": scope("usb_hard_drive_scope"),
        "time_switches": power.time_switches,
        "enable_hard_drive_sleep_log": power.enable_hard_drive_sleep_log,
        "enable_scheduled_power": power.enable_scheduled_power,
    })
}

async fn send(client: &UgosClient, next: &Power) -> Result<Power> {
    if !power(client).await?.storage_running {
        return Err(refused(
            "the storage service is starting; try again in a moment",
        ));
    }
    let _: Value = client.post("hardware/power/start", &body(next)).await?;
    power(client).await
}

fn same(now: &Power, wanted: &Power) -> bool {
    now.power_boot == wanted.power_boot
        && now.wake_on == wanted.wake_on
        && now.hard_drive_flag == wanted.hard_drive_flag
        && now.hard_drive_time == wanted.hard_drive_time
        && now.hard_drive_unit == wanted.hard_drive_unit
        && now.enable_scheduled_power == wanted.enable_scheduled_power
        && now.time_switches.len() == wanted.time_switches.len()
}

/// Change the power settings and read them back.
///
/// # Errors
///
/// Returns [`UgosError::Refused`] for nothing to change, a value the web UI
/// would refuse, or a change UGOS did not keep.
pub async fn set_power(client: &UgosClient, change: &PowerChange) -> Result<Power> {
    if *change == PowerChange::default() {
        return Err(refused("nothing to change"));
    }
    let next = apply(&power(client).await?, change)?;
    let now = send(client, &next).await?;
    if same(&now, &next) {
        Ok(now)
    } else {
        Err(refused(
            "UGOS accepted the request but the power settings did not change",
        ))
    }
}

/// A schedule entry: weekdays, `HH:MM`, and whether it powers on or off.
///
/// # Errors
///
/// Returns [`UgosError::Refused`] for a weekday outside 1 to 7, a time that is
/// not `HH:MM`, or another action than `open` or `close`.
pub fn switch(days: &[i64], time: &str, action: &str) -> Result<PowerSwitch> {
    if days.is_empty() || days.iter().any(|d| !(1..=7).contains(d)) {
        return Err(refused("weekdays are 1 (Monday) to 7 (Sunday)"));
    }
    let valid = time.split_once(':').is_some_and(|(h, m)| {
        h.len() == 2
            && m.len() == 2
            && h.parse::<u8>().is_ok_and(|h| h < 24)
            && m.parse::<u8>().is_ok_and(|m| m < 60)
    });
    if !valid {
        return Err(refused(format!("'{time}' is not a time as HH:MM")));
    }
    if !["open", "close"].contains(&action) {
        return Err(refused(
            "the action is open (power on) or close (power off)",
        ));
    }
    let mut days = days.to_vec();
    days.sort_unstable();
    days.dedup();
    Ok(PowerSwitch {
        frequency: days,
        time: time.into(),
        action: action.into(),
    })
}

/// Add an entry to the power schedule and switch the schedule on.
///
/// # Errors
///
/// Returns [`UgosError::Refused`] for a duplicate entry or one the web UI
/// would refuse.
pub async fn add_switch(client: &UgosClient, entry: PowerSwitch) -> Result<Power> {
    let mut next = power(client).await?;
    if next.time_switches.contains(&entry) {
        return Err(refused("that schedule entry exists"));
    }
    if entry.action == "open" {
        next.power_boot = true;
    }
    next.time_switches.push(entry);
    next.enable_scheduled_power = true;
    let now = send(client, &next).await?;
    if now.time_switches.len() == next.time_switches.len() {
        Ok(now)
    } else {
        Err(refused(
            "UGOS accepted the request but the entry is not in the schedule",
        ))
    }
}

/// Remove the entry at this position, counting from 1.
///
/// # Errors
///
/// Returns [`UgosError::Refused`] for a position with no entry.
pub async fn remove_switch(client: &UgosClient, position: usize) -> Result<Power> {
    let mut next = power(client).await?;
    if position == 0 || position > next.time_switches.len() {
        return Err(refused(format!(
            "there is no entry {position}; there are {}",
            next.time_switches.len()
        )));
    }
    let _ = next.time_switches.remove(position - 1);
    if next.time_switches.is_empty() {
        next.enable_scheduled_power = false;
    }
    let now = send(client, &next).await?;
    if now.time_switches.len() == next.time_switches.len() {
        Ok(now)
    } else {
        Err(refused(
            "UGOS accepted the request but the entry is still there",
        ))
    }
}
