//! Buzzer, CPU mode, fans, memory compression, LEDs and the small display.

use serde_json::{Value, json};

use super::{needs_reboot, reboot};
use crate::client::UgosClient;
use crate::error::{Result, UgosError};
use crate::types::hw::{Buzzer, Cpu, Fan, Led, LedDays, LedHours, LedScreen};

fn refused(message: impl Into<String>) -> UgosError {
    UgosError::Refused(message.into())
}

// ── Buzzer ──────────────────────────────────────────────────────────

/// What to change about the buzzer. `None` leaves an event alone.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BuzzerChange {
    /// A fan failed.
    pub fan_fault: Option<bool>,
    /// A cache disk failed.
    pub cache_exception: Option<bool>,
    /// The NAS starts.
    pub system_startup: Option<bool>,
    /// The NAS shuts down.
    pub system_shutdown: Option<bool>,
    /// A disk is too hot.
    pub temperature_too_high: Option<bool>,
}

/// The buzzer settings.
///
/// # Errors
///
/// Returns the appropriate [`UgosError`] on failure.
pub async fn buzzer(client: &UgosClient) -> Result<Buzzer> {
    client.get("hardware/buzzer/config").await
}

/// Change the buzzer settings and read them back.
///
/// # Errors
///
/// Returns [`UgosError::Refused`] for nothing to change, an event this model
/// lacks, or a change UGOS did not keep.
pub async fn set_buzzer(client: &UgosClient, change: &BuzzerChange) -> Result<Buzzer> {
    let mut next = buzzer(client).await?;
    let flag = |on: bool| i64::from(on);
    let mut touched = false;
    for (value, slot) in [
        (change.fan_fault, &mut next.fan_fault),
        (change.cache_exception, &mut next.cache_exception),
        (change.system_shutdown, &mut next.system_shutdown),
        (change.temperature_too_high, &mut next.temperature_too_high),
    ] {
        if let Some(on) = value {
            *slot = flag(on);
            touched = true;
        }
    }
    if let Some(on) = change.system_startup {
        let slot = next
            .system_startup
            .as_mut()
            .ok_or_else(|| refused("this model has no beep at startup"))?;
        *slot = flag(on);
        touched = true;
    }
    if !touched {
        return Err(refused("nothing to change"));
    }
    let _: Value = client.post("hardware/buzzer/start", &next).await?;
    let now = buzzer(client).await?;
    if now == next {
        Ok(now)
    } else {
        Err(refused(
            "UGOS accepted the request but the buzzer settings did not change",
        ))
    }
}

/// What the alarm check reports: `fan`, `hdd`, `temp`, or nothing.
///
/// # Errors
///
/// Returns the appropriate [`UgosError`] on failure.
pub async fn buzzer_alarms(client: &UgosClient) -> Result<Value> {
    let reply: Value = client.get("hardware/buzzer/testBuzzer").await?;
    Ok(reply.get("result").cloned().unwrap_or(Value::Null))
}

// ── CPU ─────────────────────────────────────────────────────────────

/// The CPU performance mode.
///
/// # Errors
///
/// Returns the appropriate [`UgosError`] on failure.
pub async fn cpu(client: &UgosClient) -> Result<Cpu> {
    client.get("hardware/cpu/config").await
}

/// Set the CPU mode: `0` high performance, `1` balanced, `2` power saving.
///
/// # Errors
///
/// Returns [`UgosError::Refused`] for a mode out of range, a model without the
/// setting, or a change UGOS did not keep.
pub async fn set_cpu(client: &UgosClient, mode: i64) -> Result<Cpu> {
    if !(0..=2).contains(&mode) {
        return Err(refused(
            "the CPU mode is 0 (high), 1 (balanced) or 2 (power saving)",
        ));
    }
    if cpu(client).await?.is_support != 1 {
        return Err(refused("this model has no CPU mode setting"));
    }
    let _: Value = client
        .post("hardware/cpu/frequency", &json!({ "frequency": mode }))
        .await?;
    let now = cpu(client).await?;
    if now.frequency == mode {
        Ok(now)
    } else {
        Err(refused(
            "UGOS accepted the request but the CPU mode did not change",
        ))
    }
}

// ── Fans ────────────────────────────────────────────────────────────

/// The fan setting.
///
/// # Errors
///
/// Returns the appropriate [`UgosError`] on failure.
pub async fn fan(client: &UgosClient) -> Result<Fan> {
    client.get("hardware/fan/config").await
}

/// Set a preset fan mode: `1` quiet, `2` automatic, `3` full speed.
///
/// # Errors
///
/// Returns [`UgosError::Refused`] for another mode, or a change UGOS did not
/// keep.
pub async fn set_fan_mode(client: &UgosClient, mode: i64) -> Result<Fan> {
    if !(1..=3).contains(&mode) {
        return Err(refused(
            "the fan mode is 1 (quiet), 2 (automatic) or 3 (full speed)",
        ));
    }
    let _: Value = client
        .get_with_params("hardware/fan/start", &[("mode", &mode.to_string())])
        .await?;
    let now = fan(client).await?;
    if now.mode == mode {
        Ok(now)
    } else {
        Err(refused(
            "UGOS accepted the request but the fan mode did not change",
        ))
    }
}

/// Temperatures of a custom curve: 17 points, 0 to 80 °C in steps of 5.
pub const CURVE_POINTS: usize = 17;

/// From this temperature on the duty is 100 %, as the web UI forces for disks
/// that run hot. It uses the lowest limit (a hard disk's), whatever disks are
/// in the NAS.
const FULL_FROM: usize = 11;

/// The pairs `[temperature, duty]` of a curve from its duties. Duties are 20
/// to 100 %, and 100 from 55 °C.
///
/// # Errors
///
/// Returns [`UgosError::Refused`] for the wrong number of points or a duty out
/// of range.
pub fn curve(duties: &[i64]) -> Result<Vec<[i64; 2]>> {
    if duties.len() != CURVE_POINTS {
        return Err(refused(format!(
            "a fan curve has {CURVE_POINTS} duties, for 0, 5, ... 80 °C; got {}",
            duties.len()
        )));
    }
    if let Some(bad) = duties.iter().find(|d| !(20..=100).contains(*d)) {
        return Err(refused(format!("a fan duty is 20 to 100 %, not {bad}")));
    }
    Ok(duties
        .iter()
        .enumerate()
        .map(|(i, duty)| {
            let temp = i64::try_from(i).unwrap_or(0) * 5;
            [temp, if i >= FULL_FROM { 100 } else { *duty }]
        })
        .collect())
}

/// Set a custom fan curve, for the device fan and the CPU fan.
///
/// # Errors
///
/// Returns [`UgosError::Refused`] when the NAS has no custom curve, or for a
/// curve the web UI would refuse.
pub async fn set_fan_curve(client: &UgosClient, sys: &[i64], cpu: &[i64]) -> Result<Fan> {
    if !fan(client).await?.support_custom {
        return Err(refused("this NAS has no custom fan curve"));
    }
    let body = json!({
        "mode": 4, "sys_fan_table": curve(sys)?, "cpu_fan_table": curve(cpu)?,
    });
    let _: Value = client.post("hardware/fan/setfan", &body).await?;
    let now = fan(client).await?;
    if now.mode == 4 {
        Ok(now)
    } else {
        Err(refused(
            "UGOS accepted the request but the fan did not switch to the curve",
        ))
    }
}

// ── Memory compression ──────────────────────────────────────────────

/// Whether memory compression is on.
///
/// # Errors
///
/// Returns the appropriate [`UgosError`] on failure.
pub async fn zram(client: &UgosClient) -> Result<bool> {
    let reply: Value = client.get("hardware/zram/config").await?;
    Ok(reply
        .get("enable")
        .and_then(Value::as_bool)
        .unwrap_or(false))
}

/// Switch memory compression on.
///
/// # Errors
///
/// Returns [`UgosError::Refused`] when UGOS did not switch it on.
pub async fn zram_on(client: &UgosClient) -> Result<()> {
    let _: Value = client.post("hardware/zram/start", &json!({})).await?;
    if zram(client).await? {
        Ok(())
    } else {
        Err(refused(
            "UGOS accepted the request but memory compression is off",
        ))
    }
}

/// Switch memory compression off, which restarts the NAS: the web UI
/// reboots it right after.
///
/// # Errors
///
/// Returns [`UgosError::NeedsForce`] without `force`.
pub async fn zram_off(client: &UgosClient, force: bool) -> Result<()> {
    if !force {
        return Err(needs_reboot("switching memory compression off"));
    }
    let _: Value = client.post("hardware/zram/stop", &json!({})).await?;
    reboot(client).await
}

// ── LEDs ────────────────────────────────────────────────────────────

/// The status LEDs.
///
/// # Errors
///
/// Returns the appropriate [`UgosError`] on failure.
pub async fn led(client: &UgosClient) -> Result<Led> {
    client.get("hardware/led/config").await
}

/// Switch the LEDs on, with the schedule they have.
///
/// # Errors
///
/// Returns [`UgosError::Refused`] when UGOS did not switch them on.
pub async fn led_on(client: &UgosClient) -> Result<Led> {
    let current = led(client).await?;
    let _: Value = client
        .post("hardware/led/start", &json!({ "info": current.info }))
        .await?;
    let now = led(client).await?;
    if now.status == 1 {
        Ok(now)
    } else {
        Err(refused("UGOS accepted the request but the LEDs are not on"))
    }
}

/// Switch the LEDs off.
///
/// # Errors
///
/// Returns [`UgosError::Refused`] when UGOS did not switch them off.
pub async fn led_off(client: &UgosClient) -> Result<Led> {
    let _: Value = client.get("hardware/led/stop").await?;
    let now = led(client).await?;
    if now.status == 2 {
        Ok(now)
    } else {
        Err(refused(
            "UGOS accepted the request but the LEDs are not off",
        ))
    }
}

/// Set the LED brightness for some weekdays, all day, in percent.
///
/// # Errors
///
/// Returns [`UgosError::Refused`] for a brightness over 100, a weekday outside
/// 1 to 7, or a change UGOS did not keep.
pub async fn led_brightness(client: &UgosClient, days: &[i64], bright: i64) -> Result<Led> {
    if !(0..=100).contains(&bright) {
        return Err(refused("the brightness is 0 to 100 %"));
    }
    if days.is_empty() || days.iter().any(|d| !(1..=7).contains(d)) {
        return Err(refused("weekdays are 1 (Monday) to 7 (Sunday)"));
    }
    let mut info = led(client).await?.info;
    for group in &mut info {
        group.days.retain(|d| !days.contains(d));
    }
    info.retain(|group| !group.days.is_empty());
    info.push(LedDays {
        days: days.to_vec(),
        time_config: vec![LedHours {
            time: (0..24).collect(),
            bright,
        }],
    });
    let _: Value = client
        .post("hardware/led/start", &json!({ "info": info }))
        .await?;
    led(client).await
}

// ── The small display ───────────────────────────────────────────────

/// The settings of the small display, if the model has one.
///
/// # Errors
///
/// Returns the appropriate [`UgosError`] on failure.
pub async fn screen(client: &UgosClient) -> Result<LedScreen> {
    client.get("hardware/ledscreen/settings").await
}

/// What to change about the small display.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ScreenChange {
    /// Brightness, 25 to 100.
    pub brightness: Option<i64>,
    /// Seconds until it goes dark: 15, 30, 60, 120, 300, or 0 for never.
    pub screen_off_seconds: Option<i64>,
    /// A wallpaper path from the list.
    pub wallpaper_path: Option<String>,
    /// A language key from the language list.
    pub language_key: Option<String>,
}

/// The wallpapers the display offers.
///
/// # Errors
///
/// Returns the appropriate [`UgosError`] on failure.
pub async fn wallpapers(client: &UgosClient) -> Result<Vec<String>> {
    let reply: Value = client.get("hardware/ledscreen/wallpaperlist").await?;
    Ok(reply
        .get("list")
        .and_then(Value::as_array)
        .map(|list| {
            list.iter()
                .filter_map(Value::as_str)
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default())
}

/// Change the small display and read it back. The whole object goes back.
///
/// # Errors
///
/// Returns [`UgosError::Refused`] for a value the web UI would refuse or a
/// change UGOS did not keep.
pub async fn set_screen(client: &UgosClient, change: &ScreenChange) -> Result<LedScreen> {
    let mut next = screen(client).await?;
    if let Some(value) = change.brightness {
        if !(25..=100).contains(&value) {
            return Err(refused("the brightness is 25 to 100 %"));
        }
        let _ = next.insert("brightness".into(), json!(value));
    }
    if let Some(value) = change.screen_off_seconds {
        if ![0, 15, 30, 60, 120, 300].contains(&value) {
            return Err(refused(
                "the time until dark is 15, 30, 60, 120, 300 or 0 seconds",
            ));
        }
        let _ = next.insert("screen_off_seconds".into(), json!(value));
    }
    if let Some(path) = &change.wallpaper_path {
        if !wallpapers(client).await?.contains(path) {
            return Err(refused(format!("'{path}' is not one of the wallpapers")));
        }
        let _ = next.insert("wallpaper_path".into(), json!(path));
    }
    if let Some(key) = &change.language_key {
        let _ = next.insert("language_key".into(), json!(key));
    }
    let _: Value = client.post("hardware/ledscreen/settings", &next).await?;
    let now = screen(client).await?;
    if now == next {
        Ok(now)
    } else {
        Err(refused(
            "UGOS accepted the request but the display settings did not change",
        ))
    }
}
