//! Rows for the SAN Manager.

use serde::Serialize;
use tabled::Tabled;
use ugos_client::types::san::{Connection, Group, Lun, Setting, Target};

use super::{FieldRow, field, yes_no};

fn gib(bytes: i64) -> String {
    if bytes < 0 {
        return "-".into();
    }
    let g = bytes / (1 << 30);
    if g >= 1024 && g % 1024 == 0 {
        format!("{} TB", g / 1024)
    } else {
        format!("{g} GB")
    }
}

const fn target_state(status: i64) -> &'static str {
    match status {
        0 | 1 => "on",
        2 => "off",
        3..=7 => "busy",
        _ => "unknown",
    }
}

const fn lun_state(status: i64) -> &'static str {
    match status {
        0 => "normal",
        1 => "damaged",
        2 => "read only",
        3 => "not enough space",
        4 => "protected",
        5 => "creating",
        6 => "editing",
        7 => "removing",
        8 => "expanding",
        _ => "unknown",
    }
}

/// A target in a list.
#[derive(Serialize)]
pub struct TargetRow {
    pub id: i64,
    pub name: String,
    pub iqn: String,
    pub luns: String,
    pub chap: String,
    pub state: String,
}

table_row! {
  TargetRow {
    id => "Id",
    name => "Name",
    iqn => "IQN",
    luns => "LUNs",
    chap => "CHAP",
    state => "State",
  }
}

impl From<&Target> for TargetRow {
    fn from(t: &Target) -> Self {
        let luns: Vec<&str> = t.luns.iter().map(|l| l.lun_name.as_str()).collect();
        Self {
            id: t.id,
            name: t.name.clone(),
            iqn: t.iqn.clone(),
            luns: if luns.is_empty() {
                "-".into()
            } else {
                luns.join(", ")
            },
            chap: t.chap_type.clone(),
            state: target_state(t.status).into(),
        }
    }
}

/// One target.
pub fn target_rows(t: &Target) -> Vec<FieldRow> {
    let mut rows = vec![
        field("Name", t.name.clone()),
        field("IQN", t.iqn.clone()),
        field("State", target_state(t.status)),
        field(
            "LUNs",
            t.luns
                .iter()
                .map(|l| l.lun_name.clone())
                .collect::<Vec<_>>()
                .join(", "),
        ),
        field("CHAP", t.chap_type.clone()),
        field("Header digest", yes_no(t.enable_head_crc)),
        field("Data digest", yes_no(t.enable_data_crc)),
        field("Several connections", yes_no(t.multi_conn)),
    ];
    if !t.chap_info.userid.is_empty() {
        rows.push(field("CHAP user", t.chap_info.userid.clone()));
    }
    if !t.chap_info.mutual_userid.is_empty() {
        rows.push(field("Target CHAP user", t.chap_info.mutual_userid.clone()));
    }
    rows
}

/// A LUN in a list.
#[derive(Serialize)]
pub struct LunRow {
    pub id: i64,
    pub name: String,
    pub kind: String,
    pub size: String,
    pub used: String,
    pub volume: String,
    pub target: String,
    pub state: String,
}

table_row! {
  LunRow {
    id => "Id",
    name => "Name",
    kind => "Type",
    size => "Size",
    used => "Used",
    volume => "Volume",
    target => "Target",
    state => "State",
  }
}

impl From<&Lun> for LunRow {
    fn from(l: &Lun) -> Self {
        Self {
            id: l.id,
            name: l.name.clone(),
            kind: l.kind.clone(),
            size: gib(l.size),
            used: gib(l.used_size),
            volume: l.volume_label.clone(),
            target: l
                .targets
                .first()
                .map_or_else(|| "-".into(), |t| t.target_name.clone()),
            state: lun_state(l.status).into(),
        }
    }
}

/// One LUN.
pub fn lun_rows(l: &Lun) -> Vec<FieldRow> {
    let access = if l.group_access.is_empty() {
        "all groups".to_owned()
    } else {
        l.group_access
            .iter()
            .map(|g| format!("{} ({})", g.group_name, g.access_mode))
            .collect::<Vec<_>>()
            .join(", ")
    };
    vec![
        field("Name", l.name.clone()),
        field("Description", l.desc.clone()),
        field("Type", l.kind.clone()),
        field("Size", gib(l.size)),
        field("Used", gib(l.used_size)),
        field("Volume", l.volume_label.clone()),
        field("State", lun_state(l.status)),
        field("Force unit access", yes_no(l.enable_fua)),
        field("Access", access),
        field(
            "Target",
            l.targets
                .first()
                .map_or_else(|| "-".into(), |t| t.target_name.clone()),
        ),
    ]
}

/// A group in a list.
#[derive(Serialize)]
pub struct GroupRow {
    pub id: i64,
    pub name: String,
    pub desc: String,
    pub initiators: String,
}

table_row! {
  GroupRow {
    id => "Id",
    name => "Name",
    desc => "Description",
    initiators => "Initiators",
  }
}

impl From<&Group> for GroupRow {
    fn from(g: &Group) -> Self {
        Self {
            id: g.id,
            name: g.name.clone(),
            desc: g.desc.clone(),
            initiators: g.initiator_iqns.join(", "),
        }
    }
}

/// The global setting.
pub fn setting_rows(s: &Setting) -> Vec<FieldRow> {
    let chap = match s.kind {
        1 => "one-way",
        2 => "mutual",
        _ => "off",
    };
    let mut rows = vec![
        field("Port", s.port.to_string()),
        field("IQN prefix", s.iqn_prefix.clone()),
        field("Default CHAP", chap),
    ];
    if !s.userid.is_empty() {
        rows.push(field("CHAP user", s.userid.clone()));
    }
    if !s.mutual_userid.is_empty() {
        rows.push(field("Target CHAP user", s.mutual_userid.clone()));
    }
    rows
}

/// A connected initiator.
#[derive(Serialize)]
pub struct ConnectionRow {
    pub target: String,
    pub ip: String,
    pub iqn: String,
    pub status: String,
}

table_row! {
  ConnectionRow {
    target => "Target",
    ip => "Address",
    iqn => "Initiator IQN",
    status => "State",
  }
}

impl From<&Connection> for ConnectionRow {
    fn from(c: &Connection) -> Self {
        Self {
            target: c.label.clone(),
            ip: c.ip.clone(),
            iqn: c.iqn.clone(),
            status: c.status.clone(),
        }
    }
}
