use super::{find_flag_arg, find_install_arg, find_launch_arg, find_run_arg};

#[test]
fn finds_specific_supported_flags() {
    let args = vec![
        "prefixr".to_string(),
        "--launch".to_string(),
        "game-123".to_string(),
        "--run".to_string(),
        "game-456".to_string(),
        "--install".to_string(),
        "/tmp/setup.exe".to_string(),
    ];

    assert_eq!(find_launch_arg(&args), Some("game-123".to_string()));
    assert_eq!(find_run_arg(&args), Some("game-456".to_string()));
    assert_eq!(find_install_arg(&args), Some("/tmp/setup.exe".to_string()));
}

#[test]
fn ignores_missing_flags_and_trailing_flag_without_value() {
    let args = vec!["prefixr".to_string(), "--launch".to_string()];

    assert_eq!(find_flag_arg(&args, "--run"), None);
    assert_eq!(find_launch_arg(&args), None);
}

#[test]
fn returns_the_first_matching_flag_value() {
    let args = vec![
        "prefixr".to_string(),
        "--run".to_string(),
        "first".to_string(),
        "--run".to_string(),
        "second".to_string(),
    ];

    assert_eq!(find_run_arg(&args), Some("first".to_string()));
}
