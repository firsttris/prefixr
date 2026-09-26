use super::*;

#[test]
fn checksum_asset_names_match_real_releases() {
    // Real asset-name pairs, sampled from each source's GitHub releases.
    assert_eq!(strip_tar_gz("GE-Proton11-7-x86_64.tar.gz"), "GE-Proton11-7-x86_64.sha512sum");
    assert_eq!(
        strip_tar_xz("proton-cachyos-11.0-20260703-slr-x86_64.tar.xz"),
        "proton-cachyos-11.0-20260703-slr-x86_64.sha512sum"
    );
    assert_eq!(shared_sha256sums_name("wine-11.18-staging-amd64-wow64.tar.xz"), "sha256sums.txt");
}

#[test]
fn extract_single_digest_takes_the_first_token() {
    let contents = "7db87e9787e20c35cbdac26018431d5794626b626e4067b050684e45a88cc2ca229d7d263519eafb2e168cde5bef57611065d159d3685aaec152ccb9abe3073f  GE-Proton11-7-x86_64.tar.gz\n";
    assert_eq!(
        extract_single_digest(contents, "GE-Proton11-7-x86_64.tar.gz"),
        Some("7db87e9787e20c35cbdac26018431d5794626b626e4067b050684e45a88cc2ca229d7d263519eafb2e168cde5bef57611065d159d3685aaec152ccb9abe3073f".to_string())
    );
}

#[test]
fn extract_shared_digest_finds_the_matching_line() {
    let contents = "\
5717663b0541afe99efee507e1eac88d452b8adff0d3a5065abeed527890a3e8  wine-11.18-amd64.tar.xz
f899879b8c37e0b20adca19d147cf77436f3f1a37bf16d08d27fa7137a52b9ba  wine-11.18-amd64-wow64.tar.xz
";
    assert_eq!(
        extract_shared_digest(contents, "wine-11.18-amd64-wow64.tar.xz"),
        Some("f899879b8c37e0b20adca19d147cf77436f3f1a37bf16d08d27fa7137a52b9ba".to_string())
    );
    assert_eq!(extract_shared_digest(contents, "wine-11.18-nonexistent.tar.xz"), None);
}

#[test]
fn extract_shared_digest_strips_binary_mode_marker() {
    let contents = "5717663b0541afe99efee507e1eac88d452b8adff0d3a5065abeed527890a3e8 *wine-11.18-amd64.tar.xz\n";
    assert_eq!(
        extract_shared_digest(contents, "wine-11.18-amd64.tar.xz"),
        Some("5717663b0541afe99efee507e1eac88d452b8adff0d3a5065abeed527890a3e8".to_string())
    );
}

#[test]
fn asset_matchers_pick_the_right_asset() {
    // Real release asset names, sampled from each source's GitHub API.
    assert!((find_source("proton-ge").unwrap().matches_asset)("GE-Proton9-20.tar.gz"));
    assert!(!(find_source("proton-ge").unwrap().matches_asset)("GE-Proton9-20.sha512sum"));

    assert!((find_source("wine-kron4ek").unwrap().matches_asset)(
        "wine-11.18-staging-amd64-wow64.tar.xz"
    ));
    assert!(!(find_source("wine-kron4ek").unwrap().matches_asset)(
        "wine-11.18-amd64-wow64.tar.xz"
    ));
    assert!(!(find_source("wine-kron4ek").unwrap().matches_asset)(
        "wine-11.18-staging-amd64.tar.xz"
    ));
    assert!(!(find_source("wine-kron4ek").unwrap().matches_asset)(
        "wine-11.18-staging-tkg-amd64-wow64.tar.xz"
    ));
    assert!(!(find_source("wine-kron4ek").unwrap().matches_asset)(
        "wine-11.18-staging-x86.tar.xz"
    ));
    assert!(!(find_source("wine-kron4ek").unwrap().matches_asset)("sha256sums.txt"));

    assert!((find_source("proton-cachyos").unwrap().matches_asset)(
        "proton-cachyos-11.0-20260703-slr-x86_64.tar.xz"
    ));
    assert!(!(find_source("proton-cachyos").unwrap().matches_asset)(
        "proton-cachyos-11.0-20260703-slr-x86_64_v3.tar.xz"
    ));
    assert!(!(find_source("proton-cachyos").unwrap().matches_asset)(
        "proton-cachyos-11.0-20260703-slr-arm64.tar.xz"
    ));
}

#[test]
fn download_requests_are_checked() {
    let source = find_source("proton-ge").unwrap();
    let url = "https://github.com/GloriousEggroll/proton-ge-custom/releases/download/GE-Proton11-7/GE-Proton11-7.tar.gz";
    assert!(check_download_request(source, "GE-Proton11-7", url).is_ok());
    for tag in ["", "..", ".hidden", "../x", "a/b", "/abs"] {
        assert!(check_download_request(source, tag, url).is_err(), "{tag}");
    }
    for url in [
        "https://evil.example/GloriousEggroll/proton-ge-custom/releases/download/x.tar.gz",
        "https://github.com/Kron4ek/Wine-Builds/releases/download/x/x.tar.xz",
        "http://github.com/GloriousEggroll/proton-ge-custom/releases/download/x.tar.gz",
    ] {
        assert!(check_download_request(source, "x", url).is_err(), "{url}");
    }
}

#[test]
fn move_renames_the_extracted_dir_and_cleans_up() {
    let dir = std::env::temp_dir().join(format!("prefixr-test-{}", uuid::Uuid::new_v4()));
    fs::create_dir_all(&dir).unwrap();

    // An archive whose top-level folder name doesn't match the GitHub
    // tag (some sources name it after the build, not the tag), while a
    // second extraction runs next to it.
    let extract = extraction_dir(&dir);
    fs::create_dir_all(extract.join("wine-lutris-GE-Proton8-26-x86_64/bin")).unwrap();
    let other = extraction_dir(&dir);
    fs::create_dir_all(other.join("wine-11.18-staging-amd64-wow64")).unwrap();

    move_extracted_dir(&extract, &dir.join("GE-Proton8-26")).unwrap();
    assert!(dir.join("GE-Proton8-26/bin").is_dir());
    assert!(!extract.exists());

    move_extracted_dir(&other, &dir.join("wine-11.18")).unwrap();
    assert!(dir.join("wine-11.18").is_dir());

    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn move_rejects_archives_without_a_single_dir() {
    let dir = std::env::temp_dir().join(format!("prefixr-test-{}", uuid::Uuid::new_v4()));
    let extract = extraction_dir(&dir);
    fs::create_dir_all(extract.join("a")).unwrap();
    fs::create_dir_all(extract.join("b")).unwrap();

    assert!(move_extracted_dir(&extract, &dir.join("target")).is_err());
    assert!(!extract.exists());
    assert!(!dir.join("target").exists());

    fs::remove_dir_all(&dir).unwrap();
}
