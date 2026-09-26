use super::*;

#[test]
fn keeps_only_the_newest_logs() {
    let dir = std::env::temp_dir().join(format!("prefixr-test-{}", uuid::Uuid::new_v4()));
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
    fs::remove_dir_all(&dir).unwrap();
}
