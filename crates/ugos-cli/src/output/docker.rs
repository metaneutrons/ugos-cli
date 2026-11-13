//! Rows for the Docker commands: containers with their figures, the Docker
//! app's own log.

use serde::Serialize;
use tabled::Tabled;
use ugos_client::ops::dockerupdates::UpdateStatus;
use ugos_client::ops::dockerview::ContainerView;
use ugos_client::types::docker::{ContainerStats, Network, NetworkMember, OpLog};

use super::{FieldRow, field, format_bytes, format_unix, yes_no};

/// A container in `docker ps`.
#[derive(Serialize)]
pub struct ContainerRow {
    pub id: String,
    pub name: String,
    pub image: String,
    pub status: String,
    pub cpu: String,
    pub memory: String,
    pub ports: String,
    pub update: String,
}

table_row! {
  ContainerRow {
    id => "ID",
    name => "Name",
    image => "Image",
    status => "Status",
    cpu => "CPU%",
    memory => "Memory",
    ports => "Ports",
    update => "Update",
  }
}

/// The ports as `nas->container/proto`, the way a `docker ps` shows them.
fn ports(stats: &ContainerStats) -> String {
    stats
        .port_mapping
        .iter()
        .map(|p| format!("{}->{}/{}", p.nas_port, p.container_port, p.port_type))
        .collect::<Vec<_>>()
        .join(", ")
}

/// `name:tag`. UGOS leaves both empty for containers of a compose project.
fn image_name(name: &str, version: &str) -> String {
    match (name.is_empty(), version.is_empty()) {
        (true, _) => "-".to_owned(),
        (false, true) => name.to_owned(),
        (false, false) => format!("{name}:{version}"),
    }
}

/// What UGOS says about the state: a loop is worth seeing in the list.
fn state(name: &str, figures: Option<&ContainerStats>) -> String {
    match figures {
        Some(s) if s.restarting => format!("{name} (restarting)"),
        Some(s) if s.image_not_exist => format!("{name} (image missing)"),
        _ => name.to_owned(),
    }
}

#[allow(clippy::cast_possible_wrap)]
impl From<&ContainerView> for ContainerRow {
    fn from(v: &ContainerView) -> Self {
        let c = &v.container;
        let s = v.stats.as_ref();
        Self {
            id: c.container_id.chars().take(12).collect(),
            name: c.name.clone(),
            image: image_name(&c.image_name, &c.version),
            status: state(&c.status, s),
            // No figures for a stopped container: say so instead of 0.
            cpu: s
                .filter(|s| s.status == "running")
                .map_or_else(|| "-".to_owned(), |s| format!("{:.0}", s.cpu_usage)),
            memory: s
                .filter(|s| s.status == "running")
                .map_or_else(|| "-".to_owned(), |s| format_bytes(s.mem_usage as i64)),
            ports: s.map(ports).unwrap_or_default(),
            update: s
                .map(|s| if s.img_has_update { "yes" } else { "" })
                .unwrap_or_default()
                .to_owned(),
        }
    }
}

/// An entry of the Docker app's log.
#[derive(Serialize)]
pub struct OpLogRow {
    pub time: String,
    pub level: String,
    pub user: String,
    pub what: String,
}

table_row! {
  OpLogRow {
    time => "Time",
    level => "Level",
    user => "User",
    what => "What",
  }
}

impl From<&OpLog> for OpLogRow {
    fn from(l: &OpLog) -> Self {
        Self {
            time: l.create_time.clone(),
            level: match l.level {
                1 => "info".to_owned(),
                2 => "failure".to_owned(),
                n => n.to_string(),
            },
            user: l.operator.clone(),
            what: l.content.clone(),
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use ugos_client::types::docker::{Container, PortMap};

    fn view(state: &str, figures: Option<ContainerStats>) -> ContainerView {
        ContainerView {
            container: serde_json::from_value::<Container>(serde_json::json!({
                "name": "web", "containerId": "0123456789abcdef", "imageName": "nginx",
                "version": "1", "status": state
            }))
            .unwrap(),
            stats: figures,
        }
    }

    fn stats(json: serde_json::Value) -> ContainerStats {
        serde_json::from_value(json).unwrap()
    }

    #[test]
    fn a_running_container_shows_its_figures_and_ports() {
        let s = stats(serde_json::json!({
            "status": "running", "cpuUsage": 3, "memUsage": 134_217_728u64,
            "imgHasUpdate": true,
            "portMapping": [{"containerPort": "80", "nasPort": "8080", "portType": "tcp"}]
        }));
        let row = ContainerRow::from(&view("running", Some(s)));
        assert_eq!(row.id, "0123456789ab");
        assert_eq!(row.cpu, "3");
        assert_eq!(row.memory, "128.0 MiB");
        assert_eq!(row.ports, "8080->80/tcp");
        assert_eq!(row.update, "yes");
    }

    #[test]
    fn a_stopped_container_has_no_figures_and_a_loop_is_marked() {
        let stopped = stats(serde_json::json!({"status": "exited", "cpuUsage": 0}));
        let row = ContainerRow::from(&view("exited", Some(stopped)));
        assert_eq!((row.cpu.as_str(), row.memory.as_str()), ("-", "-"));
        let looping = stats(serde_json::json!({"status": "running", "restarting": true}));
        assert_eq!(
            ContainerRow::from(&view("running", Some(looping))).status,
            "running (restarting)"
        );
        assert_eq!(ContainerRow::from(&view("running", None)).ports, "");
    }

    #[test]
    fn an_image_without_a_name_shows_a_dash() {
        assert_eq!(image_name("", ""), "-");
        assert_eq!(image_name("nginx", ""), "nginx");
        assert_eq!(image_name("nginx", "1.27"), "nginx:1.27");
    }

    #[test]
    fn the_levels_of_the_log_have_names() {
        let entry = |level| OpLog {
            content: "x".into(),
            create_time: "t".into(),
            level,
            operator: "u".into(),
        };
        assert_eq!(OpLogRow::from(&entry(1)).level, "info");
        assert_eq!(OpLogRow::from(&entry(2)).level, "failure");
        assert_eq!(OpLogRow::from(&entry(7)).level, "7");
        let _ = PortMap {
            container_port: String::new(),
            nas_port: String::new(),
            port_type: String::new(),
        };
    }
}

// ── Networks and updates ────────────────────────────────────────────

/// A network in `docker network-ls`.
#[derive(Serialize)]
pub struct NetworkRow {
    pub id: String,
    pub name: String,
    pub driver: String,
    pub subnet: String,
    pub gateway: String,
    pub containers: String,
}

table_row! {
  NetworkRow {
    id => "ID",
    name => "Name",
    driver => "Driver",
    subnet => "Subnet",
    gateway => "Gateway",
    containers => "Containers",
  }
}

/// UGOS writes `--` where a network has no address.
fn address(text: &str) -> String {
    if text == "--" {
        String::new()
    } else {
        text.to_owned()
    }
}

impl From<&Network> for NetworkRow {
    fn from(n: &Network) -> Self {
        Self {
            id: n.network_id.chars().take(12).collect(),
            name: n.network_name.clone(),
            driver: n.driver.clone(),
            subnet: address(&n.ipv4_subnet),
            gateway: address(&n.ipv4_gateway),
            containers: n.container_names.len().to_string(),
        }
    }
}

/// A container attached to a network.
#[derive(Serialize)]
pub struct MemberRow {
    pub id: String,
    pub name: String,
    pub state: String,
}

table_row! {
  MemberRow {
    id => "ID",
    name => "Name",
    state => "State",
  }
}

impl From<&NetworkMember> for MemberRow {
    fn from(m: &NetworkMember) -> Self {
        Self {
            id: m.container_id.chars().take(12).collect(),
            name: m.container_name.clone(),
            state: m.running.clone(),
        }
    }
}

/// The details of one network.
pub fn network_rows(n: &Network) -> Vec<FieldRow> {
    let mut rows = vec![
        field("Name", n.network_name.clone()),
        field("ID", n.network_id.clone()),
        field("Driver", n.driver.clone()),
        field("IPv4 subnet", address(&n.ipv4_subnet)),
        field("IPv4 gateway", address(&n.ipv4_gateway)),
        field("IPv6", yes_no(n.ipv6_flag)),
    ];
    if n.ipv6_flag {
        rows.push(field("IPv6 subnet", address(&n.ipv6_subnet)));
        rows.push(field("IPv6 gateway", address(&n.ipv6_gateway)));
    }
    if !n.network_card.is_empty() {
        let card = if n.network_card_invalid {
            format!("{} (gone)", n.network_card)
        } else {
            n.network_card.clone()
        };
        rows.push(field("Network card", card));
    }
    rows.push(field("IP masquerading", yes_no(!n.disable_ip_masquerade)));
    rows.push(field("Created", format_unix(n.create)));
    rows
}

/// What is outdated, and the settings.
pub fn update_rows(s: &UpdateStatus) -> Vec<FieldRow> {
    let names = |list: &[String]| {
        if list.is_empty() {
            "none".to_owned()
        } else {
            list.join(", ")
        }
    };
    let c = &s.config;
    vec![
        field(
            "Containers with an update",
            format!("{} ({})", s.container_count, names(&s.containers)),
        ),
        field(
            "Projects with an update",
            format!("{} ({})", s.project_count, names(&s.projects)),
        ),
        field("Reminder", yes_no(c.update_reminder)),
        field("Scheduled check", yes_no(c.is_plan)),
        field("Time of the check", c.plan_settings.date.clone()),
    ]
}
