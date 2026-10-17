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
fn outside_an_appimage_a_command_keeps_the_environment() {
    let command = host_std_command("true");
    assert_eq!(command.get_envs().count(), 0);
}
