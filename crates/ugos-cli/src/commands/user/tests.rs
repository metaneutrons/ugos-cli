//! Unit tests of the `user` command.

use super::*;
use ugos_client::types::syslog::GroupRef;

fn account() -> User {
    User {
        username: "anna".into(),
        uid: 1001,
        description: "Anna".into(),
        role: ROLE_USER.into(),
        groups: vec![
            GroupRef {
                gid: 100,
                name: "users".into(),
            },
            GroupRef {
                gid: 1002,
                name: "family".into(),
            },
        ],
        expire_time: ugos_client::types::users::EXPIRES_NEVER,
        enable_home_dir: true,
        ..User::default()
    }
}

fn options<'a>(name: &'a str, groups: &'a [String]) -> CreateOptions<'a> {
    CreateOptions {
        name,
        email: None,
        description: None,
        role: RoleArg::User,
        groups,
        home_dir: true,
        deny_password_change: false,
    }
}

#[test]
fn names_the_web_ui_would_refuse_are_refused() {
    for bad in [
        "",
        " anna",
        "anna ",
        "-anna",
        "root",
        "Admin",
        "ADMINISTRATOR",
    ] {
        assert!(check_name(bad).is_err(), "{bad:?} should be refused");
    }
    assert!(check_name("anna").is_ok());
    assert!(check_name("anna.b-c_d").is_ok());
}

#[test]
fn a_new_standard_user_has_a_home_folder_and_no_groups() {
    let new = build_new_user(&options("anna", &[])).unwrap();
    assert_eq!(new.role, ROLE_USER);
    assert!(new.enable_home_dir);
    assert_eq!(new.groups, Vec::<String>::new());
}

#[test]
fn creation_options_reach_the_request() {
    let groups = vec!["family".to_owned()];
    let mut asked = options("anna", &groups);
    asked.deny_password_change = true;
    asked.email = Some(" anna@example.org ");
    asked.description = Some("Anna");
    let new = build_new_user(&asked).unwrap();
    assert!(new.deny_change_pwd);
    assert_eq!(new.email, "anna@example.org");
    assert_eq!(new.description, "Anna");
    assert_eq!(new.groups, groups);
}

#[test]
fn an_administrator_may_always_change_the_password() {
    let mut asked = options("boss", &[]);
    asked.role = RoleArg::Admin;
    asked.deny_password_change = true;
    let new = build_new_user(&asked).unwrap();
    assert_eq!(new.role, ROLE_ADMIN);
    assert!(!new.deny_change_pwd);
}

#[test]
fn a_bad_name_is_refused_before_anything_is_sent() {
    assert!(build_new_user(&options("root", &[])).is_err());
}

#[test]
fn an_empty_change_is_empty() {
    assert!(UserChanges::default().is_empty());
    let one = UserChanges {
        description: Some("x".into()),
        ..UserChanges::default()
    };
    assert!(!one.is_empty());
}

#[test]
fn an_edit_keeps_everything_that_was_not_asked_for() {
    let change = UserChanges {
        description: Some("Anna B.".into()),
        ..UserChanges::default()
    };
    let edit = change.apply(&account()).unwrap();
    assert_eq!(edit.description, "Anna B.");
    assert_eq!(edit.role, ROLE_USER);
    assert_eq!(edit.groups, ["users", "family"]);
    assert_eq!(edit.expire_time, ugos_client::types::users::EXPIRES_NEVER);
    assert_eq!(edit.email, "");
    assert_eq!(edit.password, None);
}

#[test]
fn an_email_that_cannot_be_read_back_is_not_blanked() {
    // UGOS blanks the address when an edit leaves it out, so an edit that
    // cannot say what it is has to stop.
    let mut masked = account();
    masked.email = "a****a@example.org".into();
    let change = UserChanges {
        description: Some("x".into()),
        ..UserChanges::default()
    };
    let error = change.apply(&masked).unwrap_err().to_string();
    assert!(error.contains("--email"), "{error}");

    let give = UserChanges {
        email: Some(" new@example.org ".into()),
        ..UserChanges::default()
    };
    assert_eq!(give.apply(&masked).unwrap().email, "new@example.org");
}

#[test]
fn groups_are_added_once_and_removed_by_name() {
    let change = UserChanges {
        add_groups: vec!["family".into(), "media".into()],
        remove_groups: vec!["users".into()],
        ..UserChanges::default()
    };
    assert_eq!(
        change.apply(&account()).unwrap().groups,
        ["family", "media"]
    );
}

#[test]
fn disabling_sets_the_expiry_and_drops_the_date() {
    let change = UserChanges {
        expiry: Some(Expiry::DISABLED),
        ..UserChanges::default()
    };
    let edit = change.apply(&account()).unwrap();
    assert_eq!(edit.expire_time, ugos_client::types::users::DISABLED);
    assert_eq!(edit.expire_date, "");
}

#[test]
fn a_date_is_checked_as_text_since_ugos_moves_the_time() {
    let expiry = parse_date("2027-01-31", 0).unwrap();
    let change = UserChanges {
        expiry: Some(expiry),
        ..UserChanges::default()
    };
    let edit = change.apply(&account()).unwrap();
    assert!(edit.expire_time > 1);
    assert_eq!(edit.expire_date, "2027-01-31");
    // UGOS keeps the date and stores midnight of its own time zone.
    let mut now = account();
    now.expire_time = edit.expire_time - 3600;
    now.expire_date = "2027-01-31".into();
    assert!(!change.expiry_differs(&edit, &now));
    now.expire_date = "2027-02-01".into();
    assert!(change.expiry_differs(&edit, &now));
    // Neither a missing date nor a switched-off account is an expiry.
    now.expire_date = "2027-01-31".into();
    now.expire_time = 1;
    assert!(change.expiry_differs(&edit, &now));
}

#[test]
fn a_change_is_checked_against_what_the_account_shows_now() {
    let change = UserChanges {
        description: Some("Anna B.".into()),
        ..UserChanges::default()
    };
    let edit = change.apply(&account()).unwrap();
    assert!(change.mismatch(&edit, &account()).is_some());
    let mut done = account();
    done.description = "Anna B.".into();
    assert_eq!(change.mismatch(&edit, &done), None);
}

#[test]
fn a_wiped_email_is_noticed() {
    let mut had = account();
    had.email = "a****a@example.org".into();
    had.email_encrypted = String::new();
    let change = UserChanges {
        email: Some("anna@example.org".into()),
        ..UserChanges::default()
    };
    let edit = change.apply(&had).unwrap();
    assert!(
        change
            .mismatch(&edit, &account())
            .unwrap()
            .contains("email")
    );
}

#[test]
fn only_the_groups_that_were_named_are_checked() {
    let held = [
        "users".to_owned(),
        "family".to_owned(),
        "ughomeusers".to_owned(),
    ];
    let joined = group_mismatch(&["media".to_owned()], &[], &held).unwrap();
    assert!(joined.contains("media"), "{joined}");
    let kept = group_mismatch(&[], &["users".to_owned()], &held).unwrap();
    assert!(kept.contains("users"), "{kept}");
    // Groups UGOS derives are not what was asked and not a problem.
    assert_eq!(group_mismatch(&["family".to_owned()], &[], &held), None);
    assert_eq!(group_mismatch(&[], &["media".to_owned()], &held), None);
}

#[test]
fn a_group_that_did_not_stick_at_creation_is_named() {
    let mut asked = NewUser::new("anna", "");
    asked.groups = vec!["family".into(), "media".into()];
    let problem = created_mismatch(&asked, &account()).unwrap();
    assert!(problem.contains("media"), "{problem}");
    assert!(!problem.contains("family"), "{problem}");
}

#[test]
fn a_wrong_role_is_named() {
    let mut asked = NewUser::new("anna", "");
    asked.role = ROLE_ADMIN.into();
    assert!(
        created_mismatch(&asked, &account())
            .unwrap()
            .contains("role")
    );
}

#[test]
fn administrators_are_counted() {
    let mut boss = account();
    boss.role = ROLE_ADMIN.into();
    assert_eq!(admins(&[boss.clone(), account()]), 1);
    assert_eq!(admins(&[boss.clone(), boss]), 2);
}
