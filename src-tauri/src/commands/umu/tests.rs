use super::*;
use crate::test_util::TestDir;

fn temp_path() -> TestDir {
    TestDir::new("umu")
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

}

#[test]
fn read_status_ignores_empty_versions_and_missing_install() {
    let dir = temp_path();
    fs::create_dir_all(&dir).unwrap();
    fs::write(version_file(&dir), "   \n").unwrap();

    let status = read_status(&dir);

    assert!(!status.installed);
    assert_eq!(status.version, None);

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
}

#[test]
fn umu_local_dir_prefers_explicit_folders_path() {
    let env = crate::env::fake(&[
        ("UMU_FOLDERS_PATH", "/folders"),
        ("XDG_DATA_HOME", "/data"),
        ("HOME", "/home/u"),
    ]);
    assert_eq!(umu_local_dir(&env), Some(PathBuf::from("/folders/umu")));
}
#[test]
fn umu_local_dir_falls_back_to_xdg_data_home() {
    let env = crate::env::fake(&[("XDG_DATA_HOME", "/data"), ("HOME", "/home/u")]);
    assert_eq!(umu_local_dir(&env), Some(PathBuf::from("/data/umu")));
}
#[test]
fn umu_local_dir_uses_home_when_xdg_data_home_is_missing() {
    let env = crate::env::fake(&[("HOME", "/home/u")]);
    assert_eq!(umu_local_dir(&env), Some(PathBuf::from("/home/u/.local/share/umu")));
    assert_eq!(umu_local_dir(&crate::env::fake(&[])), None);
}
#[test]
fn runtime_present_parses_real_manifest() {
    let dir = temp_path();
    fs::create_dir_all(&dir).unwrap();
    // Verbatim from GE-Proton11-7.
    fs::write(
        dir.join("toolmanifest.vdf"),
        "\"manifest\"\n{\n  \"version\" \"2\"\n  \"commandline\" \"/proton %verb%\"\n  \"require_tool_appid\" \"4183110\"\n  \"use_sessions\" \"1\"\n}\n",
    )
    .unwrap();

    let umu_home = dir.join("folders");
    let env = crate::env::fake(&[("UMU_FOLDERS_PATH", umu_home.to_str().unwrap())]);
    assert!(!runtime_present_in(&dir, &env));
    fs::create_dir_all(umu_home.join("umu/steamrt4")).unwrap();
    assert!(!runtime_present_in(&dir, &env));
    fs::create_dir_all(umu_home.join("umu/steamrt4/steamrt4_platform_4.0.20260914.260627"))
        .unwrap();
    assert!(runtime_present_in(&dir, &env));
}

#[test]
fn runtime_present_without_manifest_is_assumed() {
    assert!(runtime_present(Path::new("/nonexistent/runner")));
}
