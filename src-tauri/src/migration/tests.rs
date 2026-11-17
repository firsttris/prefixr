use super::*;
use crate::env::fake;
use crate::test_util::TestDir;

#[test]
fn identifier_is_the_one_in_tauri_conf() {
    let conf: Value = serde_json::from_str(include_str!("../../tauri.conf.json")).unwrap();
    assert_eq!(conf["identifier"], IDENTIFIER);
}

#[test]
fn recognizes_prefixr_commands() {
    assert!(mentions_prefixr("\"/usr/bin/prefixr\""));
    assert!(mentions_prefixr(
        "/home/u/Apps/prefixr_0.2.2_amd64.AppImage"
    ));
    assert!(mentions_prefixr(
        "run io.github.firsttris.Prefixr --run 1234"
    ));
    assert!(!mentions_prefixr(
        "run io.github.firsttris.Laufzeit --run 1234"
    ));
}

fn write(path: &Path, text: &str) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
}

#[test]
fn moves_the_folders_and_the_paths_in_the_config() {
    let home = TestDir::new("migrate-home");
    let home_str = home.to_str().unwrap().to_string();
    let env = fake(&[("HOME", &home_str)]);
    let old_data = home.join(".local/share/com.tristan.prefixr");
    let config = format!(
        r#"{{
  "runners_dir": "{data}/runners",
  "prefixes": [{{"path": "{data}/prefixes/gog-quake"}}, {{"path": "/mnt/games/pfx"}}],
  "games": [{{"name": "{data} is not a path"}}]
}}"#,
        data = old_data.display()
    );
    write(
        &home.join(".config/com.tristan.prefixr/config.json"),
        &config,
    );
    write(&old_data.join("runners/GE-Proton/proton"), "");
    write(
        &home.join(".cache/com.tristan.prefixr/umu-database.json"),
        "[]",
    );

    migrate(&env).unwrap();

    let new_data = home.join(".local/share/com.tristan.laufzeit");
    assert!(new_data.join("runners/GE-Proton/proton").is_file());
    assert!(home
        .join(".cache/com.tristan.laufzeit/umu-database.json")
        .is_file());
    assert!(!home.join(".config/com.tristan.prefixr").exists());
    assert!(!home.join(".cache/com.tristan.prefixr").exists());
    // The data folder's old path still leads there.
    assert_eq!(
        fs::read_link(&old_data).unwrap(),
        PathBuf::from("com.tristan.laufzeit")
    );
    assert!(old_data.join("runners/GE-Proton/proton").is_file());

    let raw = fs::read_to_string(home.join(".config/com.tristan.laufzeit/config.json")).unwrap();
    let value: Value = serde_json::from_str(&raw).unwrap();
    assert_eq!(
        value["runners_dir"],
        new_data.join("runners").to_string_lossy().as_ref()
    );
    assert_eq!(
        value["prefixes"][0]["path"],
        new_data
            .join("prefixes/gog-quake")
            .to_string_lossy()
            .as_ref()
    );
    assert_eq!(value["prefixes"][1]["path"], "/mnt/games/pfx");
    // Only whole path components are changed.
    assert_eq!(
        value["games"][0]["name"],
        format!("{} is not a path", old_data.display())
    );

    // Once moved, a second start changes nothing.
    migrate(&env).unwrap();
    assert!(home
        .join(".config/com.tristan.laufzeit/config.json")
        .is_file());
}

#[test]
fn leaves_folders_alone_that_exist_under_the_new_name() {
    let home = TestDir::new("migrate-both");
    let env = fake(&[("HOME", home.to_str().unwrap())]);
    write(
        &home.join(".config/com.tristan.prefixr/config.json"),
        "{\"old\": 1}",
    );
    write(
        &home.join(".config/com.tristan.laufzeit/config.json"),
        "{\"new\": 1}",
    );

    migrate(&env).unwrap();

    assert_eq!(
        fs::read_to_string(home.join(".config/com.tristan.laufzeit/config.json")).unwrap(),
        "{\"new\": 1}"
    );
    assert!(home
        .join(".config/com.tristan.prefixr/config.json")
        .is_file());
}

#[test]
fn nothing_to_do_on_a_new_install() {
    let home = TestDir::new("migrate-new");
    let env = fake(&[("HOME", home.to_str().unwrap())]);
    migrate(&env).unwrap();
    assert!(fs::read_dir(&*home).unwrap().next().is_none());
}

#[test]
fn honors_the_xdg_variables() {
    let home = TestDir::new("migrate-xdg");
    let config_home = home.join("cfg");
    let env = fake(&[
        ("HOME", home.to_str().unwrap()),
        ("XDG_CONFIG_HOME", config_home.to_str().unwrap()),
        // Relative: ignored, as Tauri does.
        ("XDG_DATA_HOME", "relative/data"),
    ]);
    write(&config_home.join("com.tristan.prefixr/config.json"), "{}");
    write(&home.join(".local/share/com.tristan.prefixr/x"), "");

    migrate(&env).unwrap();

    assert!(config_home
        .join("com.tristan.laufzeit/config.json")
        .is_file());
    assert!(home.join(".local/share/com.tristan.laufzeit/x").is_file());
}

#[test]
fn in_a_flatpak_paths_follow_the_moved_app_folder() {
    let pairs = moved_paths(&fake(&[
        ("HOME", "/home/u"),
        ("FLATPAK_ID", "io.github.firsttris.Laufzeit"),
        (
            "XDG_DATA_HOME",
            "/home/u/.var/app/io.github.firsttris.Laufzeit/data",
        ),
    ]));
    let mut value = serde_json::json!({
        "runners_dir": "/home/u/.var/app/io.github.firsttris.Prefixr/data/com.tristan.prefixr/runners",
        "exe": "/home/u/.var/app/io.github.firsttris.Prefixr/data/Games/a.exe",
        "other": "/home/u/.var/app/com.valvesoftware.Steam/x",
    });
    assert!(rewrite_paths(&mut value, &pairs));
    assert_eq!(
        value["runners_dir"],
        "/home/u/.var/app/io.github.firsttris.Laufzeit/data/com.tristan.laufzeit/runners"
    );
    assert_eq!(
        value["exe"],
        "/home/u/.var/app/io.github.firsttris.Laufzeit/data/Games/a.exe"
    );
    assert_eq!(value["other"], "/home/u/.var/app/com.valvesoftware.Steam/x");
}

#[test]
fn replaces_prefixrs_skill() {
    let home = TestDir::new("migrate-skill");
    let env = fake(&[("HOME", home.to_str().unwrap())]);
    let old = home.join(".claude/skills/prefixr/SKILL.md");
    write(
        &old,
        "---\nname: prefixr\n---\n\n# Prefixr\n\nRun `prefixr game list`.\n",
    );

    migrate_skill(&env);

    assert!(!home.join(".claude/skills/prefixr").exists());
    let new = fs::read_to_string(home.join(".claude/skills/laufzeit/SKILL.md")).unwrap();
    assert!(new.contains("# Laufzeit"));
}

#[test]
fn keeps_a_skill_folder_with_more_in_it() {
    let home = TestDir::new("migrate-skill-more");
    let env = fake(&[("HOME", home.to_str().unwrap())]);
    write(
        &home.join(".claude/skills/prefixr/SKILL.md"),
        "\n# Prefixr\n",
    );
    write(&home.join(".claude/skills/prefixr/notes.md"), "mine");

    migrate_skill(&env);

    assert!(home.join(".claude/skills/prefixr/notes.md").is_file());
    assert!(home.join(".claude/skills/laufzeit/SKILL.md").is_file());
}
