use super::*;
use crate::test_util::TestDir;

fn config() -> AppConfig {
    let mut config: AppConfig = serde_json::from_value(serde_json::json!({
        "runners_dir": "/runners",
        "games": [{
            "id": "3f1c2a52-9a43-4f53-9a8e-1b2c3d4e5f60",
            "name": "Hollow Signal",
            "exe_path": "/games/hollow.exe",
            "prefix_path": "/prefixes/a",
            "runner_id": "wine",
        }],
        "steamgriddb": { "api_key": "sgdb-secret" },
        "github": { "token": "gh-secret" },
    }))
    .unwrap();
    config.library.create_category("Kinder").unwrap();
    config
}

fn locations(dir: &Path) -> Locations {
    let data_dir = dir.join("data");
    fs::create_dir_all(data_dir.join("artwork")).unwrap();
    fs::create_dir_all(data_dir.join("exe-icons")).unwrap();
    fs::write(data_dir.join("artwork/cover.png"), b"cover").unwrap();
    fs::write(data_dir.join("exe-icons/icon.png"), b"icon").unwrap();
    Locations {
        backups_dir: data_dir.join("backups"),
        data_dir,
    }
}

#[test]
fn a_backup_holds_the_config_and_the_artwork_and_says_what_it_is() {
    let dir = TestDir::new("backup-write");
    let at = locations(&dir);
    let dest = dir.join("out/prefixr-backup.tar.gz");
    let info = write_backup(&dest, &config(), &at, false, BackupReason::Manual, "0.1.6").unwrap();
    assert_eq!((info.games, info.categories), (1, 1));
    assert!(!info.secrets_included && info.size_bytes > 0);
    assert_eq!(info.file_name, "prefixr-backup.tar.gz");
    assert!(!dir.join("out/prefixr-backup.partial").exists());

    let (manifest, restored) = read_backup(&dest).unwrap();
    assert_eq!(manifest.reason, BackupReason::Manual);
    assert_eq!(manifest.app_version, "0.1.6");
    assert_eq!(restored.games[0].name, "Hollow Signal");
    assert_eq!(restored.library.categories[0].name, "Kinder");
    // Left out unless asked for.
    assert_eq!(restored.steamgriddb.api_key, None);
    assert_eq!(restored.github.token, None);

    let staging = dir.join("staging");
    unpack_folders(&dest, &staging, u64::MAX).unwrap();
    assert_eq!(
        fs::read(staging.join("artwork/cover.png")).unwrap(),
        b"cover"
    );
    assert_eq!(
        fs::read(staging.join("exe-icons/icon.png")).unwrap(),
        b"icon"
    );
}

#[test]
fn secrets_are_only_in_a_backup_when_asked_for() {
    let dir = TestDir::new("backup-secrets");
    let dest = dir.join("with.tar.gz");
    write_backup(
        &dest,
        &config(),
        &locations(&dir),
        true,
        BackupReason::Manual,
        "1",
    )
    .unwrap();
    let (manifest, restored) = read_backup(&dest).unwrap();
    assert!(manifest.secrets_included);
    assert_eq!(restored.steamgriddb.api_key.as_deref(), Some("sgdb-secret"));
}

#[test]
fn a_restore_keeps_this_machines_secrets_where_the_backup_has_none() {
    let mut restored = config();
    restored.steamgriddb.api_key = None;
    restored.github.token = Some("from-backup".to_string());
    let mut current = config();
    current.steamgriddb.api_key = Some("here".to_string());
    current.runners_dir = "/home/me/runners".into();
    let merged = merge_local(restored, &current);
    assert_eq!(
        merged.runners_dir,
        std::path::PathBuf::from("/home/me/runners")
    );
    assert_eq!(merged.steamgriddb.api_key.as_deref(), Some("here"));
    assert_eq!(merged.github.token.as_deref(), Some("from-backup"));
}

#[test]
fn restoring_swaps_the_folders_in_and_drops_the_old_ones_on_commit() {
    let dir = TestDir::new("backup-swap");
    let at = locations(&dir);
    let staging = dir.join("staging");
    fs::create_dir_all(staging.join("artwork")).unwrap();
    fs::write(staging.join("artwork/restored.png"), b"r").unwrap();
    let swapped = put_folders_in(&staging, &at.data_dir, "t").unwrap();
    assert!(at.data_dir.join("artwork/restored.png").exists());
    assert!(!at.data_dir.join("artwork/cover.png").exists());
    // Not in the backup: empty now, as it was when the backup was made.
    assert!(at.data_dir.join("exe-icons").is_dir());
    assert!(!at.data_dir.join("exe-icons/icon.png").exists());
    assert!(at.data_dir.join("artwork.t.old/cover.png").exists());
    swapped.commit();
    assert!(!at.data_dir.join("artwork.t.old").exists());
    assert!(!at.data_dir.join("exe-icons.t.old").exists());
}

#[test]
fn a_failed_restore_puts_both_folders_back() {
    let dir = TestDir::new("backup-rollback");
    let at = locations(&dir);
    let staging = dir.join("staging");
    fs::create_dir_all(staging.join("artwork")).unwrap();
    fs::write(staging.join("artwork/restored.png"), b"r").unwrap();
    // The config save failing afterwards.
    let swapped = put_folders_in(&staging, &at.data_dir, "t").unwrap();
    swapped.roll_back();
    assert!(at.data_dir.join("artwork/cover.png").exists());
    assert!(!at.data_dir.join("artwork/restored.png").exists());
    assert!(at.data_dir.join("exe-icons/icon.png").exists());

    // Moving a folder in failing: a file where exe-icons should go back.
    let staging = dir.join("staging2");
    fs::create_dir_all(staging.join("exe-icons")).unwrap();
    fs::create_dir_all(staging.join("artwork")).unwrap();
    fs::write(at.data_dir.join("exe-icons.t.old"), b"in the way").unwrap();
    assert!(put_folders_in(&staging, &at.data_dir, "t").is_err());
    assert!(
        at.data_dir.join("artwork/cover.png").exists(),
        "artwork back as it was"
    );
    assert!(at.data_dir.join("exe-icons/icon.png").exists());
}

#[test]
fn files_that_arent_backups_are_refused() {
    let dir = TestDir::new("backup-refuse");
    let plain = dir.join("plain.tar.gz");
    fs::write(&plain, b"not even gzip").unwrap();
    assert!(read_backup(&plain)
        .unwrap_err()
        .contains("not a Prefixr backup"));

    let no_manifest = dir.join("no-manifest.tar.gz");
    write_raw_archive(&no_manifest, &[("config.json", b"{}")]);
    assert!(read_backup(&no_manifest)
        .unwrap_err()
        .contains("not a Prefixr backup"));

    let newer = dir.join("newer.tar.gz");
    let manifest = br#"{"format":99,"app_version":"9.0.0","created_at":1,"reason":"manual","games":0,"categories":0,"secrets_included":false}"#;
    write_raw_archive(
        &newer,
        &[
            ("manifest.json", manifest),
            ("config.json", br#"{"runners_dir":"/r"}"#),
        ],
    );
    assert!(read_backup(&newer)
        .unwrap_err()
        .contains("newer Prefixr (9.0.0)"));

    let bad_config = dir.join("bad-config.tar.gz");
    let manifest = br#"{"format":1,"app_version":"0.1.6","created_at":1,"reason":"manual","games":0,"categories":0,"secrets_included":false}"#;
    write_raw_archive(
        &bad_config,
        &[("manifest.json", manifest), ("config.json", b"[1,2]")],
    );
    assert!(read_backup(&bad_config).unwrap_err().contains("is invalid"));
}

#[test]
fn only_plain_paths_in_the_backed_up_folders_are_unpacked() {
    assert!(safe_folder_entry(Path::new("artwork/cover.png")));
    assert!(safe_folder_entry(Path::new("exe-icons/a/b.png")));
    assert!(!safe_folder_entry(Path::new("config.json")));
    assert!(!safe_folder_entry(Path::new("artwork/../../.bashrc")));
    assert!(!safe_folder_entry(Path::new("/etc/passwd")));
    assert!(!safe_folder_entry(Path::new(
        "stores/epic/legendary/user.json"
    )));

    // Anything else in an archive is simply not unpacked.
    let dir = TestDir::new("backup-unpack");
    let archive = dir.join("extra.tar.gz");
    write_raw_archive(
        &archive,
        &[("stores/x.json", b"{}"), ("artwork/ok.png", b"ok")],
    );
    let staging = dir.join("staging");
    unpack_folders(&archive, &staging, u64::MAX).unwrap();
    assert!(staging.join("artwork/ok.png").exists());
    assert!(!staging.join("stores").exists());
}

#[test]
fn automatic_backups_are_kept_to_the_newest_and_daily_once_a_day() {
    let dir = TestDir::new("backup-prune");
    let backups = dir.join("backups");
    fs::create_dir_all(&backups).unwrap();
    assert!(daily_due(&backups, SystemTime::now()));
    for i in 0..5 {
        let path = backups.join(format!("daily-{i}.tar.gz"));
        fs::write(&path, b"x").unwrap();
        let at = SystemTime::now() - Duration::from_secs((10 - i) * 86_400);
        fs::File::options()
            .write(true)
            .open(&path)
            .unwrap()
            .set_modified(at)
            .unwrap();
    }
    fs::write(backups.join("notes.txt"), b"not a backup").unwrap();
    // The newest is from six days ago: a daily one is due.
    assert!(daily_due(&backups, SystemTime::now()));
    let keep = backups.join("daily-0.tar.gz");
    prune(&backups, 2, Some(&keep));
    let left: Vec<_> = automatic_backups(&backups)
        .iter()
        .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
        .collect();
    assert_eq!(left, ["daily-4.tar.gz", "daily-3.tar.gz", "daily-0.tar.gz"]);
    assert!(backups.join("notes.txt").exists());

    fs::write(backups.join("daily-new.tar.gz"), b"x").unwrap();
    assert!(!daily_due(&backups, SystemTime::now()));
}

#[test]
fn backup_files_are_named_by_reason_and_utc_time() {
    let at = SystemTime::UNIX_EPOCH + Duration::from_secs(1_792_312_200); // 2026-10-18 08:30:00 UTC
    assert_eq!(
        file_name(BackupReason::Manual, at),
        "prefixr-backup-2026-10-18-083000.tar.gz"
    );
    assert_eq!(
        file_name(BackupReason::Daily, at),
        "daily-2026-10-18-083000.tar.gz"
    );
    assert_eq!(
        file_name(BackupReason::BeforeRestore, at),
        "before-restore-2026-10-18-083000.tar.gz"
    );
    assert_eq!(civil(0), (1970, 1, 1, 0, 0, 0));
    assert_eq!(civil(951_782_400), (2000, 2, 29, 0, 0, 0));
}

#[test]
fn links_and_oversized_archives_are_not_unpacked() {
    let dir = TestDir::new("backup-links");
    let archive = dir.join("links.tar.gz");
    {
        let file = fs::File::create(&archive).unwrap();
        let mut builder = tar::Builder::new(GzEncoder::new(file, Compression::default()));
        let mut header = tar::Header::new_gnu();
        header.set_entry_type(tar::EntryType::Symlink);
        header.set_size(0);
        builder
            .append_link(&mut header, "artwork/evil", "/etc")
            .unwrap();
        let mut header = tar::Header::new_gnu();
        header.set_entry_type(tar::EntryType::Link);
        header.set_size(0);
        builder
            .append_link(&mut header, "artwork/hard", "/etc/passwd")
            .unwrap();
        let mut header = tar::Header::new_gnu();
        header.set_size(4);
        header.set_mode(0o600);
        header.set_cksum();
        builder
            .append_data(&mut header, "artwork/ok.png", &b"okay"[..])
            .unwrap();
        builder.into_inner().unwrap().finish().unwrap();
    }
    let staging = dir.join("staging");
    unpack_folders(&archive, &staging, u64::MAX).unwrap();
    assert!(staging.join("artwork/ok.png").exists());
    assert!(fs::symlink_metadata(staging.join("artwork/evil")).is_err());
    assert!(fs::symlink_metadata(staging.join("artwork/hard")).is_err());

    // Over the cap: refused, and nothing left behind.
    assert!(unpack_folders(&archive, &staging, 3)
        .unwrap_err()
        .contains("more than"));
    assert!(!staging.exists());
}

#[test]
fn only_automatic_backups_are_listed_and_pruned() {
    let dir = TestDir::new("backup-own");
    let backups = dir.join("backups");
    fs::create_dir_all(&backups).unwrap();
    for name in ["daily-1.tar.gz", "before-restore-1.tar.gz", "my-own.tar.gz"] {
        fs::write(backups.join(name), b"x").unwrap();
    }
    assert_eq!(automatic_backups(&backups).len(), 2);
    prune(&backups, 0, None);
    assert!(backups.join("my-own.tar.gz").exists());
    assert!(automatic_backups(&backups).is_empty());
}

#[test]
fn backups_are_only_readable_by_their_owner() {
    use std::os::unix::fs::PermissionsExt;
    let dir = TestDir::new("backup-mode");
    let dest = dir.join("b.tar.gz");
    write_backup(
        &dest,
        &config(),
        &locations(&dir),
        true,
        BackupReason::Manual,
        "1",
    )
    .unwrap();
    assert_eq!(
        fs::metadata(&dest).unwrap().permissions().mode() & 0o777,
        0o600
    );
}

#[test]
fn the_daily_backup_is_skipped_when_nothing_changed() {
    let dir = TestDir::new("backup-changed");
    let file = dir.join("config.json");
    fs::write(&file, b"{}").unwrap();
    let written = fs::metadata(&file).unwrap().modified().unwrap();
    assert!(!changed_since(
        written + Duration::from_secs(10),
        std::slice::from_ref(&file)
    ));
    assert!(changed_since(written - Duration::from_secs(10), &[file]));
    assert!(!changed_since(SystemTime::now(), &[dir.join("not-there")]));
}

#[test]
fn leftovers_of_a_cut_off_restore_are_cleaned_up_once_old() {
    let dir = TestDir::new("backup-leftovers");
    let at = locations(&dir);
    fs::create_dir_all(&at.backups_dir).unwrap();
    for path in [
        at.data_dir.join("restore-1-2.partial"),
        at.data_dir.join("artwork.1-2.old"),
    ] {
        fs::create_dir_all(&path).unwrap();
    }
    fs::write(at.backups_dir.join("daily-x.tar.partial"), b"half").unwrap();
    // Too new: a running restore's.
    clean_leftovers(&at, SystemTime::now());
    assert!(at.data_dir.join("restore-1-2.partial").exists());
    clean_leftovers(&at, SystemTime::now() + Duration::from_secs(2 * 60 * 60));
    assert!(!at.data_dir.join("restore-1-2.partial").exists());
    assert!(!at.data_dir.join("artwork.1-2.old").exists());
    assert!(!at.backups_dir.join("daily-x.tar.partial").exists());
    assert!(at.data_dir.join("artwork/cover.png").exists());
}
