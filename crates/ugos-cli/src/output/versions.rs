//! Rows for the File Version Explorer.

use serde::Serialize;
use tabled::Tabled;
use ugos_client::types::versions::{DayMarks, FileVersion, VersionFolder, VersionItem};

use super::{FieldRow, field, format_unix, yes_no};

/// A folder and its version settings.
#[derive(Serialize)]
pub struct FolderRow {
    pub name: String,
    pub path: String,
    pub on: String,
    pub keep: String,
    pub size: String,
    pub note: String,
}

table_row! {
  FolderRow {
    name => "Name",
    path => "Folder",
    on => "Versions",
    keep => "Keeps",
    size => "Space",
    note => "Note",
  }
}

impl From<&VersionFolder> for FolderRow {
    fn from(f: &VersionFolder) -> Self {
        let note = if f.is_generation_failure {
            "making versions failed"
        } else if f.is_repairing {
            "repairing"
        } else if !f.is_available_volume || !f.is_enable_folder {
            "cannot keep versions"
        } else {
            ""
        };
        Self {
            name: f.name.clone(),
            path: f.path.clone(),
            on: yes_no(f.enable),
            keep: if f.enable {
                f.save_number.to_string()
            } else {
                "-".into()
            },
            size: format!("{} MB", f.size / 1_048_576),
            note: note.into(),
        }
    }
}

/// A file as listed at a time.
#[derive(Serialize)]
pub struct ItemRow {
    pub name: String,
    pub kind: String,
    pub size: i64,
    pub changed: String,
    pub deleted: String,
}

table_row! {
  ItemRow {
    name => "Name",
    kind => "Type",
    size => "Size",
    changed => "Changed",
    deleted => "Deleted",
  }
}

impl From<&VersionItem> for ItemRow {
    fn from(i: &VersionItem) -> Self {
        Self {
            name: i.name.clone(),
            kind: if i.file_type == 1 { "folder" } else { "file" }.into(),
            size: i.size,
            changed: format_unix(i.mtime),
            deleted: yes_no(i.is_deleted),
        }
    }
}

/// A version of a file.
#[derive(Serialize)]
pub struct VersionRow {
    pub version: String,
    pub made: String,
    pub changed: String,
    pub size: i64,
}

table_row! {
  VersionRow {
    version => "Version",
    made => "Version made",
    changed => "File changed",
    size => "Size",
  }
}

impl From<&FileVersion> for VersionRow {
    fn from(v: &FileVersion) -> Self {
        Self {
            version: v.version_number.clone(),
            made: format_unix(v.version_timestamp),
            changed: format_unix(v.last_modify_time),
            size: v.size,
        }
    }
}

/// The calendar of a month.
pub fn calendar_rows(days: &[DayMarks]) -> Vec<FieldRow> {
    days.iter()
        .enumerate()
        .filter(|(_, d)| d.create || d.modify || d.delete)
        .map(|(n, d)| {
            let mut what = Vec::new();
            if d.create {
                what.push("made");
            }
            if d.modify {
                what.push("changed");
            }
            if d.delete {
                what.push("deleted");
            }
            field(&format!("Day {}", n + 1), what.join(", "))
        })
        .collect()
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use ugos_client::types::versions::{DayMarks, FileVersion, VersionFolder, VersionItem};

    use super::{FolderRow, ItemRow, VersionRow, calendar_rows};

    #[test]
    fn folders_items_and_versions_are_shown() {
        let on = FolderRow::from(&VersionFolder {
            enable: true,
            save_number: 5,
            size: 3 << 20,
            is_available_volume: true,
            is_enable_folder: true,
            ..VersionFolder::default()
        });
        assert_eq!(
            (on.on.as_str(), on.keep.as_str(), on.size.as_str()),
            ("yes", "5", "3 MB")
        );
        let off = FolderRow::from(&VersionFolder::default());
        assert_eq!(
            (off.keep.as_str(), off.note.as_str()),
            ("-", "cannot keep versions")
        );
        let failed = FolderRow::from(&VersionFolder {
            is_generation_failure: true,
            ..VersionFolder::default()
        });
        assert_eq!(failed.note, "making versions failed");
        let item = ItemRow::from(&VersionItem {
            file_type: 1,
            is_deleted: true,
            ..VersionItem::default()
        });
        assert_eq!(
            (item.kind.as_str(), item.deleted.as_str()),
            ("folder", "yes")
        );
        let v = VersionRow::from(&FileVersion {
            version_number: "17".into(),
            size: 4,
            ..FileVersion::default()
        });
        assert_eq!((v.version.as_str(), v.size), ("17", 4));
    }

    #[test]
    fn the_calendar_lists_only_days_with_marks() {
        let days = vec![
            DayMarks::default(),
            DayMarks {
                create: true,
                delete: true,
                ..DayMarks::default()
            },
        ];
        let rows = calendar_rows(&days);
        assert_eq!(rows.len(), 1);
        assert_eq!(
            (rows[0].field.as_str(), rows[0].value.as_str()),
            ("Day 2", "made, deleted")
        );
    }
}
