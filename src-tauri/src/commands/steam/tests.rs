use super::*;
use std::path::PathBuf;

fn game(name: &str) -> Game {
    serde_json::from_value(serde_json::json!({
        "id": "6f2c1a3e-0000-4000-8000-000000000001",
        "name": name,
        "exe_path": "/g/game.exe",
        "prefix_path": "/p",
        "runner_id": "r",
    }))
    .unwrap()
}

fn string<'a>(fields: &'a Map, key: &str) -> &'a str {
    match binary_vdf::get(fields, key) {
        Some(Value::String(s)) => s,
        other => panic!("{key}: {other:?}"),
    }
}

#[test]
fn app_id_is_stable_and_marked_non_steam() {
    let id = game("x").id;
    assert_eq!(shortcut_app_id(id), shortcut_app_id(id));
    assert!(shortcut_app_id(id) & 0x8000_0000 != 0);
}

fn native_launch() -> OwnLaunch {
    OwnLaunch {
        program: PathBuf::from("/apps/prefixr"),
        args: Vec::new(),
        start_dir: PathBuf::from("/apps"),
    }
}

#[test]
fn a_flatpak_shortcut_runs_flatpak() {
    let env = crate::env::fake(&[
        ("FLATPAK_ID", "io.github.firsttris.Prefixr"),
        ("HOME", "/home/u"),
    ]);
    let launch = own_launch(&env).unwrap();
    let mut shortcuts = Map::new();
    let g = game("x");
    upsert_shortcut(&mut shortcuts, &g, 42, &launch, None);

    let Value::Map(fields) = &shortcuts[0].1 else {
        panic!()
    };
    assert_eq!(string(fields, "Exe"), "\"flatpak\"");
    assert_eq!(string(fields, "StartDir"), "\"/home/u\"");
    assert_eq!(
        string(fields, "LaunchOptions"),
        "run io.github.firsttris.Prefixr --run 6f2c1a3e-0000-4000-8000-000000000001"
    );
    // Found again by its id, also after an export from the native build.
    assert_eq!(find_shortcut(&shortcuts, g.id), Some(0));
}

#[test]
fn adds_then_updates_the_same_entry() {
    let other: Map = vec![("AppName".into(), Value::String("Other".into()))];
    let mut shortcuts: Map = vec![("5".into(), Value::Map(other))];
    let launch = native_launch();

    let first = game("Old");
    upsert_shortcut(&mut shortcuts, &first, 42, &launch, None);
    assert_eq!(shortcuts.len(), 2);
    assert_eq!(shortcuts[0].0, "0");
    assert_eq!(shortcuts[1].0, "1");

    // The user hid it in Steam; a rename in Prefixr keeps that.
    let Value::Map(fields) = &mut shortcuts[1].1 else {
        panic!()
    };
    binary_vdf::set(fields, "IsHidden", Value::Int(1));
    let renamed = game("New");
    assert_eq!(existing_app_id(&shortcuts, renamed.id), Some(42));
    upsert_shortcut(
        &mut shortcuts,
        &renamed,
        42,
        &launch,
        Some(Path::new("/i.png")),
    );

    assert_eq!(shortcuts.len(), 2);
    let Value::Map(fields) = &shortcuts[1].1 else {
        panic!()
    };
    assert_eq!(string(fields, "AppName"), "New");
    assert_eq!(string(fields, "Exe"), "\"/apps/prefixr\"");
    assert_eq!(string(fields, "StartDir"), "\"/apps\"");
    assert_eq!(string(fields, "icon"), "/i.png");
    assert_eq!(
        string(fields, "LaunchOptions"),
        "--run 6f2c1a3e-0000-4000-8000-000000000001"
    );
    assert_eq!(binary_vdf::get(fields, "IsHidden"), Some(&Value::Int(1)));
}

#[test]
fn removes_only_the_games_entry() {
    let other: Map = vec![("AppName".into(), Value::String("Other".into()))];
    let mut shortcuts: Map = vec![("0".into(), Value::Map(other))];
    let g = game("x");
    upsert_shortcut(&mut shortcuts, &g, 42, &native_launch(), None);
    shortcuts.swap(0, 1);

    assert_eq!(remove_entry(&mut shortcuts, g.id), Some(Some(42)));
    assert_eq!(shortcuts.len(), 1);
    assert_eq!(shortcuts[0].0, "0");
    assert_eq!(remove_entry(&mut shortcuts, g.id), None);
}
