use super::{
    build_max_map_count_status, finalize_max_map_count_fix, manual_fix_command,
    parse_max_map_count, sysctl_fix_script, MaxMapCountStatus, RECOMMENDED_MAX_MAP_COUNT,
    SYSCTL_DROPIN_PATH,
};
use crate::error::AppError;

fn assert_status(status: MaxMapCountStatus, current: u64, sufficient: bool, can_fix: bool) {
    assert_eq!(status.current, current);
    assert_eq!(status.recommended, RECOMMENDED_MAX_MAP_COUNT);
    assert_eq!(status.sufficient, sufficient);
    assert_eq!(status.can_fix, can_fix);
}

#[test]
fn parses_max_map_count_or_falls_back_to_zero() {
    assert_eq!(parse_max_map_count(Some("2147483642\n")), RECOMMENDED_MAX_MAP_COUNT);
    assert_eq!(parse_max_map_count(Some("not-a-number")), 0);
    assert_eq!(parse_max_map_count(None), 0);
}

#[test]
fn builds_status_with_threshold_and_fix_availability() {
    assert_status(
        build_max_map_count_status(RECOMMENDED_MAX_MAP_COUNT - 1, false),
        RECOMMENDED_MAX_MAP_COUNT - 1,
        false,
        false,
    );
    assert_status(
        build_max_map_count_status(RECOMMENDED_MAX_MAP_COUNT, true),
        RECOMMENDED_MAX_MAP_COUNT,
        true,
        true,
    );
}

#[test]
fn exposes_manual_and_pkexec_fix_commands() {
    assert_eq!(
        manual_fix_command(),
        format!(
            "sudo sh -c 'echo \"vm.max_map_count = {RECOMMENDED_MAX_MAP_COUNT}\" > {SYSCTL_DROPIN_PATH} && sysctl --system'"
        )
    );
    assert_eq!(
        sysctl_fix_script(),
        format!(
            "echo 'vm.max_map_count = {RECOMMENDED_MAX_MAP_COUNT}' > {SYSCTL_DROPIN_PATH} && sysctl --system"
        )
    );
}

#[test]
fn reports_missing_pkexec_with_manual_fallback() {
    let error = finalize_max_map_count_fix(false, Ok((true, "0".to_string()))).unwrap_err();

    match error {
        AppError::PkexecNotInstalled { command } => {
            assert_eq!(command, manual_fix_command());
        }
        other => panic!("unexpected error: {other:?}"),
    }
}

#[test]
fn reports_failed_or_unspawnable_pkexec_runs() {
    let error = finalize_max_map_count_fix(true, Ok((false, "exit status: 126".to_string())))
        .unwrap_err();
    match error {
        AppError::SysctlChangeFailed { status } => {
            assert_eq!(status, "exit status: 126");
        }
        other => panic!("unexpected error: {other:?}"),
    }

    let error = finalize_max_map_count_fix(true, Err("Could not run pkexec: boom".to_string()))
        .unwrap_err();
    match error {
        AppError::Other { message } => {
            assert_eq!(message, "Could not run pkexec: boom");
        }
        other => panic!("unexpected error: {other:?}"),
    }
}

#[test]
fn accepts_successful_pkexec_runs() {
    assert!(finalize_max_map_count_fix(true, Ok((true, "exit status: 0".to_string()))).is_ok());
}
