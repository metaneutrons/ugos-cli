//! Rows for the file services.

use serde::Serialize;
use serde_json::Value;
use tabled::Tabled;
use ugos_client::ops::fileserver::{Config, Field, Kind, Service};
use ugos_client::types::shares::SmbAdvanced;

use super::{FieldRow, field, yes_no};

fn shown(f: &Field, value: Option<&Value>) -> String {
    let Some(value) = value else {
        return "-".into();
    };
    match (f.kind, value) {
        (Kind::Bool, Value::Bool(b)) => yes_no(*b),
        (Kind::Code(names), v) => v
            .as_i64()
            .and_then(|n| names.iter().find(|(_, code)| *code == n))
            .map_or_else(|| v.to_string(), |(name, _)| (*name).to_owned()),
        (Kind::Protocols, Value::Array(list)) => list
            .iter()
            .filter_map(Value::as_i64)
            .map(|n| if n == 1 { "smb1" } else { "smb2" })
            .collect::<Vec<_>>()
            .join(", "),
        (_, Value::String(s)) if s.is_empty() => "-".into(),
        (_, Value::String(s)) => s.clone(),
        (_, v) => v.to_string(),
    }
}

/// A service and its settings.
pub fn config_rows(service: Service, config: &Config) -> Vec<FieldRow> {
    let state = if service.running(config) {
        "running"
    } else {
        "stopped"
    };
    let mut rows = vec![field("Service", state)];
    if service == Service::Smb {
        let text = |key: &str| {
            config
                .get(key)
                .and_then(Value::as_str)
                .unwrap_or("-")
                .to_owned()
        };
        rows.push(field("Server name", text("nas_name")));
        rows.push(field(
            "Joined to a domain",
            yes_no(config.get("is_joined_domain").and_then(Value::as_bool) == Some(true)),
        ));
    }
    for f in service.fields() {
        rows.push(field(f.alias, shown(f, config.get(f.key))));
    }
    rows
}

/// The discovery services and Time Machine targets.
pub fn advanced_rows(advanced: &SmbAdvanced) -> Vec<FieldRow> {
    let targets: Vec<&str> = advanced
        .shares
        .iter()
        .filter(|s| s.time_backup)
        .map(|s| s.name.as_str())
        .collect();
    vec![
        field("WS-Discovery (Windows)", yes_no(advanced.wsdd2)),
        field("Bonjour (Macs)", yes_no(advanced.bonjour)),
        field("UPnP", yes_no(advanced.upnp)),
        field(
            "Time Machine targets",
            if targets.is_empty() {
                "none".to_owned()
            } else {
                targets.join(", ")
            },
        ),
    ]
}

/// A setting and what it takes.
#[derive(Serialize)]
pub struct SettingRow {
    pub alias: String,
    pub takes: String,
    pub help: String,
}

table_row! {
  SettingRow {
    alias => "Setting",
    takes => "Takes",
    help => "Meaning",
  }
}

fn takes(kind: Kind) -> String {
    match kind {
        Kind::Bool => "on, off".into(),
        Kind::Int(low, high) => format!("{low} to {high}"),
        Kind::Text(max) => format!("text, up to {max}"),
        Kind::Code(names) => names.iter().map(|(n, _)| *n).collect::<Vec<_>>().join(", "),
        Kind::Word(words) => words.join(", "),
        Kind::Protocols => "smb1, smb2, smb1,smb2".into(),
        Kind::Addresses => "IP addresses, comma separated".into(),
        Kind::Veto => "/name/other/".into(),
        Kind::Workgroup => "up to 15 characters".into(),
    }
}

/// The settings of a service.
pub fn setting_rows(service: Service) -> Vec<SettingRow> {
    service
        .fields()
        .iter()
        .map(|f| SettingRow {
            alias: f.alias.into(),
            takes: takes(f.kind),
            help: f.help.into(),
        })
        .collect()
}
