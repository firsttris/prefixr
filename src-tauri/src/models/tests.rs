use super::*;

fn globals() -> (PerformanceConfig, GraphicsConfig, MangoHudConfig, ProtonConfig) {
    (
        PerformanceConfig {
            gamemode_enabled: true,
            power_profile_enabled: true,
            inhibit_sleep_enabled: false,
        },
        GraphicsConfig {
            gamescope: GamescopeSettings {
                enabled: true,
                width: Some(1920),
                height: Some(1080),
                ..GamescopeSettings::default()
            },
            vkbasalt: VkBasaltSettings {
                enabled: true,
                ..VkBasaltSettings::default()
            },
        },
        MangoHudConfig {
            enabled: true,
            layout: MangoHudLayout::default(),
        },
        ProtonConfig {
            options: BTreeMap::from([
                ("PROTON_ENABLE_HDR".to_string(), true),
                ("PROTON_NO_NTSYNC".to_string(), true),
            ]),
        },
    )
}

#[test]
fn empty_overrides_keep_global_settings() {
    let (performance, graphics, mangohud, proton) = globals();
    let settings =
        GameOverrides::default().resolve(&performance, &graphics, &mangohud, &proton);
    assert!(settings.performance.gamemode_enabled);
    assert!(settings.performance.power_profile_enabled);
    assert_eq!(settings.graphics.gamescope, graphics.gamescope);
    assert!(settings.mangohud.enabled);
    assert_eq!(settings.proton, proton.options);
}

#[test]
fn overrides_replace_only_their_own_settings() {
    let (performance, graphics, mangohud, proton) = globals();
    let overrides = GameOverrides {
        performance: PerformanceOverrides {
            power_profile_enabled: Some(false),
            ..PerformanceOverrides::default()
        },
        graphics: GraphicsOverrides {
            gamescope: Some(GamescopeSettings {
                enabled: true,
                width: Some(1280),
                fps_limit: Some(40),
                ..GamescopeSettings::default()
            }),
            vkbasalt: None,
        },
        overlay: OverlayOverrides {
            enabled: Some(false),
            layout: None,
        },
        proton: BTreeMap::from([
            ("PROTON_NO_NTSYNC".to_string(), false),
            ("PROTON_ENABLE_WAYLAND".to_string(), true),
        ]),
    };
    let settings = overrides.resolve(&performance, &graphics, &mangohud, &proton);
    assert!(settings.performance.gamemode_enabled);
    assert!(!settings.performance.power_profile_enabled);
    assert!(settings.graphics.vkbasalt.enabled);
    assert_eq!(settings.graphics.gamescope.width, Some(1280));
    // The gamescope block is replaced as a whole, so the global height
    // doesn't leak through.
    assert_eq!(settings.graphics.gamescope.height, None);
    assert!(!settings.mangohud.enabled);
    assert_eq!(settings.mangohud.layout.preset, "standard");
    let env: Vec<_> = settings.proton_env().collect();
    assert_eq!(
        env,
        vec![
            ("PROTON_ENABLE_HDR".to_string(), "1".to_string()),
            ("PROTON_ENABLE_WAYLAND".to_string(), "1".to_string()),
            ("PROTON_NO_NTSYNC".to_string(), "0".to_string()),
        ]
    );
}

#[test]
fn mangohud_config_keeps_its_flat_json_shape() {
    let json = serde_json::to_value(MangoHudConfig::default()).unwrap();
    assert_eq!(json["enabled"], false);
    assert_eq!(json["preset"], "standard");
    let parsed: MangoHudConfig =
        serde_json::from_str(r#"{"enabled":true,"position":"top-right"}"#).unwrap();
    assert!(parsed.enabled);
    assert_eq!(parsed.layout.position, "top-right");
    assert_eq!(parsed.layout.preset, "standard");
}

#[test]
fn games_without_overrides_deserialize() {
    let game: Game = serde_json::from_str(
        r#"{"id":"00000000-0000-0000-0000-000000000000","name":"x","exe_path":"/x.exe","prefix_path":"/p","runner_id":"r"}"#,
    )
    .unwrap();
    assert!(game.overrides.performance.gamemode_enabled.is_none());
    assert!(game.overrides.graphics.gamescope.is_none());
    assert!(game.overrides.proton.is_empty());
    assert!(game.umu_id.is_none());
    assert!(game.umu_env().is_empty());
}

#[test]
fn launch_args_split_like_a_windows_command_line() {
    let split = |args: &str| split_launch_args(args).unwrap();
    assert_eq!(split("  --launcher-skip   -dx11 "), ["--launcher-skip", "-dx11"]);
    assert_eq!(
        split(r#"-path "C:\My Games\x" --name=O'Brien"#),
        ["-path", r"C:\My Games\x", "--name=O'Brien"]
    );
    assert_eq!(split(r#"--dir="a b"c "" x"#), ["--dir=a bc", "", "x"]);
    assert!(split("").is_empty());
    assert!(split_launch_args(r#"-path "C:\x"#).is_err());
}

#[test]
fn umu_ids_are_normalized() {
    assert_eq!(normalize_umu_id(None), None);
    assert_eq!(normalize_umu_id(Some("  ".into())), None);
    assert_eq!(normalize_umu_id(Some(" 1091500 ".into())).as_deref(), Some("umu-1091500"));
    assert_eq!(normalize_umu_id(Some("umu-61500".into())).as_deref(), Some("umu-61500"));
    assert_eq!(normalize_umu_store(Some("none".into())), None);
    assert_eq!(normalize_umu_store(Some("GOG".into())).as_deref(), Some("gog"));
}
