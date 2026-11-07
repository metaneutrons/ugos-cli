//! Unit tests of the share operations.

use super::*;

fn volume(name: &str, path: &str) -> Volume {
    Volume {
        name: name.into(),
        path: path.into(),
        ..Volume::default()
    }
}

fn acl() -> PathAcl {
    serde_json::from_str(
        r#"{"acls": [
            {"name": "admin", "gid": 10, "permission_mask": 7, "select_permissions": 7,
             "users": [{"username": "fabian", "uid": 1000, "is_admin": true,
                        "permission_mask": 7, "select_permissions": -1}]},
            {"name": "family", "gid": 101, "permission_mask": 4, "select_permissions": 5,
             "users": [{"username": "anna", "uid": 1001, "is_admin": false,
                        "permission_mask": 5, "select_permissions": -1}]}
        ], "total_users": 2}"#,
    )
    .unwrap()
}

#[test]
fn names_the_web_ui_would_refuse_are_refused() {
    assert!(check_name("media").is_ok());
    assert!(check_name("  media  ").is_ok());
    assert!(check_name("").is_err());
    assert!(check_name("   ").is_err());
    assert!(check_name(&"x".repeat(81)).is_err());
    assert!(check_name(&"x".repeat(80)).is_ok());
    assert!(check_name("a/b").is_err());
    assert!(check_name("a\\b").is_err());
    assert!(check_name("@recycle").is_err());
    assert!(check_name("Home").is_err());
    assert!(check_name("personal_folder").is_err());
}

#[test]
fn the_only_volume_is_picked_without_being_asked() {
    let one = [volume("Volume 1", "/volume1")];
    assert_eq!(pick_volume(&one, None).unwrap(), "/volume1");
    assert_eq!(pick_volume(&one, Some("/volume1/")).unwrap(), "/volume1");
    assert_eq!(pick_volume(&one, Some("Volume 1")).unwrap(), "/volume1");
}

#[test]
fn several_volumes_need_a_choice_and_an_unknown_one_lists_them() {
    let two = [
        volume("Volume 1", "/volume1"),
        volume("Volume 2", "/volume2"),
    ];
    let err = pick_volume(&two, None).unwrap_err().to_string();
    assert!(err.contains("/volume1, /volume2"), "{err}");
    assert_eq!(pick_volume(&two, Some("/volume2")).unwrap(), "/volume2");
    let err = pick_volume(&two, Some("/volume9")).unwrap_err().to_string();
    assert!(err.contains("/volume1, /volume2"), "{err}");
    assert!(pick_volume(&[], None).is_err());
}

#[test]
fn quotas_are_a_number_and_a_unit() {
    assert_eq!(parse_quota("500m").unwrap(), (500, 0));
    assert_eq!(parse_quota("100g").unwrap(), (100, 1));
    assert_eq!(parse_quota("2T").unwrap(), (2, 2));
    assert!(parse_quota("5 GB").is_err());
    assert_eq!(parse_quota("5GB").unwrap(), (5, 1));
    assert_eq!(parse_quota("5mb").unwrap(), (5, 0));
}

#[test]
fn a_quota_without_a_unit_or_with_nonsense_is_refused() {
    assert!(
        parse_quota("100")
            .unwrap_err()
            .to_string()
            .contains("no unit")
    );
    assert!(parse_quota("g").is_err());
    assert!(parse_quota("1.5g").is_err());
    assert!(parse_quota("-3g").is_err());
    assert!(parse_quota("5k").unwrap_err().to_string().contains("'k'"));
    assert!(
        parse_quota("0g")
            .unwrap_err()
            .to_string()
            .contains("no limit")
    );
    assert!(parse_quota("").is_err());
}

#[test]
fn a_settings_change_applies_and_is_checked() {
    let mut info = ShareInfo {
        quota: 0,
        unit: 2,
        ..ShareInfo::default()
    };
    let change = SettingsChange {
        quota: Some((5, 1)),
        network_places_hide: Some(true),
        no_permission_hide: None,
    };
    assert!(!change.is_empty());
    change.apply(&mut info);
    assert_eq!((info.quota, info.unit), (5, 1));
    assert!(info.network_places_hide);
    assert!(change.mismatch(&info).is_none());

    // A request UGOS ignored shows up as a mismatch.
    let untouched = ShareInfo::default();
    assert!(change.mismatch(&untouched).unwrap().contains("quota"));
}

#[test]
fn removing_a_quota_keeps_the_unit_it_had() {
    let mut info = ShareInfo {
        quota: 5,
        unit: 2,
        ..ShareInfo::default()
    };
    let change = SettingsChange {
        quota: Some((0, 1)),
        ..SettingsChange::default()
    };
    change.apply(&mut info);
    assert_eq!((info.quota, info.unit), (0, 2));
    assert!(change.mismatch(&info).is_none());
}

#[test]
fn an_empty_change_is_empty() {
    assert!(SettingsChange::default().is_empty());
}

#[test]
fn rules_parse_with_their_aliases() {
    assert_eq!(
        parse_rule("user:anna=rw").unwrap(),
        Rule {
            kind: PrincipalKind::User,
            name: "anna".into(),
            access: Access::ReadWrite
        }
    );
    let group = parse_rule("Group: family = READ").unwrap();
    assert_eq!(group.kind, PrincipalKind::Group);
    assert_eq!(group.name, "family");
    assert_eq!(group.access, Access::Read);
    assert_eq!(parse_rule("g:x=inherit").unwrap().access, Access::Inherit);
    assert_eq!(parse_rule("u:x=none").unwrap().access, Access::None);
}

#[test]
fn malformed_rules_say_what_is_wrong() {
    assert!(
        parse_rule("anna")
            .unwrap_err()
            .to_string()
            .contains("no '='")
    );
    assert!(
        parse_rule("anna=rw")
            .unwrap_err()
            .to_string()
            .contains("user or group")
    );
    assert!(
        parse_rule("team:anna=rw")
            .unwrap_err()
            .to_string()
            .contains("'team'")
    );
    assert!(
        parse_rule("user:=rw")
            .unwrap_err()
            .to_string()
            .contains("nobody")
    );
    assert!(
        parse_rule("user:anna=sometimes")
            .unwrap_err()
            .to_string()
            .contains("sometimes")
    );
}

#[test]
fn a_plan_resolves_names_and_marks_administrators() {
    let rules = [
        parse_rule("user:fabian=read").unwrap(),
        parse_rule("group:family=rw").unwrap(),
        parse_rule("user:anna=inherit").unwrap(),
    ];
    let changes = plan_changes(&acl(), &rules).unwrap();
    assert_eq!(changes.len(), 3);
    assert_eq!(changes[0].kind, PrincipalKind::User);
    assert_eq!(changes[0].permission_mask, 4);
    assert!(changes[0].is_admin);
    assert_eq!(changes[1].kind, PrincipalKind::Group);
    assert_eq!(changes[1].permission_mask, 7);
    assert!(!changes[1].is_admin);
    assert_eq!(changes[2].permission_mask, -1);
    assert!(!changes[2].is_admin);
}

#[test]
fn an_unknown_name_is_refused_with_the_known_ones() {
    let err = plan_changes(&acl(), &[parse_rule("user:bob=rw").unwrap()])
        .unwrap_err()
        .to_string();
    assert!(err.contains("no user 'bob'"), "{err}");
    assert!(err.contains("fabian, anna"), "{err}");
    let err = plan_changes(&acl(), &[parse_rule("group:ghosts=rw").unwrap()])
        .unwrap_err()
        .to_string();
    assert!(err.contains("no group 'ghosts'"), "{err}");
    assert!(err.contains("admin, family"), "{err}");
}

#[test]
fn a_group_is_not_looked_up_among_the_users() {
    assert!(plan_changes(&acl(), &[parse_rule("user:family=rw").unwrap()]).is_err());
    assert!(plan_changes(&acl(), &[parse_rule("group:anna=rw").unwrap()]).is_err());
}

#[test]
fn rules_the_list_shows_are_applied_and_the_others_are_named() {
    // `read` is stored as 5, and still counts as read.
    let applied = [parse_rule("group:family=read").unwrap()];
    assert_eq!(unapplied(&acl(), &applied), Vec::<String>::new());

    // The administrators group stays at read and write.
    let refused = unapplied(&acl(), &[parse_rule("group:admin=read").unwrap()]);
    assert_eq!(refused, vec!["group 'admin' stays at rw instead of read"]);

    let refused = unapplied(&acl(), &[parse_rule("user:anna=rw").unwrap()]);
    assert_eq!(refused, vec!["user 'anna' stays at inherit instead of rw"]);

    let gone = unapplied(&acl(), &[parse_rule("user:bob=rw").unwrap()]);
    assert_eq!(gone, vec!["user 'bob' is gone from the list"]);
}
