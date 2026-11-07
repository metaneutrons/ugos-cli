//! The FTP service. FTP and FTPS are one service with two switches.

use serde_json::Value;

use super::fields::{Field, Kind};
use super::{Config, PortUse, flag, number, refused};
use crate::error::{Result, UgosError};

/// The settings. The ones the web UI shows no control for are left out.
pub const FIELDS: &[Field] = &[
    Field::flag(
        "ftp",
        "enable_ftp",
        "plain FTP, which sends passwords in the clear",
    ),
    Field::flag("ftps", "enable_ftps", "FTP over TLS"),
    Field::of("port", "port", Kind::Int(1, 65535), "control port"),
    Field::flag("log", "enable_log", "keep a log"),
    Field::of(
        "timeout",
        "timeout",
        Kind::Int(1, 7200),
        "idle timeout in seconds",
    ),
    Field::flag("utf8", "enable_utf8", "UTF-8 file names"),
    Field::flag("passive", "enable_pasv_port", "limit the passive ports"),
    Field::of(
        "passive-min",
        "pasv_min_port",
        Kind::Int(1024, 65535),
        "lowest passive port",
    ),
    Field::of(
        "passive-max",
        "pasv_max_port",
        Kind::Int(1024, 65535),
        "highest passive port",
    ),
    Field::flag(
        "limit-per-ip",
        "enable_max_per_ip",
        "limit connections per address",
    ),
    Field::of(
        "per-ip",
        "max_per_ip",
        Kind::Int(2, 10),
        "connections per address",
    ),
    Field::flag("limit-rate", "enable_rate", "limit the transfer rate"),
    Field::of(
        "upload-rate",
        "max_upload_rate",
        Kind::Int(1, 999_999),
        "upload limit in KB/s",
    ),
    Field::of(
        "download-rate",
        "max_download_rate",
        Kind::Int(1, 999_999),
        "download limit in KB/s",
    ),
];

/// The passive range: both ends or neither, in order.
fn passive_range(current: &Config, next: &mut Config) -> Result<()> {
    if !flag(next, "enable_pasv_port") {
        for key in ["pasv_min_port", "pasv_max_port"] {
            let old = current.get(key).cloned().unwrap_or_else(|| Value::from(0));
            let _ = next.insert(key.into(), old);
        }
        return Ok(());
    }
    let (low, high) = (number(next, "pasv_min_port"), number(next, "pasv_max_port"));
    if low == 0 || high == 0 {
        return Err(refused(
            "the passive range needs passive-min and passive-max",
        ));
    }
    if low > high {
        let _ = next.insert("pasv_min_port".into(), Value::from(high));
        let _ = next.insert("pasv_max_port".into(), Value::from(low));
    }
    Ok(())
}

pub(super) fn finish(
    current: &Config,
    next: &mut Config,
    enable: bool,
    force: bool,
) -> Result<Vec<PortUse>> {
    if enable && !flag(next, "enable_ftp") && !flag(next, "enable_ftps") {
        // Starting without a choice means the secure one.
        let _ = next.insert("enable_ftps".into(), Value::Bool(true));
    }
    if !force && flag(next, "enable_ftp") && !flag(current, "enable_ftp") {
        return Err(UgosError::NeedsForce(
            "plain FTP sends names and passwords in the clear; FTPS does not".into(),
        ));
    }
    passive_range(current, next)?;
    if flag(next, "enable_rate")
        && (number(next, "max_upload_rate") <= 0 || number(next, "max_download_rate") <= 0)
    {
        return Err(refused(
            "limiting the rate needs upload-rate and download-rate above 0",
        ));
    }
    let port = match number(next, "port") {
        0 => 21,
        p => p,
    };
    let _ = next.insert("port".into(), Value::from(port));
    Ok(vec![PortUse {
        module: "ftp",
        key: "port",
        name: None,
        port,
    }])
}
