use super::{Place, in_recycle};

#[test]
fn only_a_path_with_a_recycle_folder_is_a_bin_path() {
    assert!(in_recycle("/volume1/docs/#recycle"));
    assert!(in_recycle("/volume1/docs/#recycle/a/b.txt"));
    assert!(in_recycle("/home/fabian/#recycle/x"));
    assert!(!in_recycle("/volume1/docs/a.txt"));
    assert!(!in_recycle("/volume1/docs/#recycled/a"));
    assert!(!in_recycle("/volume1/#recycle-notes/a"));
}

#[test]
fn the_three_roots_have_their_codes() {
    assert_eq!(Place::Personal.root(), Some(2));
    assert_eq!(Place::Common.root(), Some(3));
    assert_eq!(Place::Users.root(), Some(4));
    assert_eq!(Place::Path("/x".into()).root(), None);
}
