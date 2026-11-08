//! Rows for password policies, invitations and imports.

use serde::Serialize;
use tabled::Tabled;
use ugos_client::types::policy::{ImportRow, Invitation, PasswordExpiry, PasswordRules};

use super::{FieldRow, field, format_unix, yes_no};

/// The strength rules of passwords.
pub fn rules_rows(r: &PasswordRules) -> Vec<FieldRow> {
    vec![
        field("Shortest password", r.min_length.to_string()),
        field("No user name in it", yes_no(r.disable_contain_key)),
        field("No common password", yes_no(r.disable_common_pwd)),
        field("Upper and lower case", yes_no(r.must_contain_case)),
        field("A digit", yes_no(r.must_contain_num)),
        field("A special character", yes_no(r.must_contain_special_char)),
    ]
}

/// The expiry of passwords.
pub fn expiry_rows(e: &PasswordExpiry) -> Vec<FieldRow> {
    vec![
        field("Passwords expire", yes_no(e.enable)),
        field("Valid for", format!("{} days", e.max_days)),
        field("Warning", format!("{} days before", e.warn_age)),
        field("Change is forced", yes_no(e.force_change)),
        field(
            "Never expire for",
            if e.white_list.is_empty() {
                "-".into()
            } else {
                e.white_list.join(", ")
            },
        ),
    ]
}

/// A point in time as the other tables show it.
pub fn time_text(ts: i64) -> String {
    format_unix(ts)
}

/// An application for an account.
#[derive(Serialize)]
pub struct InvitationRow {
    pub id: i64,
    pub name: String,
    pub email: String,
    pub reason: String,
    pub when: String,
    pub approved_by: String,
}

table_row! {
  InvitationRow {
    id => "Id",
    name => "Name",
    email => "Email",
    reason => "Reason",
    when => "Applied",
    approved_by => "Approved by",
  }
}

impl From<&Invitation> for InvitationRow {
    fn from(i: &Invitation) -> Self {
        Self {
            id: i.id,
            name: i.username.clone(),
            email: i.email.clone(),
            reason: i.reason.clone(),
            when: format_unix(i.created_at),
            approved_by: if i.agree_name.is_empty() {
                "-".into()
            } else {
                i.agree_name.clone()
            },
        }
    }
}

/// A row of an import file. The password is not shown.
#[derive(Serialize)]
pub struct ImportRowView {
    pub name: String,
    pub role: String,
    pub email: String,
    pub ok: String,
    pub problem: String,
}

table_row! {
  ImportRowView {
    name => "Name",
    role => "Role",
    email => "Email",
    ok => "Valid",
    problem => "Problem",
  }
}

impl From<&ImportRow> for ImportRowView {
    fn from(r: &ImportRow) -> Self {
        Self {
            name: r.username.clone(),
            role: r.role.clone(),
            email: r.email.clone(),
            ok: yes_no(r.valid),
            problem: r.err_tip.clone(),
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use ugos_client::types::policy::{ImportRow, Invitation, PasswordExpiry, PasswordRules};

    use super::{ImportRowView, InvitationRow, expiry_rows, rules_rows};

    fn value(rows: &[super::FieldRow], name: &str) -> String {
        rows.iter()
            .find(|r| r.field == name)
            .map(|r| r.value.clone())
            .unwrap_or_default()
    }

    #[test]
    fn policies_are_shown() {
        let rules = PasswordRules {
            min_length: 8,
            must_contain_num: true,
            ..PasswordRules::default()
        };
        let rows = rules_rows(&rules);
        assert_eq!(
            (
                value(&rows, "Shortest password").as_str(),
                value(&rows, "A digit").as_str()
            ),
            ("8", "yes")
        );
        let expiry = PasswordExpiry {
            enable: true,
            max_days: 90,
            warn_age: 7,
            white_list: vec!["a".into()],
            ..PasswordExpiry::default()
        };
        let rows = expiry_rows(&expiry);
        assert_eq!(value(&rows, "Valid for"), "90 days");
        assert_eq!(value(&rows, "Never expire for"), "a");
        assert_eq!(
            value(&expiry_rows(&PasswordExpiry::default()), "Never expire for"),
            "-"
        );
    }

    #[test]
    fn applications_and_import_rows_are_shown_without_passwords() {
        let inv = InvitationRow::from(&Invitation {
            id: 3,
            username: "x".into(),
            ..Invitation::default()
        });
        assert_eq!((inv.id, inv.approved_by.as_str()), (3, "-"));
        let row = ImportRowView::from(&ImportRow {
            username: "u".into(),
            password: "secret".into(),
            valid: false,
            err_tip: "taken".into(),
            ..ImportRow::default()
        });
        assert_eq!((row.ok.as_str(), row.problem.as_str()), ("no", "taken"));
        assert!(!serde_json::to_string(&row).unwrap().contains("secret"));
    }
}
