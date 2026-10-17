use super::*;
use crate::test_util::TestDir;

const TOOL: Helper = Helper {
    name: "tool",
    display_name: "Tool",
    version: "1.2.3",
    assets: &[HelperAsset {
        arch: "x86_64",
        url: "https://example.com/tool",
        // sha256("hello")
        sha256: "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824",
    }],
    source_url: "https://example.com/source",
};

#[test]
fn every_pin_is_a_full_sha256_from_github_releases() {
    for helper in [&LEGENDARY, &GOGDL] {
        assert!(!helper.assets.is_empty());
        for asset in helper.assets {
            assert_eq!(asset.sha256.len(), 64, "{}", asset.url);
            assert!(asset.sha256.chars().all(|c| c.is_ascii_hexdigit()));
            // Tagged `0.20.43` or `v1.3.1`, depending on the project.
            let release = |tag: String| format!("{}/releases/download/{tag}/", helper.source_url);
            assert!(
                asset.url.starts_with(&release(helper.version.to_string()))
                    || asset
                        .url
                        .starts_with(&release(format!("v{}", helper.version))),
                "{} isn't a release of {} {}",
                asset.url,
                helper.name,
                helper.version
            );
        }
        // The two architectures Linux gaming PCs and handhelds run on.
        assert!(asset_for(helper, "x86_64").is_some());
        assert!(asset_for(helper, "aarch64").is_some());
    }
}

#[test]
fn verifies_the_download_against_the_pin() {
    let asset = &TOOL.assets[0];
    assert!(verify(asset, b"hello").is_ok());
    let error = verify(asset, b"hello!").unwrap_err();
    assert!(error.contains("Checksum mismatch"), "{error}");
}

#[test]
fn each_version_has_a_file_of_its_own() {
    assert_eq!(
        helper_path(Path::new("/data/helpers"), &TOOL),
        PathBuf::from("/data/helpers/tool-1.2.3")
    );
    assert_eq!(asset_for(&TOOL, "riscv64").map(|a| a.url), None);
}

#[test]
fn finds_older_versions_but_not_other_tools() {
    let dir = TestDir::new("helpers");
    for name in [
        "tool-1.2.3",
        "tool-1.0.0",
        "tool-1.2.3.download",
        "tooling-2",
        "other-1",
    ] {
        fs::write(dir.join(name), "").unwrap();
    }
    let keep = helper_path(&dir, &TOOL);
    let mut stale: Vec<String> = stale_versions(&dir, &TOOL, &keep)
        .iter()
        .map(|path| path.file_name().unwrap().to_string_lossy().into_owned())
        .collect();
    stale.sort();
    assert_eq!(stale, ["tool-1.0.0", "tool-1.2.3.download"]);
    // Nothing to keep: every file of the tool, for removing it.
    assert_eq!(stale_versions(&dir, &TOOL, Path::new("")).len(), 3);
    assert!(stale_versions(&dir.join("missing"), &TOOL, &keep).is_empty());
}
