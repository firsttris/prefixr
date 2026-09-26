use super::*;

static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn temp_path() -> PathBuf {
    std::env::temp_dir().join(format!("prefixr-test-{}", uuid::Uuid::new_v4()))
}

fn zipapp_asset(name: &str, digest: Option<&str>) -> GitHubAsset {
    GitHubAsset {
        name: name.to_string(),
        browser_download_url: "https://example.invalid/umu.tar".to_string(),
        digest: digest.map(str::to_string),
    }
}

#[test]
fn release_helpers_pick_zipapp_and_digest() {
    let assets = vec![
        zipapp_asset("umu-runner.tar.gz", Some("sha256:ignored")),
        zipapp_asset("umu-0.1.0-zipapp.tar", Some("sha256:abc123")),
    ];

    let asset = release_zipapp_asset(&assets).unwrap();

    assert_eq!(asset.name, "umu-0.1.0-zipapp.tar");
    assert_eq!(asset_sha256_hex(asset), Some("abc123"));
}

#[test]
fn release_helpers_reject_missing_or_malformed_digest() {
    let missing = zipapp_asset("umu-0.1.0-zipapp.tar", None);
    let malformed = zipapp_asset("umu-0.1.0-zipapp.tar", Some("md5:abc123"));

    assert_eq!(asset_sha256_hex(&missing), None);
    assert_eq!(asset_sha256_hex(&malformed), None);
}

#[test]
fn read_status_requires_umu_run_and_trims_versions() {
    let dir = temp_path();
    fs::create_dir_all(umu_run_path(&dir).parent().unwrap()).unwrap();
    fs::write(umu_run_path(&dir), "#!/bin/sh\n").unwrap();
    fs::write(version_file(&dir), "  v1.2.3 \n").unwrap();

    let status = read_status(&dir);

    assert!(status.installed);
    assert_eq!(status.version.as_deref(), Some("v1.2.3"));

    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn read_status_ignores_empty_versions_and_missing_install() {
    let dir = temp_path();
    fs::create_dir_all(&dir).unwrap();
    fs::write(version_file(&dir), "   \n").unwrap();

    let status = read_status(&dir);

    assert!(!status.installed);
    assert_eq!(status.version, None);

    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn required_runtime_name_maps_known_app_ids() {
    assert_eq!(required_runtime_name("\"require_tool_appid\" \"4183110\""), Some("steamrt4"));
    assert_eq!(required_runtime_name("\"require_tool_appid\" \"1628350\""), Some("steamrt3"));
    assert_eq!(required_runtime_name("\"require_tool_appid\" \"1391110\""), Some("steamrt2"));
    assert_eq!(required_runtime_name("\"require_tool_appid\" \"999\""), None);
    assert_eq!(required_runtime_name("\"other\" \"4183110\""), None);
}

#[test]
fn runtime_dir_complete_needs_platform_marker() {
    let dir = temp_path();
    fs::create_dir_all(&dir).unwrap();
    assert!(!runtime_dir_complete(&dir));
    fs::create_dir_all(dir.join("steamrt4_payload")).unwrap();
    assert!(!runtime_dir_complete(&dir));
    fs::create_dir_all(dir.join("steamrt4_platform_4.0.20260914.260627")).unwrap();
    assert!(runtime_dir_complete(&dir));
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn umu_local_dir_prefers_explicit_folders_path() {
    let _lock = ENV_LOCK.lock().unwrap();
    let base = temp_path();
    std::env::set_var("UMU_FOLDERS_PATH", &base);
    std::env::remove_var("XDG_DATA_HOME");
    std::env::remove_var("HOME");

    assert_eq!(umu_local_dir(), Some(base.join("umu")));

    std::env::remove_var("UMU_FOLDERS_PATH");
}

#[test]
fn umu_local_dir_falls_back_to_xdg_data_home() {
    let _lock = ENV_LOCK.lock().unwrap();
    let base = temp_path();
    std::env::remove_var("UMU_FOLDERS_PATH");
    std::env::set_var("XDG_DATA_HOME", &base);
    std::env::remove_var("HOME");

    assert_eq!(umu_local_dir(), Some(base.join("umu")));

    std::env::remove_var("XDG_DATA_HOME");
}

#[test]
fn umu_local_dir_uses_home_when_xdg_data_home_is_missing() {
    let _lock = ENV_LOCK.lock().unwrap();
    let home = temp_path();
    std::env::remove_var("UMU_FOLDERS_PATH");
    std::env::remove_var("XDG_DATA_HOME");
    std::env::set_var("HOME", &home);

    assert_eq!(umu_local_dir(), Some(home.join(".local/share/umu")));

    std::env::remove_var("HOME");
}

#[test]
fn runtime_present_parses_real_manifest() {
    let _lock = ENV_LOCK.lock().unwrap();
    let dir = temp_path();
    fs::create_dir_all(&dir).unwrap();
    // Verbatim from GE-Proton11-7.
    fs::write(
        dir.join("toolmanifest.vdf"),
        "\"manifest\"\n{\n  \"version\" \"2\"\n  \"commandline\" \"/proton %verb%\"\n  \"require_tool_appid\" \"4183110\"\n  \"use_sessions\" \"1\"\n}\n",
    )
    .unwrap();

    let umu_home = dir.join("folders");
    std::env::set_var("UMU_FOLDERS_PATH", &umu_home);
    assert!(!runtime_present(&dir));
    fs::create_dir_all(umu_home.join("umu/steamrt4")).unwrap();
    assert!(!runtime_present(&dir));
    fs::create_dir_all(umu_home.join("umu/steamrt4/steamrt4_platform_4.0.20260914.260627"))
        .unwrap();
    assert!(runtime_present(&dir));
    std::env::remove_var("UMU_FOLDERS_PATH");

    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn runtime_present_without_manifest_is_assumed() {
    assert!(runtime_present(Path::new("/nonexistent/runner")));
}
