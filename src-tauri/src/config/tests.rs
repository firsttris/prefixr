use super::*;

#[test]
fn migrates_flat_performance_and_overrides() {
    let mut value = json!({
        "runners_dir": "/r",
        "performance": {
            "gamemode_enabled": true,
            "power_profile_enabled": false,
            "inhibit_sleep_enabled": true,
            "vkbasalt_enabled": true,
            "vkbasalt_sharpen": true,
            "vkbasalt_sharpness": 0.6,
            "vkbasalt_smaa": false,
            "vkbasalt_deband": true,
            "gamescope_enabled": true,
            "gamescope_width": 1280,
            "gamescope_height": null,
            "gamescope_fps_limit": 40,
            "gamescope_fullscreen": false
        },
        "games": [{
            "id": "00000000-0000-0000-0000-000000000000",
            "name": "x",
            "exe_path": "/x.exe",
            "prefix_path": "/p",
            "runner_id": "r",
            "overrides": {
                "mangohud_enabled": false,
                "gamemode_enabled": null,
                "vkbasalt": null,
                "gamescope": {
                    "enabled": true, "width": 800, "height": 600,
                    "fps_limit": null, "fullscreen": true
                },
                "proton_options": { "PROTON_ENABLE_HDR": true }
            }
        }]
    });
    migrate(&mut value);
    let config: AppConfig = serde_json::from_value(value).unwrap();

    assert!(config.performance.gamemode_enabled);
    assert!(config.performance.inhibit_sleep_enabled);
    assert!(config.graphics.vkbasalt.enabled);
    assert!((config.graphics.vkbasalt.sharpness - 0.6).abs() < 1e-6);
    assert!(config.graphics.gamescope.enabled);
    assert_eq!(config.graphics.gamescope.width, Some(1280));
    assert_eq!(config.graphics.gamescope.fps_limit, Some(40));

    let overrides = &config.games[0].overrides;
    assert_eq!(overrides.overlay.enabled, Some(false));
    assert!(overrides.performance.gamemode_enabled.is_none());
    assert!(overrides.graphics.vkbasalt.is_none());
    assert_eq!(overrides.graphics.gamescope.as_ref().unwrap().width, Some(800));
    assert_eq!(overrides.proton.get("PROTON_ENABLE_HDR"), Some(&true));
}

#[test]
fn disk_json_leaves_out_icons() {
    let value = json!({
        "runners_dir": "/r",
        "games": [{
            "id": "00000000-0000-0000-0000-000000000000",
            "name": "x",
            "exe_path": "/x.exe",
            "prefix_path": "/p",
            "runner_id": "r",
            "icon": "data:image/png;base64,AAAA"
        }]
    });
    let config: AppConfig = serde_json::from_value(value).unwrap();
    assert!(config.games[0].icon.is_some());

    let written: Value = serde_json::from_str(&to_disk_json(&config).unwrap()).unwrap();
    assert!(written["games"][0].get("icon").is_none());
    assert_eq!(written["games"][0]["name"], "x");
}

#[test]
fn leaves_current_shape_alone() {
    let mut value = json!({
        "runners_dir": "/r",
        "graphics": { "gamescope": { "enabled": true } },
        "games": [{
            "id": "00000000-0000-0000-0000-000000000000",
            "name": "x",
            "exe_path": "/x.exe",
            "prefix_path": "/p",
            "runner_id": "r",
            "overrides": { "overlay": { "enabled": true }, "proton": { "A": false } }
        }]
    });
    let before = value.clone();
    migrate(&mut value);
    assert_eq!(value, before);
}