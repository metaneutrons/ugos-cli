//! Hardware, power, time, language and terminal: the logic behind the
//! `ugos_hw_*`, `ugos_time_*`, `ugos_lang_*`, `ugos_terminal_*` and
//! `ugos_system_*` tools.
//!
//! The same operations as `ugos-cli`. A change that restarts the NAS or opens
//! a way in needs `force`, and the model is told to ask the user first.

use rmcp::schemars;
use serde::Serialize;
use ugos_client::UgosClient;
use ugos_client::error::UgosError;
use ugos_client::ops::hw::basic::{self, BuzzerChange, ScreenChange};
use ugos_client::ops::hw::clock::{self, ClockMode, TimeChange};
use ugos_client::ops::hw::gpu::{self, GpuMode};
use ugos_client::ops::hw::parse;
use ugos_client::ops::hw::power::{self, PowerChange};
use ugos_client::ops::hw::terminal::{self, TerminalChange};
use ugos_client::ops::hw::ups::{self, Source, UpsSetting};

use crate::manage::failure;

fn json<T: Serialize>(result: Result<T, UgosError>) -> String {
    match result {
        Ok(value) => serde_json::to_string_pretty(&value).unwrap_or_default(),
        Err(e) => failure(&e),
    }
}

fn done(result: Result<String, UgosError>) -> String {
    result.unwrap_or_else(|e| failure(&e))
}

fn refused(message: &str) -> UgosError {
    UgosError::Refused(message.into())
}

// ── Parameters ──────────────────────────────────────────────────────

/// A switch that needs `force` to go on.
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ForceParam {
    /// Go on although this restarts the NAS. Ask the user first.
    #[serde(default)]
    pub force: bool,
    /// Target NAS name or host. Required when multiple targets are configured.
    #[serde(default)]
    pub target: String,
}

/// A position in a list.
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct PositionParam {
    /// Position, counting from 1.
    pub position: usize,
    /// Target NAS name or host. Required when multiple targets are configured.
    #[serde(default)]
    pub target: String,
}

/// Which events sound the buzzer.
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct BuzzerParam {
    /// A fan fails.
    #[serde(default)]
    pub fan_fault: Option<bool>,
    /// A cache disk fails.
    #[serde(default)]
    pub cache_exception: Option<bool>,
    /// The NAS starts (not on every model).
    #[serde(default)]
    pub startup: Option<bool>,
    /// The NAS shuts down.
    #[serde(default)]
    pub shutdown: Option<bool>,
    /// A disk is too hot.
    #[serde(default)]
    pub too_hot: Option<bool>,
    /// Target NAS name or host. Required when multiple targets are configured.
    #[serde(default)]
    pub target: String,
}

/// A preset mode.
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ModeParam {
    /// The mode: for the CPU `high`, `balanced` or `save`; for the fan
    /// `quiet`, `auto` or `full`.
    pub mode: String,
    /// Target NAS name or host. Required when multiple targets are configured.
    #[serde(default)]
    pub target: String,
}

/// A custom fan curve.
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct FanCurveParam {
    /// 17 duties (20 to 100 %) for the device fan at 0, 5, ... 80 °C.
    pub sys: Vec<i64>,
    /// 17 duties for the CPU fan. From 55 °C the duty is 100 whatever is given.
    pub cpu: Vec<i64>,
    /// Target NAS name or host. Required when multiple targets are configured.
    #[serde(default)]
    pub target: String,
}

/// LED brightness.
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct LedParam {
    /// Weekdays, `1` Monday to `7` Sunday.
    pub days: Vec<i64>,
    /// Brightness in percent.
    pub percent: i64,
    /// Target NAS name or host. Required when multiple targets are configured.
    #[serde(default)]
    pub target: String,
}

/// The small display.
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ScreenParam {
    /// Brightness, 25 to 100.
    #[serde(default)]
    pub brightness: Option<i64>,
    /// Seconds until it goes dark: 15, 30, 60, 120, 300, or 0 for never.
    #[serde(default)]
    pub off_after: Option<i64>,
    /// A wallpaper path from the wallpaper list.
    #[serde(default)]
    pub wallpaper: Option<String>,
    /// A language key from the language list.
    #[serde(default)]
    pub language: Option<String>,
    /// Target NAS name or host. Required when multiple targets are configured.
    #[serde(default)]
    pub target: String,
}

/// Power settings.
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct PowerParam {
    /// Power on again after a power loss.
    #[serde(default)]
    pub auto_on: Option<bool>,
    /// Wake on LAN.
    #[serde(default)]
    pub wol: Option<bool>,
    /// When disks sleep: `never`, `10m`, `20m`, `30m`, `1h`, `2h` or `3h`.
    #[serde(default)]
    pub sleep: Option<String>,
    /// When USB disks sleep, like `sleep`.
    #[serde(default)]
    pub usb_sleep: Option<String>,
    /// What wakes a sleeping disk: `login` or `rw`.
    #[serde(default)]
    pub wake: Option<String>,
    /// Log the sleeping.
    #[serde(default)]
    pub sleep_log: Option<bool>,
    /// The power schedule on or off.
    #[serde(default)]
    pub schedule: Option<bool>,
    /// Target NAS name or host. Required when multiple targets are configured.
    #[serde(default)]
    pub target: String,
}

/// A power schedule entry.
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ScheduleParam {
    /// Weekdays, `1` Monday to `7` Sunday, comma separated, or `daily`,
    /// `weekdays`, `weekend`.
    pub days: String,
    /// Time as `HH:MM`.
    pub time: String,
    /// `on` powers the NAS on, `off` powers it off.
    pub action: String,
    /// Target NAS name or host. Required when multiple targets are configured.
    #[serde(default)]
    pub target: String,
}

/// The UPS.
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct UpsParam {
    /// A UPS on a USB port.
    #[serde(default)]
    pub usb: bool,
    /// Share the USB UPS with other NAS (NUT server).
    #[serde(default)]
    pub nut_server: bool,
    /// A UPS read by SNMP at this address.
    #[serde(default)]
    pub snmp: Option<String>,
    /// SNMP community.
    #[serde(default)]
    pub community: Option<String>,
    /// SNMP version, 1 or 2 (default 2).
    #[serde(default)]
    pub snmp_version: Option<i64>,
    /// SNMP MIB (default `auto`).
    #[serde(default)]
    pub mib: Option<String>,
    /// A UPS that another NAS serves, at this address.
    #[serde(default)]
    pub network: Option<String>,
    /// What to do on battery: `standby` or `shutdown` (default).
    #[serde(default)]
    pub action: Option<String>,
    /// When: `battery-low` (default), `now`, or a time like `5m` or `2h`.
    #[serde(default)]
    pub wait: Option<String>,
    /// Target NAS name or host. Required when multiple targets are configured.
    #[serde(default)]
    pub target: String,
}

/// A GPU mode.
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct GpuParam {
    /// The GPU id or name.
    pub gpu: String,
    /// `nas` or `vfio` (passthrough).
    pub mode: String,
    /// Accept the passthrough disclaimer, as the web UI asks the user to.
    #[serde(default)]
    pub accept_disclaimer: bool,
    /// Go on although the NAS restarts. Ask the user first.
    #[serde(default)]
    pub force: bool,
    /// Target NAS name or host. Required when multiple targets are configured.
    #[serde(default)]
    pub target: String,
}

/// Time settings.
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct TimeParam {
    /// A time zone from the zone list, e.g. `Europe/Berlin`.
    #[serde(default)]
    pub timezone: Option<String>,
    /// Keep the clock from this time server.
    #[serde(default)]
    pub server: Option<String>,
    /// Set the clock by hand, as `YYYY-MM-DD HH:MM:SS` in UTC.
    #[serde(default)]
    pub manual_utc: Option<String>,
    /// Go on although the cloud certificate may not be valid then.
    #[serde(default)]
    pub force: bool,
    /// Target NAS name or host. Required when multiple targets are configured.
    #[serde(default)]
    pub target: String,
}

/// SSH and Telnet.
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct TerminalParam {
    /// SSH on or off.
    #[serde(default)]
    pub ssh: Option<bool>,
    /// SSH port.
    #[serde(default)]
    pub ssh_port: Option<i64>,
    /// SFTP over SSH.
    #[serde(default)]
    pub sftp: Option<bool>,
    /// SSH encryption: `1` low or `3` high.
    #[serde(default)]
    pub ssh_encryption: Option<i64>,
    /// How long SSH stays on: `1` 10 min, `2` 30 min, `3` 1 h, `4` 2 h, `5` 3 h,
    /// `6` 6 h, `7` until switched off.
    #[serde(default)]
    pub ssh_duration: Option<i64>,
    /// Telnet on or off. Telnet sends passwords in the clear.
    #[serde(default)]
    pub telnet: Option<bool>,
    /// Telnet port.
    #[serde(default)]
    pub telnet_port: Option<i64>,
    /// Only from the local network, for both.
    #[serde(default)]
    pub lan_only: Option<bool>,
    /// Go on although this switches a service on. Ask the user first.
    #[serde(default)]
    pub force: bool,
    /// Target NAS name or host. Required when multiple targets are configured.
    #[serde(default)]
    pub target: String,
}

/// The finder switches.
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct FinderParam {
    /// Join the improvement plan.
    #[serde(default)]
    pub improvement: Option<bool>,
    /// Let the NAS be found from the web.
    #[serde(default)]
    pub web_find: Option<bool>,
    /// Target NAS name or host. Required when multiple targets are configured.
    #[serde(default)]
    pub target: String,
}

// ── Buzzer, CPU, fans, LEDs, display ────────────────────────────────

pub async fn buzzer_show(c: &UgosClient) -> String {
    json(basic::buzzer(c).await)
}

pub async fn buzzer_alarms(c: &UgosClient) -> String {
    json(basic::buzzer_alarms(c).await)
}

pub async fn buzzer_set(c: &UgosClient, p: &BuzzerParam) -> String {
    let change = BuzzerChange {
        fan_fault: p.fan_fault,
        cache_exception: p.cache_exception,
        system_startup: p.startup,
        system_shutdown: p.shutdown,
        temperature_too_high: p.too_hot,
    };
    json(basic::set_buzzer(c, &change).await)
}

pub async fn cpu_show(c: &UgosClient) -> String {
    json(basic::cpu(c).await)
}

pub async fn cpu_set(c: &UgosClient, p: &ModeParam) -> String {
    let run = async {
        let code = match p.mode.as_str() {
            "high" => 0,
            "balanced" => 1,
            "save" => 2,
            _ => return Err(refused("the CPU mode is high, balanced or save")),
        };
        basic::set_cpu(c, code).await
    };
    json(run.await)
}

pub async fn fan_show(c: &UgosClient) -> String {
    json(basic::fan(c).await)
}

pub async fn fan_set(c: &UgosClient, p: &ModeParam) -> String {
    let run = async {
        let code = match p.mode.as_str() {
            "quiet" => 1,
            "auto" => 2,
            "full" => 3,
            _ => return Err(refused("the fan mode is quiet, auto or full")),
        };
        basic::set_fan_mode(c, code).await
    };
    json(run.await)
}

pub async fn fan_curve(c: &UgosClient, p: &FanCurveParam) -> String {
    json(basic::set_fan_curve(c, &p.sys, &p.cpu).await)
}

pub async fn led_show(c: &UgosClient) -> String {
    json(basic::led(c).await)
}

pub async fn led_on(c: &UgosClient) -> String {
    json(basic::led_on(c).await)
}

pub async fn led_off(c: &UgosClient) -> String {
    json(basic::led_off(c).await)
}

pub async fn led_brightness(c: &UgosClient, p: &LedParam) -> String {
    json(basic::led_brightness(c, &p.days, p.percent).await)
}

pub async fn screen_show(c: &UgosClient) -> String {
    json(basic::screen(c).await)
}

pub async fn screen_wallpapers(c: &UgosClient) -> String {
    json(basic::wallpapers(c).await)
}

pub async fn screen_set(c: &UgosClient, p: &ScreenParam) -> String {
    let change = ScreenChange {
        brightness: p.brightness,
        screen_off_seconds: p.off_after,
        wallpaper_path: p.wallpaper.clone(),
        language_key: p.language.clone(),
    };
    if change == ScreenChange::default() {
        return "error: nothing to change".into();
    }
    json(basic::set_screen(c, &change).await)
}

pub async fn zram_show(c: &UgosClient) -> String {
    json(basic::zram(c).await)
}

pub async fn zram_on(c: &UgosClient) -> String {
    done(
        basic::zram_on(c)
            .await
            .map(|()| "Memory compression is on".into()),
    )
}

pub async fn zram_off(c: &UgosClient, p: &ForceParam) -> String {
    done(
        basic::zram_off(c, p.force)
            .await
            .map(|()| "Memory compression is off; the NAS restarts".into()),
    )
}

// ── Power and UPS ───────────────────────────────────────────────────

pub async fn power_show(c: &UgosClient) -> String {
    json(power::power(c).await)
}

pub async fn power_set(c: &UgosClient, p: &PowerParam) -> String {
    let run = async {
        let change = PowerChange {
            power_boot: p.auto_on,
            wake_on: p.wol,
            sleep: p.sleep.as_deref().map(parse::sleep).transpose()?,
            usb_sleep: p.usb_sleep.as_deref().map(parse::sleep).transpose()?,
            strategy: match p.wake.as_deref() {
                None => None,
                Some("login") => Some(1),
                Some("rw") => Some(2),
                Some(_) => return Err(refused("wake is login or rw")),
            },
            sleep_log: p.sleep_log,
            schedule: p.schedule,
        };
        power::set_power(c, &change).await
    };
    json(run.await)
}

pub async fn schedule_add(c: &UgosClient, p: &ScheduleParam) -> String {
    let run = async {
        let action = match p.action.as_str() {
            "on" => "open",
            "off" => "close",
            _ => return Err(refused("the action is on or off")),
        };
        power::add_switch(c, power::switch(&parse::days(&p.days)?, &p.time, action)?).await
    };
    json(run.await)
}

pub async fn schedule_remove(c: &UgosClient, p: &PositionParam) -> String {
    json(power::remove_switch(c, p.position).await)
}

pub async fn ups_show(c: &UgosClient) -> String {
    json(ups::ups(c).await)
}

fn ups_setting(p: &UpsParam) -> Result<UpsSetting, UgosError> {
    let source = match (&p.snmp, &p.network, p.usb) {
        (Some(ip), None, false) => Source::Snmp {
            ip: ip.clone(),
            community: p.community.clone().unwrap_or_default(),
            version: p.snmp_version.unwrap_or(2),
            mib: p.mib.clone().unwrap_or_else(|| "auto".into()),
        },
        (None, Some(master_ip), false) => Source::Network {
            master_ip: master_ip.clone(),
        },
        (None, None, true) => Source::Usb {
            nut_server: p.nut_server,
        },
        _ => return Err(refused("give one of usb, snmp and network")),
    };
    let protect_type = match p.action.as_deref() {
        None | Some("shutdown") => 1,
        Some("standby") => 0,
        Some(_) => return Err(refused("the action is standby or shutdown")),
    };
    Ok(UpsSetting {
        source,
        protect_type,
        wait: parse::wait(p.wait.as_deref().unwrap_or("battery-low"))?,
    })
}

pub async fn ups_set(c: &UgosClient, p: &UpsParam) -> String {
    let run = async { ups::set_ups(c, &ups_setting(p)?).await };
    json(run.await)
}

pub async fn ups_disconnect(c: &UgosClient) -> String {
    done(
        ups::disconnect(c)
            .await
            .map(|()| "The UPS is disconnected".into()),
    )
}

pub async fn ups_check(c: &UgosClient, ip: &str) -> String {
    done(
        ups::check_master(c, ip)
            .await
            .map(|()| format!("A NUT server answers at {ip}")),
    )
}

pub async fn ups_mibs(c: &UgosClient) -> String {
    json(ups::mibs(c).await)
}

pub async fn ups_firmware(c: &UgosClient) -> String {
    json(ups::firmware(c).await)
}

pub async fn gpu_list(c: &UgosClient) -> String {
    json(gpu::gpus(c).await)
}

pub async fn gpu_mode(c: &UgosClient, p: &GpuParam) -> String {
    let run = async {
        let mode = match p.mode.as_str() {
            "nas" => GpuMode::Nas,
            "vfio" => GpuMode::Vfio,
            _ => return Err(refused("the GPU mode is nas or vfio")),
        };
        gpu::set_mode(c, &p.gpu, mode, p.accept_disclaimer, p.force)
            .await
            .map(|()| "The GPU mode is set; the NAS restarts".to_owned())
    };
    done(run.await)
}

// ── Time, language, terminal, host name ─────────────────────────────

pub async fn time_show(c: &UgosClient) -> String {
    json(clock::time(c).await)
}

pub async fn time_zones(c: &UgosClient) -> String {
    json(clock::time(c).await.map(|t| t.zones))
}

pub async fn time_set(c: &UgosClient, p: &TimeParam) -> String {
    let run = async {
        let mode = match (&p.server, &p.manual_utc) {
            (Some(host), None) => Some(ClockMode::Server(host.clone())),
            (None, Some(text)) => Some(ClockMode::Manual(parse::unix_utc(text)?)),
            (Some(_), Some(_)) => return Err(refused("give a server or a manual time, not both")),
            (None, None) => None,
        };
        clock::set_time(
            c,
            &TimeChange {
                timezone: p.timezone.clone(),
                mode,
            },
            p.force,
        )
        .await
    };
    json(run.await)
}

pub async fn time_servers(c: &UgosClient) -> String {
    json(clock::servers(c).await)
}

pub async fn time_server_add(c: &UgosClient, host: &str) -> String {
    json(clock::add_server(c, host).await)
}

pub async fn time_server_remove(c: &UgosClient, host: &str) -> String {
    json(clock::remove_server(c, host).await)
}

pub async fn lang_show(c: &UgosClient) -> String {
    json(clock::language(c).await)
}

pub async fn lang_list(c: &UgosClient) -> String {
    json(clock::languages(c).await)
}

pub async fn lang_set(c: &UgosClient, key: &str) -> String {
    json(clock::set_language(c, key).await)
}

pub async fn terminal_show(c: &UgosClient) -> String {
    json(
        terminal::terminal(c)
            .await
            .map(|t| serde_json::json!({ "ssh": t.ssh, "telnet": t.telnet })),
    )
}

pub async fn terminal_set(c: &UgosClient, p: &TerminalParam) -> String {
    let change = TerminalChange {
        ssh: p.ssh,
        ssh_port: p.ssh_port,
        sftp: p.sftp,
        ssh_encryption: p.ssh_encryption,
        ssh_duration: p.ssh_duration,
        telnet: p.telnet,
        telnet_port: p.telnet_port,
        lan_only: p.lan_only,
    };
    json(
        terminal::set_terminal(c, &change, p.force)
            .await
            .map(|t| serde_json::json!({ "ssh": t.ssh, "telnet": t.telnet })),
    )
}

pub async fn hostname_set(c: &UgosClient, name: &str) -> String {
    json(clock::set_hostname(c, name).await)
}

pub async fn finder_show(c: &UgosClient) -> String {
    json(clock::finder(c).await)
}

pub async fn finder_set(c: &UgosClient, p: &FinderParam) -> String {
    json(clock::set_finder(c, p.improvement, p.web_find).await)
}
