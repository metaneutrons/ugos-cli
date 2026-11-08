//! Rows for share permissions and quotas.

use serde::Serialize;
use tabled::Tabled;
use ugos_client::types::access::{PermissionRow as Permission, QuotaRow};

/// What may be done on a share.
#[derive(Serialize)]
pub struct PermissionRow {
    pub share: String,
    pub path: String,
    pub set: String,
    pub effective: String,
}

table_row! {
  PermissionRow {
    share => "Share",
    path => "Path",
    set => "Set",
    effective => "Counts",
  }
}

const fn level(code: i64) -> &'static str {
    match code {
        -1 => "inherit",
        0 | 1 => "none",
        4 | 5 => "read",
        6 | 7 => "write",
        _ => "unknown",
    }
}

impl From<&Permission> for PermissionRow {
    fn from(p: &Permission) -> Self {
        Self {
            share: p.name.clone(),
            path: p.path.clone(),
            set: level(p.permission).into(),
            effective: level(p.permission_mask).into(),
        }
    }
}

/// The limit on a volume.
#[derive(Serialize)]
pub struct QuotaTableRow {
    pub volume: String,
    pub limit: String,
    pub largest: String,
}

table_row! {
  QuotaTableRow {
    volume => "Volume",
    limit => "Limit",
    largest => "Largest possible",
  }
}

fn size(kib: i64) -> String {
    if kib == 0 {
        return "none".into();
    }
    let mb = kib / 1024;
    if mb >= 1024 * 1024 && mb % (1024 * 1024) == 0 {
        format!("{} TB", mb / (1024 * 1024))
    } else if mb >= 1024 && mb % 1024 == 0 {
        format!("{} GB", mb / 1024)
    } else {
        format!("{mb} MB")
    }
}

impl From<&QuotaRow> for QuotaTableRow {
    fn from(r: &QuotaRow) -> Self {
        Self {
            volume: r.path.clone(),
            limit: size(r.hard),
            largest: size(r.max_value / 1024),
        }
    }
}

#[cfg(test)]
mod tests {
    use ugos_client::types::access::{PermissionRow as Permission, QuotaRow};

    use super::{PermissionRow, QuotaTableRow, size};

    #[test]
    fn permissions_and_limits_are_shown() {
        let row = PermissionRow::from(&Permission {
            name: "docs".into(),
            permission: 7,
            permission_mask: 0,
            ..Permission::default()
        });
        assert_eq!(
            (row.set.as_str(), row.effective.as_str()),
            ("write", "none")
        );
        assert_eq!(
            PermissionRow::from(&Permission {
                permission: -1,
                ..Permission::default()
            })
            .set,
            "inherit"
        );
        assert_eq!(size(0), "none");
        assert_eq!(size(512 * 1024), "512 MB");
        assert_eq!(size(2 << 20), "2 GB");
        assert_eq!(size(1 << 30), "1 TB");
        let q = QuotaTableRow::from(&QuotaRow {
            path: "/volume1".into(),
            hard: 1 << 20,
            max_value: 1 << 40,
            ..QuotaRow::default()
        });
        assert_eq!((q.limit.as_str(), q.largest.as_str()), ("1 GB", "1 TB"));
    }
}
