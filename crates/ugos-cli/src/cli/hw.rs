//! Arguments of the hardware, power, time, language and terminal commands.

use clap::{Args, Subcommand, ValueEnum};

/// On or off.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum OnOff {
    /// On.
    On,
    /// Off.
    Off,
}

impl OnOff {
    /// As a flag.
    #[must_use]
    pub const fn flag(self) -> bool {
        matches!(self, Self::On)
    }
}

/// `hw` subcommands: the hardware of the NAS.
#[derive(Debug, Subcommand)]
pub enum HwAction {
    /// The buzzer.
    Buzzer {
        #[command(subcommand)]
        action: BuzzerAction,
    },
    /// The CPU performance mode.
    Cpu {
        #[command(subcommand)]
        action: CpuAction,
    },
    /// The fans.
    Fan {
        #[command(subcommand)]
        action: FanAction,
    },
    /// The status LEDs.
    Led {
        #[command(subcommand)]
        action: LedAction,
    },
    /// The small display of some models.
    Screen {
        #[command(subcommand)]
        action: ScreenAction,
    },
    /// Sleep, power on after a loss, Wake on LAN and the power schedule.
    Power {
        #[command(subcommand)]
        action: PowerAction,
    },
    /// The UPS.
    Ups {
        #[command(subcommand)]
        action: UpsAction,
    },
    /// Memory compression.
    Zram {
        #[command(subcommand)]
        action: ZramAction,
    },
    /// Dedicated GPUs.
    Gpu {
        #[command(subcommand)]
        action: GpuAction,
    },
}

/// Buzzer subcommands.
#[derive(Debug, Subcommand)]
pub enum BuzzerAction {
    /// Show which events sound the buzzer.
    Show,
    /// Show which alarm is sounding now: fan, hdd, temp.
    Alarms,
    /// Choose the events that sound it.
    Set {
        /// A fan fails.
        #[arg(long, value_enum)]
        fan_fault: Option<OnOff>,
        /// A cache disk fails.
        #[arg(long, value_enum)]
        cache_exception: Option<OnOff>,
        /// The NAS starts (not on every model).
        #[arg(long, value_enum)]
        startup: Option<OnOff>,
        /// The NAS shuts down.
        #[arg(long, value_enum)]
        shutdown: Option<OnOff>,
        /// A disk is too hot.
        #[arg(long, value_enum)]
        too_hot: Option<OnOff>,
    },
}

/// CPU mode subcommands.
#[derive(Debug, Subcommand)]
pub enum CpuAction {
    /// Show the mode.
    Show,
    /// Set the mode.
    Set {
        /// `high`, `balanced` or `save`.
        #[arg(value_enum)]
        mode: CpuMode,
    },
}

/// CPU performance modes.
#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum CpuMode {
    /// High performance.
    High,
    /// Balanced.
    Balanced,
    /// Power saving.
    Save,
}

/// Fan subcommands.
#[derive(Debug, Subcommand)]
pub enum FanAction {
    /// Show the mode and the curves.
    Show,
    /// Set a preset: `quiet`, `auto` or `full`.
    Set {
        /// The preset.
        #[arg(value_enum)]
        mode: FanMode,
    },
    /// Set a custom curve from 17 duties for 0, 5, ... 80 °C (20 to 100 %).
    /// From 55 °C the duty is 100 whatever is given.
    Curve {
        /// Duties for the device fan, comma separated.
        #[arg(long)]
        sys: String,
        /// Duties for the CPU fan, comma separated.
        #[arg(long)]
        cpu: String,
    },
}

/// Fan presets.
#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum FanMode {
    /// Quiet.
    Quiet,
    /// Automatic.
    Auto,
    /// Full speed.
    Full,
}

/// LED subcommands.
#[derive(Debug, Subcommand)]
pub enum LedAction {
    /// Show the LEDs and their schedule.
    Show,
    /// Switch the LEDs on.
    On,
    /// Switch the LEDs off.
    Off,
    /// Set the brightness for weekdays, all day.
    Brightness {
        /// Weekdays, `1` Monday to `7` Sunday, comma separated.
        #[arg(long)]
        days: String,
        /// Brightness in percent.
        #[arg(long)]
        percent: i64,
    },
}

/// Small display subcommands.
#[derive(Debug, Subcommand)]
pub enum ScreenAction {
    /// Show the settings.
    Show,
    /// List the wallpapers.
    Wallpapers,
    /// Change the settings.
    Set {
        /// Brightness, 25 to 100.
        #[arg(long)]
        brightness: Option<i64>,
        /// Seconds until it goes dark: 15, 30, 60, 120, 300, or 0 for never.
        #[arg(long)]
        off_after: Option<i64>,
        /// A wallpaper path from `hw screen wallpapers`.
        #[arg(long)]
        wallpaper: Option<String>,
        /// A language key from `lang list`.
        #[arg(long)]
        language: Option<String>,
    },
}

/// Power subcommands.
#[derive(Debug, Subcommand)]
pub enum PowerAction {
    /// Show the settings and the schedule.
    Show,
    /// Change the settings.
    Set(PowerSetArgs),
    /// The power schedule.
    Schedule {
        #[command(subcommand)]
        action: ScheduleAction,
    },
}

/// What to set about power.
#[derive(Debug, Args)]
pub struct PowerSetArgs {
    /// Power on again after a power loss.
    #[arg(long, value_enum)]
    pub auto_on: Option<OnOff>,
    /// Wake on LAN.
    #[arg(long, value_enum)]
    pub wol: Option<OnOff>,
    /// When disks sleep: `never`, `10m`, `20m`, `30m`, `1h`, `2h` or `3h`.
    #[arg(long)]
    pub sleep: Option<String>,
    /// When USB disks sleep, like `--sleep`.
    #[arg(long)]
    pub usb_sleep: Option<String>,
    /// Wake a sleeping disk on `login` or on `rw` (read or write).
    #[arg(long, value_enum)]
    pub wake: Option<Wake>,
    /// Log the sleeping.
    #[arg(long, value_enum)]
    pub sleep_log: Option<OnOff>,
    /// The power schedule.
    #[arg(long, value_enum)]
    pub schedule: Option<OnOff>,
}

/// What wakes a sleeping disk.
#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum Wake {
    /// A login.
    Login,
    /// A read or a write.
    Rw,
}

/// Power schedule subcommands.
#[derive(Debug, Subcommand)]
pub enum ScheduleAction {
    /// Add an entry and switch the schedule on.
    Add {
        /// Weekdays, `1` Monday to `7` Sunday, comma separated, or `daily`,
        /// `weekdays`, `weekend`.
        #[arg(long)]
        days: String,
        /// Time as `HH:MM`.
        #[arg(long)]
        time: String,
        /// `on` powers the NAS on, `off` powers it off.
        #[arg(long, value_enum)]
        action: OnOff,
    },
    /// Remove the entry at this position, counting from 1.
    Rm {
        /// The position.
        position: usize,
    },
}

/// UPS subcommands.
#[derive(Debug, Subcommand)]
pub enum UpsAction {
    /// Show the UPS.
    Show,
    /// Connect a UPS or change its setting.
    Set(UpsSetArgs),
    /// Disconnect the UPS.
    Disconnect,
    /// List the MIBs an SNMP UPS can use.
    Mibs,
    /// Show the firmware state of a USB UPS.
    Firmware,
    /// Check that a NUT server answers.
    Check {
        /// Its address.
        ip: String,
    },
}

/// What to set about the UPS.
#[derive(Debug, Args)]
pub struct UpsSetArgs {
    /// A UPS on a USB port.
    #[arg(long, group = "source")]
    pub usb: bool,
    /// Share the USB UPS with other NAS (NUT server).
    #[arg(long, requires = "usb")]
    pub nut_server: bool,
    /// A UPS read by SNMP at this address.
    #[arg(long, group = "source", requires = "community")]
    pub snmp: Option<String>,
    /// SNMP community.
    #[arg(long)]
    pub community: Option<String>,
    /// SNMP version, 1 or 2.
    #[arg(long, default_value_t = 2)]
    pub snmp_version: i64,
    /// SNMP MIB.
    #[arg(long, default_value = "auto")]
    pub mib: String,
    /// A UPS that another NAS serves, at this address.
    #[arg(long, group = "source")]
    pub network: Option<String>,
    /// What to do on battery: `standby` or `shutdown`.
    #[arg(long, value_enum, default_value = "shutdown")]
    pub action: UpsActionKind,
    /// When: `battery-low`, `now`, or a time like `5m` or `2h`.
    #[arg(long, default_value = "battery-low")]
    pub wait: String,
}

/// What the UPS makes the NAS do.
#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum UpsActionKind {
    /// Go to standby.
    Standby,
    /// Shut down.
    Shutdown,
}

/// Memory compression subcommands.
#[derive(Debug, Subcommand)]
pub enum ZramAction {
    /// Show whether it is on.
    Show,
    /// Switch it on.
    On,
    /// Switch it off. The NAS restarts.
    Off {
        /// Go on although the NAS restarts.
        #[arg(long)]
        force: bool,
    },
}

/// GPU subcommands.
#[derive(Debug, Subcommand)]
pub enum GpuAction {
    /// List the GPUs.
    List,
    /// Switch a GPU between the NAS and a VM. The NAS restarts.
    Mode {
        /// The GPU id or name.
        gpu: String,
        /// `nas` or `vfio` (passthrough).
        #[arg(value_enum)]
        mode: GpuModeArg,
        /// Accept the passthrough disclaimer, as the web UI asks.
        #[arg(long)]
        accept_disclaimer: bool,
        /// Go on although the NAS restarts.
        #[arg(long)]
        force: bool,
    },
}

/// Where a GPU works.
#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum GpuModeArg {
    /// For the NAS.
    Nas,
    /// For a VM, by passthrough.
    Vfio,
}

/// `time` subcommands.
#[derive(Debug, Subcommand)]
pub enum TimeAction {
    /// Show time, time zone and how the clock is kept.
    Show,
    /// List the time zones.
    Zones,
    /// Change the time zone or how the clock is kept.
    Set {
        /// A time zone from `time zones`, e.g. `Europe/Berlin`.
        #[arg(long)]
        timezone: Option<String>,
        /// Keep the clock from this time server.
        #[arg(long, conflicts_with = "manual_utc")]
        server: Option<String>,
        /// Set the clock by hand, as `YYYY-MM-DD HH:MM:SS` in UTC.
        #[arg(long)]
        manual_utc: Option<String>,
        /// Go on although the cloud certificate may not be valid then.
        #[arg(long)]
        force: bool,
    },
    /// The list of time servers.
    Server {
        #[command(subcommand)]
        action: ServerAction,
    },
}

/// Time server subcommands.
#[derive(Debug, Subcommand)]
pub enum ServerAction {
    /// List them.
    List,
    /// Add one.
    Add {
        /// The host.
        host: String,
    },
    /// Remove one.
    Rm {
        /// The host.
        host: String,
    },
}

/// `lang` subcommands: the default language of the NAS.
#[derive(Debug, Subcommand)]
pub enum LangAction {
    /// Show the default language.
    Show,
    /// List the languages.
    List,
    /// Set the default language: a key from the list, or `auto`.
    Set {
        /// The key.
        key: String,
    },
}

/// `terminal` subcommands: SSH and Telnet.
#[derive(Debug, Subcommand)]
pub enum TerminalAction {
    /// Show SSH and Telnet.
    Show,
    /// Change them. Switching either on needs `--force`.
    Set(TerminalSetArgs),
}

/// What to set about SSH and Telnet.
#[derive(Debug, Args)]
pub struct TerminalSetArgs {
    /// SSH on or off.
    #[arg(long, value_enum)]
    pub ssh: Option<OnOff>,
    /// SSH port.
    #[arg(long)]
    pub ssh_port: Option<i64>,
    /// SFTP over SSH.
    #[arg(long, value_enum)]
    pub sftp: Option<OnOff>,
    /// SSH encryption: `1` low or `3` high.
    #[arg(long)]
    pub ssh_encryption: Option<i64>,
    /// How long SSH stays on: `1` 10 min, `2` 30 min, `3` 1 h, `4` 2 h,
    /// `5` 3 h, `6` 6 h, `7` until switched off.
    #[arg(long)]
    pub ssh_duration: Option<i64>,
    /// Telnet on or off. Telnet sends passwords in the clear.
    #[arg(long, value_enum)]
    pub telnet: Option<OnOff>,
    /// Telnet port.
    #[arg(long)]
    pub telnet_port: Option<i64>,
    /// Only from the local network, for both.
    #[arg(long, value_enum)]
    pub lan_only: Option<OnOff>,
    /// Go on although this switches a service on.
    #[arg(long)]
    pub force: bool,
}
