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
    unpack_folders(&dest, &staging).unwrap();
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
    let merged = merge_secrets(restored, &current);
    assert_eq!(merged.steamgriddb.api_key.as_deref(), Some("here"));
    assert_eq!(merged.github.token.as_deref(), Some("from-backup"));
}

#[test]
fn restoring_swaps_the_folders_in_and_drops_the_old_ones() {
    let dir = TestDir::new("backup-swap");
    let at = locations(&dir);
    let staging = dir.join("staging");
    fs::create_dir_all(staging.join("artwork")).unwrap();
    fs::write(staging.join("artwork/restored.png"), b"r").unwrap();
    swap_folders(&staging, &at.data_dir).unwrap();
    assert!(at.data_dir.join("artwork/restored.png").exists());
    assert!(!at.data_dir.join("artwork/cover.png").exists());
    // Not in the backup: empty now, as it was when the backup was made.
    assert!(at.data_dir.join("exe-icons").is_dir());
    assert!(!at.data_dir.join("exe-icons/icon.png").exists());
    assert!(!at.data_dir.join("artwork.old").exists() && !staging.exists());
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
    unpack_folders(&archive, &staging).unwrap();
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
