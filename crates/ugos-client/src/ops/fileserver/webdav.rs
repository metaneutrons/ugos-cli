//! The `WebDAV` service.

use serde_json::Value;

use super::fields::{Field, Kind};
use super::{Config, PortUse, flag, number, refused};
use crate::error::Result;

/// The settings. `enable_anonymous` has no control in the web UI.
pub const FIELDS: &[Field] = &[
    Field::flag("http", "enable_http", "serve over HTTP"),
    Field::flag("https", "enable_https", "serve over HTTPS"),
    Field::of(
        "http-port",
        "http_port",
        Kind::Int(1, 65535),
        "HTTP port (5005 when unset)",
    ),
    Field::of(
        "https-port",
        "https_port",
        Kind::Int(1, 65535),
        "HTTPS port (5006 when unset)",
    ),
    Field::flag("log", "enable_webdav_log", "send WebDAV events to the log"),
    Field::flag(
        "no-path-limit",
        "disable_path_len_constraint",
        "lift the path length limit",
    ),
];

pub(super) fn finish(current: &Config, next: &mut Config) -> Result<Vec<PortUse>> {
    let _ = current;
    let (http, https) = (flag(next, "enable_http"), flag(next, "enable_https"));
    if !http && !https {
        return Err(refused("WebDAV needs HTTP or HTTPS"));
    }
    for (key, default) in [("http_port", 5005), ("https_port", 5006)] {
        if number(next, key) == 0 {
            let _ = next.insert(key.into(), Value::from(default));
        }
    }
    if http && https && number(next, "http_port") == number(next, "https_port") {
        return Err(refused("the HTTP and the HTTPS port must differ"));
    }
    let mut ports = Vec::new();
    if http {
        ports.push(PortUse {
            module: "webdav",
            key: "http_port",
            name: None,
            port: number(next, "http_port"),
        });
    }
    if https {
        ports.push(PortUse {
            module: "webdav",
            key: "https_port",
            name: None,
            port: number(next, "https_port"),
        });
    }
    Ok(ports)
}
