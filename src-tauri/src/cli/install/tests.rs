use super::*;
use crate::cli::{Cli, Command, GameCommand};
use clap::Parser;
use std::fs;

fn shortcut(name: &str, exe: &str) -> DetectedShortcut {
    DetectedShortcut {
        name: name.to_string(),
        exe_path: exe.to_string(),
    }
}

#[test]
fn the_loop_device_is_read_from_udisksctl() {
    assert_eq!(
        parse_loop_device("Mapped file /home/me/Games/Disc 1.iso as /dev/loop3."),
        Some("/dev/loop3".to_string())
    );
    assert_eq!(parse_loop_device("Error setting up loop device"), None);
}

#[test]
fn the_mount_point_is_read_from_the_mount_table() {
    let mounts = "/dev/nvme0n1p2 / btrfs rw 0 0\n\
                  /dev/loop3 /run/media/me/HALF\\040LIFE\\0402 iso9660 ro,nosuid 0 0\n\
                  /dev/loop31 /mnt/other iso9660 ro 0 0\n";
    assert_eq!(
        mount_point_of(mounts, "/dev/loop3"),
        Some(PathBuf::from("/run/media/me/HALF LIFE 2"))
    );
    assert_eq!(mount_point_of(mounts, "/dev/loop4"), None);
}

#[test]
fn autorun_inf_names_the_program_without_its_arguments() {
    assert_eq!(
        autorun_program("[autorun]\r\nopen=Setup.exe /autorun\r\nicon=game.ico\r\n"),
        Some("Setup.exe".to_string())
    );
    assert_eq!(
        autorun_program("[AutoRun]\n; menu\nOPEN = \"Install\\Game Setup.exe\" -x\n"),
        Some("Install/Game Setup.exe".to_string())
    );
    assert_eq!(
        autorun_program("[autorun]\nshellexecute=.\\autorun.exe\n"),
        Some("autorun.exe".to_string())
    );
    assert_eq!(
        autorun_program("[other]\nopen=wrong.exe\n[autorun]\nlabel=Disc\n"),
        None
    );
}

#[test]
fn a_discs_installer_is_found_in_any_case() {
    let disc = tempfile::tempdir().unwrap();
    fs::write(disc.path().join("SETUP.EXE"), "").unwrap();
    assert_eq!(
        disc_installer(disc.path(), None).unwrap(),
        disc.path().join("SETUP.EXE")
    );

    fs::create_dir(disc.path().join("INSTALL")).unwrap();
    fs::write(disc.path().join("INSTALL/GAMESETUP.EXE"), "").unwrap();
    fs::write(
        disc.path().join("AUTORUN.INF"),
        "[autorun]\nopen=install\\gamesetup.exe\n",
    )
    .unwrap();
    assert_eq!(
        disc_installer(disc.path(), None).unwrap(),
        disc.path().join("INSTALL/GAMESETUP.EXE")
    );

    assert_eq!(
        disc_installer(disc.path(), Some(Path::new("setup.exe"))).unwrap(),
        disc.path().join("SETUP.EXE")
    );
    assert!(disc_installer(disc.path(), Some(Path::new("missing.exe"))).is_err());
}

#[test]
fn a_disc_without_an_installer_lists_its_programs() {
    let disc = tempfile::tempdir().unwrap();
    fs::write(disc.path().join("Launcher.exe"), "").unwrap();
    let error = disc_installer(disc.path(), None).unwrap_err();
    assert!(error.contains("--setup"), "{error}");
    assert!(error.contains("Launcher.exe"), "{error}");
}

#[test]
fn the_one_shortcut_names_the_game() {
    let picked = pick_shortcut(
        &[shortcut("Half-Life 2", "/p/drive_c/Games/HL2/hl2.exe")],
        Path::new("/p"),
        "GE-Proton10-7",
    )
    .unwrap();
    assert_eq!(
        picked,
        (
            PathBuf::from("/p/drive_c/Games/HL2/hl2.exe"),
            Some("Half-Life 2".to_string())
        )
    );
}

#[test]
fn no_shortcut_or_several_say_how_to_add_the_game() {
    let none = pick_shortcut(&[], Path::new("/p"), "GE-Proton10-7").unwrap_err();
    assert!(none.contains("/p/drive_c"), "{none}");
    assert!(
        none.contains("prefixr game add --exe <exe> --prefix /p --runner GE-Proton10-7"),
        "{none}"
    );

    let several = pick_shortcut(
        &[
            shortcut("Game", "/p/drive_c/G/game.exe"),
            shortcut("Game Config", "/p/drive_c/G/config.exe"),
        ],
        Path::new("/p"),
        "r",
    )
    .unwrap_err();
    assert!(several.contains("Game: /p/drive_c/G/game.exe"), "{several}");
    assert!(
        several.contains("Game Config: /p/drive_c/G/config.exe"),
        "{several}"
    );
}

#[test]
fn only_iso_files_are_disc_images() {
    assert!(is_disc_image(Path::new("/g/Disc.ISO")));
    assert!(is_disc_image(Path::new("/g/disc.iso")));
    assert!(!is_disc_image(Path::new("/g/setup.exe")));
    assert!(!is_disc_image(Path::new("/g/iso")));
}

#[test]
fn what_follows_the_double_dash_goes_to_the_installer() {
    let cli = Cli::try_parse_from([
        "prefixr",
        "game",
        "install",
        "/g/setup.exe",
        "--prefix",
        "/p",
        "--name",
        "Quake",
        "--",
        "/VERYSILENT",
        "/DIR=C:\\Games\\Quake",
        "--prefix",
    ])
    .unwrap();
    let Command::Game(GameCommand::Install(install)) = cli.command else {
        panic!("not game install: {:?}", cli.command);
    };
    assert_eq!(install.prefix, PathBuf::from("/p"));
    assert_eq!(install.name.as_deref(), Some("Quake"));
    assert_eq!(install.runner, None);
    assert_eq!(
        install.installer_args,
        ["/VERYSILENT", "/DIR=C:\\Games\\Quake", "--prefix"]
    );
}
