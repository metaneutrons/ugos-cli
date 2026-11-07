//! `hw`: buzzer, CPU mode, fans, LEDs, the small display, memory compression
//! and GPUs. Power and the UPS are in `hw_power`.

use std::io::Write;

use anyhow::{Result, bail};
use serde::Serialize;
use ugos_client::UgosClient;
use ugos_client::ops::hw::basic::{
    BuzzerChange, ScreenChange, buzzer, buzzer_alarms, cpu, fan, led, led_brightness, led_off,
    led_on, screen, set_buzzer, set_cpu, set_fan_curve, set_fan_mode, set_screen, wallpapers, zram,
    zram_off, zram_on,
};
use ugos_client::ops::hw::gpu::{GpuMode, gpus, set_mode};
use ugos_client::ops::hw::parse;

use super::hw_power;
use super::share::with_force_hint;
use crate::cli::{
    BuzzerAction, CpuAction, CpuMode, FanAction, FanMode, GpuAction, GpuModeArg, HwAction,
    LedAction, OnOff, OutputFormat, ScreenAction, ZramAction,
};
use crate::output;
use crate::output::hw as rows;

/// Print rows of a detail view, or the value as JSON.
pub(super) fn show<T: Serialize>(
    w: &mut impl Write,
    fmt: OutputFormat,
    value: &T,
    table: &[output::FieldRow],
) -> Result<()> {
    match fmt {
        OutputFormat::Table => output::print_list(w, table, fmt),
        OutputFormat::Json => output::print_json(w, value),
    }
}

/// Run a `hw` subcommand.
pub async fn hw(
    client: &UgosClient,
    action: &HwAction,
    fmt: OutputFormat,
    w: &mut impl Write,
) -> Result<()> {
    match action {
        HwAction::Buzzer { action } => buzzer_cmd(client, action, fmt, w).await,
        HwAction::Cpu { action } => cpu_cmd(client, action, fmt, w).await,
        HwAction::Fan { action } => fan_cmd(client, action, fmt, w).await,
        HwAction::Led { action } => led_cmd(client, action, fmt, w).await,
        HwAction::Screen { action } => screen_cmd(client, action, fmt, w).await,
        HwAction::Power { action } => hw_power::power(client, action, fmt, w).await,
        HwAction::Ups { action } => hw_power::ups_cmd(client, action, fmt, w).await,
        HwAction::Zram { action } => zram_cmd(client, action, fmt, w).await,
        HwAction::Gpu { action } => gpu_cmd(client, action, fmt, w).await,
    }
}

fn on(value: Option<OnOff>) -> Option<bool> {
    value.map(OnOff::flag)
}

async fn buzzer_cmd(
    client: &UgosClient,
    action: &BuzzerAction,
    fmt: OutputFormat,
    w: &mut impl Write,
) -> Result<()> {
    let now = match action {
        BuzzerAction::Show => buzzer(client).await?,
        BuzzerAction::Alarms => {
            let alarms = buzzer_alarms(client).await?;
            return output::print_json(w, &alarms);
        }
        BuzzerAction::Set {
            fan_fault,
            cache_exception,
            startup,
            shutdown,
            too_hot,
        } => {
            let change = BuzzerChange {
                fan_fault: on(*fan_fault),
                cache_exception: on(*cache_exception),
                system_startup: on(*startup),
                system_shutdown: on(*shutdown),
                temperature_too_high: on(*too_hot),
            };
            set_buzzer(client, &change).await?
        }
    };
    show(w, fmt, &now, &rows::buzzer_rows(&now))
}

async fn cpu_cmd(
    client: &UgosClient,
    action: &CpuAction,
    fmt: OutputFormat,
    w: &mut impl Write,
) -> Result<()> {
    let now = match action {
        CpuAction::Show => cpu(client).await?,
        CpuAction::Set { mode } => {
            let code = match mode {
                CpuMode::High => 0,
                CpuMode::Balanced => 1,
                CpuMode::Save => 2,
            };
            set_cpu(client, code).await?
        }
    };
    show(w, fmt, &now, &rows::cpu_rows(&now))
}

async fn fan_cmd(
    client: &UgosClient,
    action: &FanAction,
    fmt: OutputFormat,
    w: &mut impl Write,
) -> Result<()> {
    let now = match action {
        FanAction::Show => fan(client).await?,
        FanAction::Set { mode } => {
            let code = match mode {
                FanMode::Quiet => 1,
                FanMode::Auto => 2,
                FanMode::Full => 3,
            };
            set_fan_mode(client, code).await?
        }
        FanAction::Curve { sys, cpu } => {
            set_fan_curve(client, &parse::numbers(sys)?, &parse::numbers(cpu)?).await?
        }
    };
    show(w, fmt, &now, &rows::fan_rows(&now))
}

async fn led_cmd(
    client: &UgosClient,
    action: &LedAction,
    fmt: OutputFormat,
    w: &mut impl Write,
) -> Result<()> {
    let now = match action {
        LedAction::Show => led(client).await?,
        LedAction::On => led_on(client).await?,
        LedAction::Off => led_off(client).await?,
        LedAction::Brightness { days, percent } => {
            led_brightness(client, &parse::numbers(days)?, *percent).await?
        }
    };
    show(w, fmt, &now, &rows::led_rows(&now))
}

async fn screen_cmd(
    client: &UgosClient,
    action: &ScreenAction,
    fmt: OutputFormat,
    w: &mut impl Write,
) -> Result<()> {
    let now = match action {
        ScreenAction::Show => screen(client).await?,
        ScreenAction::Wallpapers => {
            let list = wallpapers(client).await?;
            return output::print_json(w, &list);
        }
        ScreenAction::Set {
            brightness,
            off_after,
            wallpaper,
            language,
        } => {
            let change = ScreenChange {
                brightness: *brightness,
                screen_off_seconds: *off_after,
                wallpaper_path: wallpaper.clone(),
                language_key: language.clone(),
            };
            if change == ScreenChange::default() {
                bail!("nothing to change: see `hw screen set --help`");
            }
            set_screen(client, &change).await?
        }
    };
    show(w, fmt, &now, &rows::screen_rows(&now))
}

async fn zram_cmd(
    client: &UgosClient,
    action: &ZramAction,
    fmt: OutputFormat,
    w: &mut impl Write,
) -> Result<()> {
    match action {
        ZramAction::Show => {}
        ZramAction::On => zram_on(client).await?,
        ZramAction::Off { force } => {
            zram_off(client, *force).await.map_err(with_force_hint)?;
            return output::print_success(w, "Memory compression is off; the NAS restarts", fmt);
        }
    }
    let state = zram(client).await?;
    output::print_success(
        w,
        &format!("Memory compression is {}", if state { "on" } else { "off" }),
        fmt,
    )
}

async fn gpu_cmd(
    client: &UgosClient,
    action: &GpuAction,
    fmt: OutputFormat,
    w: &mut impl Write,
) -> Result<()> {
    match action {
        GpuAction::List => {
            let list = gpus(client).await?;
            match fmt {
                OutputFormat::Table => {
                    let table: Vec<rows::GpuRow> = list.iter().map(Into::into).collect();
                    output::print_list(w, &table, fmt)
                }
                OutputFormat::Json => output::print_json(w, &list),
            }
        }
        GpuAction::Mode {
            gpu,
            mode,
            accept_disclaimer,
            force,
        } => {
            let target = match mode {
                GpuModeArg::Nas => GpuMode::Nas,
                GpuModeArg::Vfio => GpuMode::Vfio,
            };
            set_mode(client, gpu, target, *accept_disclaimer, *force)
                .await
                .map_err(with_force_hint)?;
            output::print_success(w, "The GPU mode is set; the NAS restarts", fmt)
        }
    }
}
