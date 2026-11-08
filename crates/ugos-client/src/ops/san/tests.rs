use super::check;

#[test]
fn names_iqns_and_chap_follow_the_web_ui() {
    assert!(check::name("tank-1_a").is_ok());
    assert!(check::name("a b").is_err());
    assert!(check::name(&"a".repeat(33)).is_err());
    assert!(check::name("").is_err());
    assert!(check::description("fine text").is_ok());
    assert!(check::description("no:colon").is_err());
    assert!(check::target_iqn("iqn.2025-03.com.ugreen:target-1.abc").is_ok());
    assert!(check::target_iqn("iqn with space").is_err());
    assert!(check::initiator_iqn("iqn.1991-05.com.microsoft:pc").is_ok());
    assert!(check::initiator_iqn(&"a".repeat(224)).is_err());
    assert!(check::chap_user("User1").is_ok());
    assert!(check::chap_user("toolongusername1").is_err());
    assert!(check::chap_password("abcdefghij12").is_ok());
    assert!(check::chap_password("short").is_err());
    assert!(check::chap_password("abcdefghij12345678").is_err());
}

#[test]
fn the_iqn_prefix_is_lowercased_and_limited() {
    assert_eq!(
        check::iqn_prefix("IQN.2025-03.Com.Ugreen").unwrap(),
        "iqn.2025-03.com.ugreen"
    );
    assert!(check::iqn_prefix("iqn_bad").is_err());
    assert!(check::iqn_prefix(&"a".repeat(96)).is_err());
}

#[test]
fn sizes_are_whole_units_of_gb_or_tb() {
    assert_eq!(check::size("10").unwrap(), (10, "GB"));
    assert_eq!(check::size("10G").unwrap(), (10, "GB"));
    assert_eq!(check::size("10gb").unwrap(), (10, "GB"));
    assert_eq!(check::size("2T").unwrap(), (2, "TB"));
    assert!(check::size("1.5T").is_err());
    assert!(check::size("0").is_err());
    assert!(check::size("5M").is_err());
    assert_eq!(check::bytes(1, "GB"), 1 << 30);
    assert_eq!(check::bytes(2, "TB"), 2 << 40);
}
