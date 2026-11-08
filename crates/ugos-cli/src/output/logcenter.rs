//! Rows for the Log Center.

use serde::Serialize;
use tabled::Tabled;
use ugos_client::types::logcenter::{
    ArchiveConfig, ArchiveFile, ArchiveRecord, NotifyConfig, NotifyRecord, UsersModules,
};

use super::{FieldRow, field, format_unix, yes_no};

fn list(items: &[String]) -> String {
    if items.is_empty() {
        "-".into()
    } else {
        items.join(", ")
    }
}

/// The archive policy.
pub fn archive_rows(c: &ArchiveConfig) -> Vec<FieldRow> {
    let trigger = |n: i64, unit: &str| {
        if n == 0 {
            "-".to_owned()
        } else {
            format!("{n} {unit}")
        }
    };
    vec![
        field("Archiving", yes_no(c.status)),
        field("At log size", trigger(c.data_bytes, "MB")),
        field("At entries", trigger(c.num, "entries")),
        field("Older than", trigger(c.remain_time, "months")),
        field(
            "Folder",
            if c.path.is_empty() {
                "-".into()
            } else {
                c.path.clone()
            },
        ),
        field("Folder exists", yes_no(c.path_exist)),
    ]
}

/// The notification policy.
pub fn notify_rows(c: &NotifyConfig) -> Vec<FieldRow> {
    vec![
        field("Notifying", yes_no(c.status)),
        field(
            "Keyword",
            c.keywords
                .first()
                .filter(|k| !k.is_empty())
                .cloned()
                .unwrap_or_else(|| "-".into()),
        ),
        field("Levels", list(&c.levels)),
        field("Modules", list(&c.modules)),
        field("Users", list(&c.operators)),
    ]
}

/// Users and modules in the log.
pub fn users_modules_rows(u: &UsersModules) -> Vec<FieldRow> {
    vec![
        field("Modules", list(&u.modules)),
        field("Users", list(&u.users)),
    ]
}

/// An archive that was made.
#[derive(Serialize)]
pub struct ArchiveRecordRow {
    pub made: String,
    pub from: String,
    pub to: String,
    pub file: String,
}

table_row! {
  ArchiveRecordRow {
    made => "Made",
    from => "First entry",
    to => "Last entry",
    file => "File",
  }
}

impl From<&ArchiveRecord> for ArchiveRecordRow {
    fn from(r: &ArchiveRecord) -> Self {
        Self {
            made: format_unix(r.archive_time),
            from: format_unix(r.start_time),
            to: format_unix(r.end_time),
            file: r.output_path.clone(),
        }
    }
}

/// An archive in the viewer.
#[derive(Serialize)]
pub struct ArchiveFileRow {
    pub id: i64,
    pub name: String,
    pub from: String,
    pub to: String,
    pub in_use: String,
}

table_row! {
  ArchiveFileRow {
    id => "Id",
    name => "File",
    from => "First entry",
    to => "Last entry",
    in_use => "In use",
  }
}

impl From<&ArchiveFile> for ArchiveFileRow {
    fn from(f: &ArchiveFile) -> Self {
        Self {
            id: f.archive_id,
            name: f.file_name.clone(),
            from: format_unix(f.start_time),
            to: format_unix(f.end_time),
            in_use: yes_no(f.in_use),
        }
    }
}

/// A notification that was sent.
#[derive(Serialize)]
pub struct NotifyRecordRow {
    pub time: String,
    pub log: String,
    pub id: String,
}

table_row! {
  NotifyRecordRow {
    time => "Sent",
    log => "Entry",
    id => "Log id",
  }
}

impl From<&NotifyRecord> for NotifyRecordRow {
    fn from(r: &NotifyRecord) -> Self {
        Self {
            time: format_unix(r.notify_time),
            log: r.log_content.clone(),
            id: r.log_id.clone(),
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use ugos_client::types::logcenter::{
        ArchiveConfig, ArchiveFile, ArchiveRecord, NotifyConfig, NotifyRecord, UsersModules,
    };

    use super::{
        ArchiveFileRow, ArchiveRecordRow, NotifyRecordRow, archive_rows, notify_rows,
        users_modules_rows,
    };

    fn value(rows: &[super::FieldRow], name: &str) -> String {
        rows.iter()
            .find(|r| r.field == name)
            .map(|r| r.value.clone())
            .unwrap_or_default()
    }

    #[test]
    fn the_policies_are_shown() {
        let archive = ArchiveConfig {
            status: true,
            data_bytes: 500,
            remain_time: 3,
            path: "/a".into(),
            ..ArchiveConfig::default()
        };
        let rows = archive_rows(&archive);
        assert_eq!(value(&rows, "At log size"), "500 MB");
        assert_eq!(value(&rows, "At entries"), "-");
        assert_eq!(value(&rows, "Older than"), "3 months");
        let notify = NotifyConfig {
            status: true,
            keywords: vec![String::new()],
            levels: vec!["critical".into()],
            ..NotifyConfig::default()
        };
        let rows = notify_rows(&notify);
        assert_eq!(value(&rows, "Keyword"), "-");
        assert_eq!(value(&rows, "Levels"), "critical");
        assert_eq!(value(&rows, "Users"), "-");
        let um = UsersModules {
            users: vec!["a".into()],
            modules: vec!["login".into(), "vault".into()],
        };
        assert_eq!(value(&users_modules_rows(&um), "Modules"), "login, vault");
    }

    #[test]
    fn archives_and_notifications_are_shown() {
        let record = ArchiveRecordRow::from(&ArchiveRecord {
            output_path: "/x.ulog".into(),
            ..ArchiveRecord::default()
        });
        assert_eq!(record.file, "/x.ulog");
        let file = ArchiveFileRow::from(&ArchiveFile {
            archive_id: 4,
            in_use: true,
            ..ArchiveFile::default()
        });
        assert_eq!((file.id, file.in_use.as_str()), (4, "yes"));
        let sent = NotifyRecordRow::from(&NotifyRecord {
            log_content: "c".into(),
            log_id: "9".into(),
            ..NotifyRecord::default()
        });
        assert_eq!((sent.log.as_str(), sent.id.as_str()), ("c", "9"));
    }
}
