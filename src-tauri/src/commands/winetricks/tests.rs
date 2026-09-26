use super::{list_installed_winetricks_verbs, parse_verb_catalogue, uses_umu_winetricks};
use crate::models::{Runner, RunnerKind};
use std::fs;
use std::path::PathBuf;
use uuid::Uuid;

fn temp_path(name: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!("prefixr-{name}-{}", Uuid::new_v4()));
    fs::create_dir_all(&path).unwrap();
    path
}

#[test]
fn parses_only_supported_winetricks_categories() {
    let script = r#"
w_metadata corefonts fonts \
title="Microsoft Core Fonts"
load_corefonts()

w_metadata vcrun2022 dlls \
foo=bar
title="Visual C++ 2022"
load_vcrun2022()

w_metadata steam apps \
title="Steam"
load_steam()

w_metadata physx dlls \
foo=bar
bar=baz
load_physx()
"#;

    assert_eq!(
        parse_verb_catalogue(script)
            .into_iter()
            .map(|verb| (verb.id, verb.category, verb.title))
            .collect::<Vec<_>>(),
        vec![
            (
                "corefonts".to_string(),
                "fonts".to_string(),
                "Microsoft Core Fonts".to_string(),
            ),
            (
                "vcrun2022".to_string(),
                "dlls".to_string(),
                "Visual C++ 2022".to_string(),
            ),
            (
                "physx".to_string(),
                "dlls".to_string(),
                "physx".to_string(),
            ),
        ]
    );
}

#[test]
fn reads_installed_winetricks_verbs_and_ignores_blank_lines() {
    let prefix = temp_path("winetricks-installed");
    fs::write(
        prefix.join("winetricks.log"),
        " corefonts \n\nvcrun2022\n  dxvk  \n",
    )
    .unwrap();

    assert_eq!(
        list_installed_winetricks_verbs(prefix.display().to_string()).unwrap(),
        vec![
            "corefonts".to_string(),
            "vcrun2022".to_string(),
            "dxvk".to_string(),
        ]
    );

    let _ = fs::remove_dir_all(prefix);
}

#[test]
fn missing_winetricks_log_is_not_an_error() {
    let prefix = temp_path("winetricks-missing-log");

    assert_eq!(
        list_installed_winetricks_verbs(prefix.display().to_string()).unwrap(),
        Vec::<String>::new()
    );

    let _ = fs::remove_dir_all(prefix);
}

#[test]
fn uses_umu_winetricks_only_for_proton_with_bundled_script() {
    let proton_dir = temp_path("runner-proton");
    fs::create_dir_all(proton_dir.join("protonfixes")).unwrap();
    fs::write(proton_dir.join("protonfixes/winetricks"), "#!/bin/sh\n").unwrap();

    let proton_runner = Runner {
        id: "proton".to_string(),
        name: "GE-Proton".to_string(),
        path: proton_dir.clone(),
        kind: RunnerKind::Proton,
    };
    let wine_runner = Runner {
        id: "wine".to_string(),
        name: "Wine".to_string(),
        path: proton_dir.clone(),
        kind: RunnerKind::Wine,
    };

    assert!(uses_umu_winetricks(&proton_runner));
    assert!(!uses_umu_winetricks(&wine_runner));

    let _ = fs::remove_dir_all(proton_dir);
}
