use super::*;
use std::os::unix::fs::PermissionsExt;
use std::sync::{Mutex as StdMutex, OnceLock};

fn env_lock() -> &'static StdMutex<()> {
    static LOCK: OnceLock<StdMutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| StdMutex::new(()))
}

fn temp_path(name: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!("prefixr-test-{name}-{}", Uuid::new_v4()));
    fs::create_dir_all(&path).unwrap();
    path
}

fn set_executable(path: &Path) {
    let mut perms = fs::metadata(path).unwrap().permissions();
    perms.set_mode(0o755);
    fs::set_permissions(path, perms).unwrap();
}

fn write_prefix_arch(prefix_dir: &Path, arch: &str) {
    fs::write(
        prefix_dir.join("system.reg"),
        format!(
            "WINE REGISTRY Version 2\n;; All keys relative to \\\\Machine\n\n#arch={arch}\n\n"
        ),
    )
    .unwrap();
}

#[test]
fn normalizes_umu_fields_together() {
    assert_eq!(
        umu_fields(Some(" 12345 ".to_string()), Some(" GOG ".to_string())),
        (Some("umu-12345".to_string()), Some("gog".to_string()))
    );
    assert_eq!(umu_fields(None, Some("steam".to_string())), (None, None));
    assert_eq!(umu_fields(Some("".to_string()), Some("none".to_string())), (None, None));
}

#[test]
fn command_on_path_uses_current_path_entries() {
    let _guard = env_lock().lock().unwrap();
    let temp_dir = temp_path("path-check");
    let command = temp_dir.join("gamemoderun");
    fs::write(&command, b"#!/bin/sh\n").unwrap();
    set_executable(&command);

    let old_path = std::env::var_os("PATH");
    std::env::set_var("PATH", &temp_dir);

    assert!(command_on_path("gamemoderun"));
    assert!(!command_on_path("definitely-missing"));

    match old_path {
        Some(value) => std::env::set_var("PATH", value),
        None => std::env::remove_var("PATH"),
    }
    fs::remove_dir_all(temp_dir).unwrap();
}

#[test]
fn launch_lock_dir_prefers_xdg_runtime_dir() {
    let _guard = env_lock().lock().unwrap();
    let runtime_dir = temp_path("runtime-dir");
    let old_runtime = std::env::var_os("XDG_RUNTIME_DIR");
    std::env::set_var("XDG_RUNTIME_DIR", &runtime_dir);

    assert_eq!(launch_lock_dir(), runtime_dir.join("prefixr"));

    match old_runtime {
        Some(value) => std::env::set_var("XDG_RUNTIME_DIR", value),
        None => std::env::remove_var("XDG_RUNTIME_DIR"),
    }
    fs::remove_dir_all(runtime_dir).unwrap();
}

#[test]
fn sanitizes_shortcut_filenames() {
    assert_eq!(sanitize_filename("Baldur's Gate 3: Deluxe/Edition"), "Baldur_s Gate 3_ Deluxe_Edition");
    assert_eq!(sanitize_filename(""), "game");
    assert_eq!(sanitize_filename("  \t\n  "), "__");
    assert_eq!(sanitize_filename("Already_Good-Name 2"), "Already_Good-Name 2");
}

#[test]
fn desktop_directory_prefers_user_dirs_config() {
    let _guard = env_lock().lock().unwrap();
    let home = temp_path("home-desktop-dir");
    fs::create_dir_all(home.join(".config")).unwrap();
    fs::write(
        home.join(".config/user-dirs.dirs"),
        "XDG_DESKTOP_DIR=\"$HOME/Schreibtisch\"\n",
    )
    .unwrap();

    let old_home = std::env::var_os("HOME");
    std::env::set_var("HOME", &home);

    assert_eq!(desktop_directory().unwrap(), home.join("Schreibtisch"));

    match old_home {
        Some(value) => std::env::set_var("HOME", value),
        None => std::env::remove_var("HOME"),
    }
    fs::remove_dir_all(home).unwrap();
}

#[test]
fn desktop_directory_falls_back_to_home_desktop() {
    let _guard = env_lock().lock().unwrap();
    let home = temp_path("home-desktop-fallback");

    let old_home = std::env::var_os("HOME");
    std::env::set_var("HOME", &home);

    assert_eq!(desktop_directory().unwrap(), home.join("Desktop"));

    match old_home {
        Some(value) => std::env::set_var("HOME", value),
        None => std::env::remove_var("HOME"),
    }
    fs::remove_dir_all(home).unwrap();
}

#[test]
fn steer_profile_to_steamuser_creates_missing_user_symlink() {
    let _guard = env_lock().lock().unwrap();
    let prefix = temp_path("steamuser-link");
    let users_dir = prefix.join("drive_c/users");
    fs::create_dir_all(&users_dir).unwrap();

    let old_user = std::env::var_os("USER");
    std::env::set_var("USER", "tristan");

    steer_profile_to_steamuser(&prefix).unwrap();

    let steamuser_dir = users_dir.join("steamuser");
    let user_dir = users_dir.join("tristan");
    assert!(steamuser_dir.is_dir());
    assert_eq!(fs::read_link(&user_dir).unwrap(), PathBuf::from("steamuser"));

    match old_user {
        Some(value) => std::env::set_var("USER", value),
        None => std::env::remove_var("USER"),
    }
    fs::remove_dir_all(prefix).unwrap();
}

#[test]
fn steer_profile_to_steamuser_keeps_existing_user_dir() {
    let _guard = env_lock().lock().unwrap();
    let prefix = temp_path("steamuser-existing");
    let user_dir = prefix.join("drive_c/users/tristan");
    fs::create_dir_all(&user_dir).unwrap();

    let old_user = std::env::var_os("USER");
    std::env::set_var("USER", "tristan");

    steer_profile_to_steamuser(&prefix).unwrap();

    assert!(fs::symlink_metadata(&user_dir).unwrap().file_type().is_dir());

    match old_user {
        Some(value) => std::env::set_var("USER", value),
        None => std::env::remove_var("USER"),
    }
    fs::remove_dir_all(prefix).unwrap();
}

#[test]
fn applications_directory_prefers_xdg_data_home() {
    let _guard = env_lock().lock().unwrap();
    let home = temp_path("home-apps-dir");
    let data_home = temp_path("xdg-data-home");

    let old_home = std::env::var_os("HOME");
    let old_xdg = std::env::var_os("XDG_DATA_HOME");
    std::env::set_var("HOME", &home);
    std::env::set_var("XDG_DATA_HOME", &data_home);

    assert_eq!(applications_directory().unwrap(), data_home.join("applications"));

    match old_home {
        Some(value) => std::env::set_var("HOME", value),
        None => std::env::remove_var("HOME"),
    }
    match old_xdg {
        Some(value) => std::env::set_var("XDG_DATA_HOME", value),
        None => std::env::remove_var("XDG_DATA_HOME"),
    }
    fs::remove_dir_all(home).unwrap();
    fs::remove_dir_all(data_home).unwrap();
}

#[test]
fn applications_directory_falls_back_to_local_share() {
    let _guard = env_lock().lock().unwrap();
    let home = temp_path("home-apps-fallback");

    let old_home = std::env::var_os("HOME");
    let old_xdg = std::env::var_os("XDG_DATA_HOME");
    std::env::set_var("HOME", &home);
    std::env::remove_var("XDG_DATA_HOME");

    assert_eq!(
        applications_directory().unwrap(),
        home.join(".local/share/applications")
    );

    match old_home {
        Some(value) => std::env::set_var("HOME", value),
        None => std::env::remove_var("HOME"),
    }
    match old_xdg {
        Some(value) => std::env::set_var("XDG_DATA_HOME", value),
        None => std::env::remove_var("XDG_DATA_HOME"),
    }
    fs::remove_dir_all(home).unwrap();
}

#[test]
fn own_executable_path_prefers_appimage_env() {
    let _guard = env_lock().lock().unwrap();
    let old_appimage = std::env::var_os("APPIMAGE");
    std::env::set_var("APPIMAGE", "/opt/Prefixr.AppImage");

    assert_eq!(own_executable_path().unwrap(), PathBuf::from("/opt/Prefixr.AppImage"));

    match old_appimage {
        Some(value) => std::env::set_var("APPIMAGE", value),
        None => std::env::remove_var("APPIMAGE"),
    }
}

#[test]
fn log_stdio_appends_stdout_and_stderr_to_the_same_file() {
    let _guard = env_lock().lock().unwrap();
    let log_dir = temp_path("log-stdio");
    let log_path = log_dir.join("game.log");
    fs::write(&log_path, b"existing\n").unwrap();

    let (stdout, stderr) = log_stdio(&log_path).unwrap();
    let status = std::process::Command::new("/bin/sh")
        .args(["-c", "printf 'out\\n'; printf 'err\\n' >&2"])
        .stdout(stdout)
        .stderr(stderr)
        .status()
        .unwrap();

    assert!(status.success());
    assert_eq!(fs::read_to_string(&log_path).unwrap(), "existing\nout\nerr\n");

    fs::remove_dir_all(log_dir).unwrap();
}

#[test]
fn relink_if_needed_replaces_wrong_links_and_ignores_missing_sources() {
    let dir = temp_path("relink");
    let src_a = dir.join("src-a.dll");
    let src_b = dir.join("src-b.dll");
    let dst = dir.join("dst.dll");
    fs::write(&src_a, b"a").unwrap();
    fs::write(&src_b, b"b").unwrap();
    std::os::unix::fs::symlink(&src_a, &dst).unwrap();

    relink_if_needed(&src_b, &dst).unwrap();
    assert_eq!(fs::canonicalize(&dst).unwrap(), fs::canonicalize(&src_b).unwrap());

    let missing = dir.join("missing.dll");
    relink_if_needed(&missing, &dst).unwrap();
    assert_eq!(fs::canonicalize(&dst).unwrap(), fs::canonicalize(&src_b).unwrap());

    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn syncs_directx_layers_into_a_64_bit_prefix() {
    let cache_dir = temp_path("dx-cache");
    let prefix_dir = temp_path("dx-prefix");
    let system32 = prefix_dir.join("drive_c/windows/system32");
    let syswow64 = prefix_dir.join("drive_c/windows/syswow64");
    fs::create_dir_all(&system32).unwrap();
    fs::create_dir_all(&syswow64).unwrap();
    write_prefix_arch(&prefix_dir, "win64");

    let dxvk64 = cache_dir.join("dxvk/x64");
    let dxvk32 = cache_dir.join("dxvk/x32");
    let vkd3d64 = cache_dir.join("vkd3d-proton/x64");
    let vkd3d32 = cache_dir.join("vkd3d-proton/x86");
    for dir in [&dxvk64, &dxvk32, &vkd3d64, &vkd3d32] {
        fs::create_dir_all(dir).unwrap();
    }

    fs::write(dxvk64.join("d3d11.dll"), b"64").unwrap();
    fs::write(dxvk32.join("d3d11.dll"), b"32").unwrap();
    fs::write(vkd3d64.join("d3d12.dll"), b"64").unwrap();
    fs::write(vkd3d32.join("d3d12.dll"), b"32").unwrap();

    let overrides = sync_directx_overrides_from_cache(&cache_dir, &prefix_dir).unwrap();

    assert_eq!(
        fs::canonicalize(system32.join("d3d11.dll")).unwrap(),
        fs::canonicalize(dxvk64.join("d3d11.dll")).unwrap()
    );
    assert_eq!(
        fs::canonicalize(syswow64.join("d3d11.dll")).unwrap(),
        fs::canonicalize(dxvk32.join("d3d11.dll")).unwrap()
    );
    assert_eq!(
        fs::canonicalize(system32.join("d3d12.dll")).unwrap(),
        fs::canonicalize(vkd3d64.join("d3d12.dll")).unwrap()
    );
    assert_eq!(
        fs::canonicalize(syswow64.join("d3d12.dll")).unwrap(),
        fs::canonicalize(vkd3d32.join("d3d12.dll")).unwrap()
    );
    assert_eq!(
        overrides,
        "d3d8,d3d9,d3d10core,d3d11,dxgi,d3d12,d3d12core=n,b"
    );

    fs::remove_dir_all(cache_dir).unwrap();
    fs::remove_dir_all(prefix_dir).unwrap();
}

#[test]
fn syncs_directx_layers_into_a_32_bit_prefix() {
    let cache_dir = temp_path("dx-cache-win32");
    let prefix_dir = temp_path("dx-prefix-win32");
    let system32 = prefix_dir.join("drive_c/windows/system32");
    fs::create_dir_all(&system32).unwrap();
    write_prefix_arch(&prefix_dir, "win32");

    let dxvk32 = cache_dir.join("dxvk/x32");
    fs::create_dir_all(&dxvk32).unwrap();
    fs::write(dxvk32.join("dxgi.dll"), b"32").unwrap();

    sync_directx_overrides_from_cache(&cache_dir, &prefix_dir).unwrap();

    assert_eq!(
        fs::canonicalize(system32.join("dxgi.dll")).unwrap(),
        fs::canonicalize(dxvk32.join("dxgi.dll")).unwrap()
    );
    assert!(!prefix_dir.join("drive_c/windows/syswow64/dxgi.dll").exists());

    fs::remove_dir_all(cache_dir).unwrap();
    fs::remove_dir_all(prefix_dir).unwrap();
}

#[test]
fn game_env_merges_list_variables() {
    let mut env = vec![
        ("WINEDLLOVERRIDES".to_string(), "winemenubuilder.exe=;d3d11=n".to_string()),
        ("LD_PRELOAD".to_string(), "libgamemodeauto.so.0".to_string()),
        ("DXVK_HUD".to_string(), "0".to_string()),
    ];
    let game_vars = HashMap::from([
        ("WINEDLLOVERRIDES".to_string(), "dinput8=n,b".to_string()),
        ("LD_PRELOAD".to_string(), "libfoo.so".to_string()),
        ("DXVK_HUD".to_string(), "fps".to_string()),
    ]);
    add_game_env(&mut env, &game_vars);

    let get = |key: &str| {
        env.iter()
            .rev()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.as_str())
    };
    assert_eq!(get("WINEDLLOVERRIDES"), Some("winemenubuilder.exe=;d3d11=n;dinput8=n,b"));
    assert_eq!(get("LD_PRELOAD"), Some("libgamemodeauto.so.0:libfoo.so"));
    assert_eq!(get("DXVK_HUD"), Some("fps"));
}

#[test]
fn inherited_preload_is_kept() {
    let overlay = "/steam/ubuntu12_64/gameoverlayrenderer.so".to_string();
    let mut env = vec![("LD_PRELOAD".to_string(), "libgamemodeauto.so.0".to_string())];
    keep_inherited_preload(&mut env, Some(overlay.clone()));
    assert_eq!(env[0].1, format!("libgamemodeauto.so.0:{overlay}"));

    let mut env = vec![("DXVK_HUD".to_string(), "fps".to_string())];
    keep_inherited_preload(&mut env, Some(overlay.clone()));
    assert_eq!(env, vec![("DXVK_HUD".to_string(), "fps".to_string())]);

    let mut env = vec![
        ("LD_PRELOAD".to_string(), "libgamemodeauto.so.0".to_string()),
        ("LD_PRELOAD".to_string(), String::new()),
    ];
    keep_inherited_preload(&mut env, Some(overlay));
    assert_eq!(env[1].1, "");

    let mut env = vec![("LD_PRELOAD".to_string(), "libgamemodeauto.so.0".to_string())];
    keep_inherited_preload(&mut env, Some("   ".to_string()));
    assert_eq!(env[0].1, "libgamemodeauto.so.0");
}

#[test]
fn env_pairs_borrow_existing_entries_in_order() {
    let env = vec![
        ("A".to_string(), "1".to_string()),
        ("B".to_string(), "2".to_string()),
    ];

    assert_eq!(env_pairs(&env), vec![("A", "1"), ("B", "2")]);
}

#[test]
fn finds_a_games_shortcuts_by_their_launch_argument() {
    let dir = std::env::temp_dir().join(format!("prefixr-test-{}", Uuid::new_v4()));
    fs::create_dir_all(&dir).unwrap();
    let (id, other) = (Uuid::new_v4(), Uuid::new_v4());
    let entry = |id: Uuid| format!("[Desktop Entry]\nName=X\nExec=\"/a/prefixr\" --launch {id}\n");
    fs::write(dir.join("X.desktop"), entry(id)).unwrap();
    fs::write(dir.join("X-12345678.desktop"), entry(other)).unwrap();
    fs::write(dir.join("notes.txt"), entry(id)).unwrap();

    assert_eq!(game_shortcuts(&dir, id), vec![dir.join("X.desktop")]);
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn desktop_entries_are_escaped() {
    assert_eq!(desktop_string("Game: Part\\2\n"), "Game: Part\\\\2 ");
    assert_eq!(
        desktop_exec_arg(Path::new("/Apps/100% \"Pre$fixr\"/a\\b.AppImage")),
        r#""/Apps/100%% \\"Pre\\$fixr\\"/a\\\\b.AppImage""#
    );
    assert_eq!(desktop_exec_arg(Path::new("/opt/prefixr")), "\"/opt/prefixr\"");
}

#[test]
fn detects_32_bit_prefixes() {
    let dir = std::env::temp_dir().join(format!("prefixr-test-{}", Uuid::new_v4()));
    fs::create_dir_all(&dir).unwrap();
    assert!(!is_win32_prefix(&dir));
    let header = |arch: &str| {
        format!("WINE REGISTRY Version 2\n;; All keys relative to \\\\Machine\n\n#arch={arch}\n\n")
    };
    fs::write(dir.join("system.reg"), header("win64")).unwrap();
    assert!(!is_win32_prefix(&dir));
    fs::write(dir.join("system.reg"), header("win32")).unwrap();
    assert!(is_win32_prefix(&dir));
    fs::remove_dir_all(&dir).unwrap();
}

fn claim_eventually(launching: &LaunchingGames, id: Uuid) -> Option<LaunchGuard<'_>> {
    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        if let Some(guard) = launching.claim(id) {
            return Some(guard);
        }
        if Instant::now() > deadline {
            return None;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}

#[test]
fn launching_games_refuses_a_launch_in_another_process() {
    let (here, elsewhere) = (LaunchingGames::default(), LaunchingGames::default());
    let id = Uuid::new_v4();
    let elsewhere_guard = elsewhere.claim(id).unwrap();
    if let Some(here_guard) = here.claim(id) {
        assert!(here.ids().contains(&id));
        drop(here_guard);
    } else {
        assert!(here.ids().is_empty());
    }
    drop(elsewhere_guard);
    assert!(claim_eventually(&here, id).is_some());
}

#[test]
fn launching_games_refuses_a_second_launch() {
    let launching = LaunchingGames::default();
    let id = Uuid::new_v4();
    let guard = launching.claim(id).unwrap();
    assert!(launching.claim(id).is_none());
    drop(guard);
    assert!(claim_eventually(&launching, id).is_some());
}

#[test]
fn launching_games_ids_track_claim_lifecycle() {
    let launching = LaunchingGames::default();
    let first = Uuid::new_v4();
    let second = Uuid::new_v4();

    let first_guard = launching.claim(first).unwrap();
    let second_guard = launching.claim(second).unwrap();

    let ids = launching.ids();
    assert!(ids.contains(&first));
    assert!(ids.contains(&second));
    assert_eq!(ids.len(), 2);

    drop(first_guard);
    let ids = launching.ids();
    assert!(!ids.contains(&first));
    assert!(ids.contains(&second));
    assert_eq!(ids.len(), 1);

    drop(second_guard);
    assert!(launching.ids().is_empty());
}

#[test]
fn process_running_rejects_missing_pids() {
    assert!(!process_running(u32::MAX));
}

#[test]
fn pkill_pattern_matches_paths_literally() {
    assert_eq!(
        pkill_pattern("/drive_c/Program Files (x86)/Game [v1.2]/game+.exe"),
        r"[/]drive_c/Program Files \(x86\)/Game \[v1\.2\]/game\+\.exe"
    );
    assert_eq!(pkill_pattern("umu-run /g.exe"), r"[u]mu-run /g\.exe");
    assert_eq!(pkill_pattern("^x"), r"\^x");
}

#[test]
fn pkill_pattern_matches_the_process_but_not_itself() {
    if !command_on_path("sleep") || !command_on_path("pgrep") {
        return;
    }

    let _guard = env_lock().lock().unwrap();
    let fraction = format!("0.{}", Uuid::new_v4().as_u128() % 1_000_000_000);
    let target = format!("sleep 30 {fraction}");
    let mut child = std::process::Command::new("sleep")
        .args(["30", &fraction])
        .spawn()
        .unwrap();
    let pattern = pkill_pattern(&target);

    let found = std::process::Command::new("pgrep")
        .args(["-f", &pattern])
        .output()
        .unwrap();
    let pids = String::from_utf8_lossy(&found.stdout);
    assert_eq!(pids.trim(), child.id().to_string());
    assert!(!regex_self_match(&pattern));

    child.kill().unwrap();
    child.wait().unwrap();
}

fn regex_self_match(pattern: &str) -> bool {
    if !command_on_path("grep") {
        return false;
    }

    use std::io::Write;
    let mut grep = std::process::Command::new("grep")
        .args(["-qE", pattern])
        .stdin(Stdio::piped())
        .spawn()
        .unwrap();
    grep.stdin.take().unwrap().write_all(pattern.as_bytes()).unwrap();
    grep.wait().unwrap().success()
}

#[test]
fn process_tree_helpers_see_real_processes() {
    if !command_on_path("sh") {
        return;
    }

    let _guard = env_lock().lock().unwrap();
    let mut child = std::process::Command::new("sh")
        .args(["-c", "sleep 30 & wait"])
        .spawn()
        .unwrap();
    let pid = child.id();
    std::thread::sleep(Duration::from_millis(200));

    assert!(process_running(pid));
    let descendants = process_descendants(pid);
    assert_eq!(descendants.len(), 1);

    tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .build()
        .unwrap()
        .block_on(kill_process_tree(pid));
    child.wait().unwrap();

    assert!(!process_running(pid));
    let deadline = Instant::now() + Duration::from_secs(2);
    while process_running(descendants[0]) && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(20));
    }
    assert!(!process_running(descendants[0]));
}
