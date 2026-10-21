use super::*;

fn vars(pairs: &[(&str, &str)]) -> Vec<(OsString, OsString)> {
    pairs
        .iter()
        .map(|(key, value)| (OsString::from(key), OsString::from(value)))
        .collect()
}

fn fix(fixes: &[EnvFix], key: &str) -> Option<Option<String>> {
    fixes
        .iter()
        .find(|(k, _)| k == key)
        .map(|(_, value)| value.as_ref().map(|v| v.to_string_lossy().into_owned()))
}

/// What an AppImage built by Tauri starts Prefixr with (seen on a real
/// system).
fn appimage_env() -> Vec<(OsString, OsString)> {
    vars(&[
        ("APPDIR", "/tmp/.mount_prefixAbc"),
        ("APPIMAGE", "/home/u/AppImages/prefixr.appimage"),
        ("PYTHONHOME", "/tmp/.mount_prefixAbc/usr/"),
        ("PYTHONPATH", "/tmp/.mount_prefixAbc/usr/share/pyshared/:"),
        (
            "LD_LIBRARY_PATH",
            "/tmp/.mount_prefixAbc/usr/lib/:/tmp/.mount_prefixAbc//usr/lib/x86_64-linux-gnu/:/opt/mine/lib",
        ),
        ("GDK_PIXBUF_MODULE_FILE", "/tmp/.mount_prefixAbc//usr/lib/gdk-pixbuf-2.0/2.10.0/loaders.cache"),
        ("PYTHONDONTWRITEBYTECODE", "1"),
        ("HOME", "/home/u"),
        ("PATH", "/usr/bin:/bin"),
        ("XDG_DATA_DIRS", "/usr/local/share:/usr/share"),
    ])
}

#[test]
fn takes_the_appimage_out_of_the_environment() {
    let fixes = appimage_fixes(appimage_env());

    assert_eq!(fix(&fixes, "PYTHONHOME"), Some(None));
    assert_eq!(fix(&fixes, "PYTHONPATH"), Some(None));
    assert_eq!(fix(&fixes, "GDK_PIXBUF_MODULE_FILE"), Some(None));
    assert_eq!(fix(&fixes, "APPDIR"), Some(None));
    assert_eq!(fix(&fixes, "APPIMAGE"), Some(None));
}

#[test]
fn keeps_the_users_own_entries() {
    let fixes = appimage_fixes(appimage_env());
    assert_eq!(
        fix(&fixes, "LD_LIBRARY_PATH"),
        Some(Some("/opt/mine/lib".to_string()))
    );
}

#[test]
fn leaves_everything_else_alone() {
    let fixes = appimage_fixes(appimage_env());
    for key in ["HOME", "PATH", "XDG_DATA_DIRS", "PYTHONDONTWRITEBYTECODE"] {
        assert_eq!(fix(&fixes, key), None, "{key}");
    }
}

#[test]
fn changes_nothing_outside_an_appimage() {
    let mut env = appimage_env();
    env.retain(|(key, _)| key != "APPDIR");
    assert!(appimage_fixes(env).is_empty());
    assert!(appimage_fixes(vars(&[("APPDIR", ""), ("PYTHONHOME", "/x")])).is_empty());
}

#[test]
fn only_paths_inside_the_mount_count() {
    let fixes = appimage_fixes(vars(&[
        ("APPDIR", "/tmp/.mount_a/"),
        ("A", "/tmp/.mount_abc/lib"),
        ("B", "/tmp/.mount_a"),
        ("C", "/tmp/.mount_a/lib:/tmp/.mount_abc/lib"),
    ]));
    assert_eq!(fix(&fixes, "A"), None);
    assert_eq!(fix(&fixes, "B"), Some(None));
    assert_eq!(
        fix(&fixes, "C"),
        Some(Some("/tmp/.mount_abc/lib".to_string()))
    );
}

#[test]
fn removes_variables_only_windows_acts_on() {
    let fixes = windows_fixes(vars(&[
        ("NoDefaultCurrentDirectoryInExePath", "1"),
        ("HOME", "/home/u"),
    ]));
    assert_eq!(
        fix(&fixes, "NoDefaultCurrentDirectoryInExePath"),
        Some(None)
    );
    assert_eq!(fix(&fixes, "HOME"), None);
    assert!(windows_fixes(vars(&[("HOME", "/home/u")])).is_empty());
}

#[test]
fn outside_an_appimage_a_command_keeps_the_environment() {
    let command = host_std_command("true");
    // Only removals of Windows-only variables, if the test's own
    // environment has any.
    assert!(command
        .get_envs()
        .all(|(key, value)| value.is_none() && WINDOWS_ONLY_VARS.iter().any(|name| key == *name)));
}

#[test]
fn is_flatpak_in_reads_flatpak_id_or_container() {
    assert!(is_flatpak_in(&crate::env::fake(&[(
        "FLATPAK_ID",
        "io.github.firsttris.Prefixr"
    )])));
    assert!(is_flatpak_in(&crate::env::fake(&[(
        "container",
        "flatpak"
    )])));
    assert!(!is_flatpak_in(&crate::env::fake(&[(
        "container",
        "podman"
    )])));
    assert!(!is_flatpak_in(&crate::env::fake(&[("FLATPAK_ID", "")])));
    assert!(!is_flatpak_in(&crate::env::fake(&[])));
}

#[test]
fn host_data_home_natively_is_xdg_data_home() {
    let env = crate::env::fake(&[
        ("XDG_DATA_HOME", "/data"),
        ("HOST_XDG_DATA_HOME", "/host-data"),
        ("HOME", "/home/u"),
    ]);
    assert_eq!(host_data_home(&env), Some(PathBuf::from("/data")));
    let env = crate::env::fake(&[("HOME", "/home/u")]);
    assert_eq!(
        host_data_home(&env),
        Some(PathBuf::from("/home/u/.local/share"))
    );
    assert_eq!(host_data_home(&crate::env::fake(&[])), None);
}

#[test]
fn host_data_home_in_a_flatpak_skips_the_apps_own_data_dir() {
    // What `flatpak run` sets up on a host that sets XDG_DATA_HOME.
    let env = crate::env::fake(&[
        ("FLATPAK_ID", "io.github.firsttris.Prefixr"),
        (
            "XDG_DATA_HOME",
            "/home/u/.var/app/io.github.firsttris.Prefixr/data",
        ),
        ("HOST_XDG_DATA_HOME", "/home/u/data"),
        ("HOME", "/home/u"),
    ]);
    assert_eq!(host_data_home(&env), Some(PathBuf::from("/home/u/data")));
    // ... and on one that doesn't (Bazzite, seen in the spike).
    let env = crate::env::fake(&[
        ("FLATPAK_ID", "io.github.firsttris.Prefixr"),
        (
            "XDG_DATA_HOME",
            "/home/u/.var/app/io.github.firsttris.Prefixr/data",
        ),
        ("HOME", "/home/u"),
    ]);
    assert_eq!(
        host_data_home(&env),
        Some(PathBuf::from("/home/u/.local/share"))
    );
}

#[test]
fn flatpak_shared_runtime_dir_is_below_the_app_id() {
    let env = crate::env::fake(&[
        ("FLATPAK_ID", "io.github.firsttris.Prefixr"),
        ("XDG_RUNTIME_DIR", "/run/user/1000"),
    ]);
    assert_eq!(
        flatpak_shared_runtime_dir(&env),
        Some(PathBuf::from(
            "/run/user/1000/app/io.github.firsttris.Prefixr"
        ))
    );
    let native = crate::env::fake(&[("XDG_RUNTIME_DIR", "/run/user/1000")]);
    assert_eq!(flatpak_shared_runtime_dir(&native), None);
    let no_runtime_dir = crate::env::fake(&[("FLATPAK_ID", "io.github.firsttris.Prefixr")]);
    assert_eq!(flatpak_shared_runtime_dir(&no_runtime_dir), None);
}
