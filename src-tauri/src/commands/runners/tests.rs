use super::*;
use crate::test_util::TestDir;

#[test]
fn deleting_a_linked_runner_keeps_its_target() {
    // What `delete_runner` relies on for a runner linked in from
    // elsewhere: `remove_dir_all` doesn't follow the link.
    let dir = TestDir::new("dir");
    let target = dir.join("steam/GE-Proton11-7");
    fs::create_dir_all(&target).unwrap();
    fs::write(target.join("proton"), "").unwrap();
    let link = dir.join("runners/GE-Proton11-7");
    fs::create_dir_all(link.parent().unwrap()).unwrap();
    std::os::unix::fs::symlink(&target, &link).unwrap();
    assert_eq!(scan_runners(link.parent().unwrap()).unwrap().len(), 1);

    fs::remove_dir_all(&link).unwrap();
    assert!(!link.exists());
    assert!(target.join("proton").is_file());
}
