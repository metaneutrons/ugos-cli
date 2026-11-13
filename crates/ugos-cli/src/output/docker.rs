//! Rows for the Docker commands: containers with their figures, the Docker
//! app's own log.

use serde::Serialize;
use tabled::Tabled;
use ugos_client::ops::dockerview::ContainerView;
use ugos_client::types::docker::{ContainerStats, OpLog};

use super::format_bytes;

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
