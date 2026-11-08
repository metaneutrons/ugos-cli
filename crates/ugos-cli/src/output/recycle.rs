//! Rows for the recycle bins.

use serde::Serialize;
use tabled::Tabled;
use ugos_client::types::recycle::{Plan, PlanTarget, RecycleBin, RecycleConf, RecycleFile};

use super::{FieldRow, field, format_unix, yes_no};

/// The two switches.
pub fn conf_rows(c: &RecycleConf) -> Vec<FieldRow> {
    vec![
        field("Shortcut on the desktop", yes_no(c.desktop_recycle)),
        field("Recycle folder inside folders", yes_no(c.dir_recycle)),
    ]
}

/// A bin in a list.
#[derive(Serialize)]
pub struct BinRow {
    pub name: String,
    pub path: String,
    pub on: String,
    pub files: String,
    pub admin_only: String,
    pub retention: String,
}

table_row! {
  BinRow {
    name => "Name",
    path => "Folder",
    on => "On",
    files => "Holds files",
    admin_only => "Admins only",
    retention => "Retention",
  }
}

impl From<&RecycleBin> for BinRow {
    fn from(b: &RecycleBin) -> Self {
        Self {
            name: b.name.clone(),
            path: b.path.clone(),
            on: yes_no(b.status),
            files: yes_no(b.recycle_not_empty),
            admin_only: if b.user_id == 0 {
                yes_no(b.allow_only_admin_recycle)
            } else {
                "-".into()
            },
            retention: format!("{} days", b.dynamic_cleaning_day.interval),
        }
    }
}

/// A file in a bin.
#[derive(Serialize)]
pub struct FileRow {
    pub name: String,
    pub kind: String,
    pub size: u64,
    pub deleted: String,
    pub by: String,
    pub path: String,
}

table_row! {
  FileRow {
    name => "Name",
    kind => "Type",
    size => "Size",
    deleted => "Deleted",
    by => "By",
    path => "Path",
  }
}

impl From<&RecycleFile> for FileRow {
    fn from(f: &RecycleFile) -> Self {
        Self {
            name: f.name.clone(),
            kind: if f.file_type == 1 { "folder" } else { "file" }.into(),
            size: u64::try_from(f.size).unwrap_or(0),
            deleted: format_unix(f.dtime),
            by: if f.uname.is_empty() {
                "-".into()
            } else {
                f.uname.clone()
            },
            path: f.path.clone(),
        }
    }
}

fn names(list: &[PlanTarget]) -> String {
    list.iter()
        .map(|t| t.name.as_str())
        .collect::<Vec<_>>()
        .join(", ")
}

/// A cleanup plan.
#[derive(Serialize)]
pub struct PlanRow {
    pub id: i64,
    pub name: String,
    pub covers: String,
    pub next: String,
}

table_row! {
  PlanRow {
    id => "Id",
    name => "Plan",
    covers => "Covers",
    next => "Next run",
  }
}

impl From<&Plan> for PlanRow {
    fn from(p: &Plan) -> Self {
        let covers = if p.is_select_all {
            "every bin".to_owned()
        } else {
            [names(&p.shares), names(&p.users)]
                .into_iter()
                .filter(|s| !s.is_empty())
                .collect::<Vec<_>>()
                .join("; ")
        };
        Self {
            id: p.id,
            name: if p.task_name.is_empty() {
                p.default_task_name.clone()
            } else {
                p.task_name.clone()
            },
            covers,
            next: format_unix(p.next_time),
        }
    }
}

/// A share or user a plan can take.
#[derive(Serialize)]
pub struct ChoiceRow {
    pub kind: String,
    pub id: i64,
    pub name: String,
}

table_row! {
  ChoiceRow {
    kind => "Kind",
    id => "Id",
    name => "Name",
  }
}

impl ChoiceRow {
    /// A share or a user.
    pub fn new(kind: &str, t: &PlanTarget) -> Self {
        Self {
            kind: kind.into(),
            id: t.id,
            name: t.name.clone(),
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use ugos_client::types::recycle::{
        CleaningDay, Plan, PlanTarget, RecycleBin, RecycleConf, RecycleFile,
    };

    use super::{BinRow, ChoiceRow, FileRow, PlanRow, conf_rows};

    #[test]
    fn a_bin_is_shown() {
        let bin = RecycleBin {
            name: "docs".into(),
            status: true,
            recycle_not_empty: true,
            allow_only_admin_recycle: true,
            dynamic_cleaning_day: CleaningDay { interval: 30 },
            ..RecycleBin::default()
        };
        let row = BinRow::from(&bin);
        assert_eq!(
            (row.on.as_str(), row.files.as_str(), row.admin_only.as_str()),
            ("yes", "yes", "yes")
        );
        assert_eq!(row.retention, "30 days");
        let user = BinRow::from(&RecycleBin {
            user_id: 1000,
            ..RecycleBin::default()
        });
        assert_eq!(user.admin_only, "-");
    }

    #[test]
    fn a_file_and_the_switches_are_shown() {
        let file = FileRow::from(&RecycleFile {
            file_type: 1,
            size: -3,
            ..RecycleFile::default()
        });
        assert_eq!(
            (file.kind.as_str(), file.size, file.by.as_str()),
            ("folder", 0, "-")
        );
        assert_eq!(conf_rows(&RecycleConf::default()).len(), 2);
    }

    #[test]
    fn plans_and_choices_are_shown() {
        let all = PlanRow::from(&Plan {
            is_select_all: true,
            default_task_name: "auto".into(),
            ..Plan::default()
        });
        assert_eq!(
            (all.covers.as_str(), all.name.as_str()),
            ("every bin", "auto")
        );
        let some = PlanRow::from(&Plan {
            task_name: "mine".into(),
            shares: vec![PlanTarget {
                id: 1,
                name: "a".into(),
            }],
            users: vec![PlanTarget {
                id: 2,
                name: "b".into(),
            }],
            ..Plan::default()
        });
        assert_eq!((some.covers.as_str(), some.name.as_str()), ("a; b", "mine"));
        assert_eq!(
            ChoiceRow::new(
                "share",
                &PlanTarget {
                    id: 4,
                    name: "x".into()
                }
            )
            .id,
            4
        );
    }
}
