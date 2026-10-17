use super::*;
use crate::test_util::TestDir;

#[test]
fn opens_only_logs_below_the_logs_folder() {
    let root = TestDir::new("logs-root");
    let game = root.join("game");
    fs::create_dir_all(&game).unwrap();
    let log = game.join("1.txt");
    fs::write(&log, "").unwrap();
    let outside = TestDir::new("logs-outside");
    let secret = outside.join("secret.txt");
    fs::write(&secret, "").unwrap();

    assert_eq!(
        log_file_in(&root, &log).unwrap(),
        fs::canonicalize(&log).unwrap()
    );
    assert!(log_file_in(&root, &secret).is_err());
    assert!(log_file_in(&root, &game.join("../../../etc/passwd")).is_err());
    assert!(log_file_in(&root, &game).is_err(), "a folder isn't a log");
    assert!(log_file_in(&root, &game.join("missing.txt")).is_err());
}

#[test]
fn keeps_only_the_newest_logs() {
    let dir = TestDir::new("dir");
    fs::create_dir_all(&dir).unwrap();
    // Seconds-named logs from older versions, then millisecond ones.
    for name in ["1790000000", "1790000001", "1790000002000", "1790000003000"] {
        fs::write(dir.join(format!("{name}.txt")), "").unwrap();
    }
    fs::write(dir.join("notes.md"), "").unwrap();

    prune_logs(&dir, 2);
    let mut left: Vec<String> = fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    left.sort();
    assert_eq!(left, ["1790000002000.txt", "1790000003000.txt", "notes.md"]);

    let log = new_log_file(&dir).unwrap();
    assert!(log.is_file());
}
