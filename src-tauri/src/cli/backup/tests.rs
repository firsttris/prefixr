use super::*;

#[test]
fn backups_are_named_by_number_or_file() {
    let automatic = [
        PathBuf::from("/b/newest.tar.gz"),
        PathBuf::from("/b/older.tar.gz"),
    ];
    assert_eq!(
        resolve("1", &automatic).unwrap(),
        PathBuf::from("/b/newest.tar.gz")
    );
    assert_eq!(
        resolve("2", &automatic).unwrap(),
        PathBuf::from("/b/older.tar.gz")
    );
    assert!(resolve("0", &automatic).is_err());
    assert!(resolve("3", &automatic).is_err());
    assert_eq!(
        resolve("/x/mine.tar.gz", &automatic).unwrap(),
        PathBuf::from("/x/mine.tar.gz")
    );
    assert!(resolve("mine.tar.gz", &automatic).unwrap().is_absolute());
}

#[test]
fn backup_commands_parse_and_take_the_lock_only_inside_a_restore() {
    use clap::Parser;
    let parse = |line: &str| match super::super::Cli::try_parse_from(line.split(' '))
        .unwrap()
        .command
    {
        super::super::Command::Backup(command) => command,
        other => panic!("not a backup command: {other:?}"),
    };
    assert!(matches!(
        parse("prefixr backup create --with-secrets"),
        BackupCommand::Create {
            path: None,
            with_secrets: true
        }
    ));
    assert!(matches!(
        parse("prefixr backup restore 1"),
        BackupCommand::Restore { .. }
    ));
    assert!(!changes_config(&parse("prefixr backup restore 1")));
    assert!(!changes_config(&parse("prefixr backup list")));
    assert!(super::super::is_cli(&[
        "prefixr".to_string(),
        "backup".to_string()
    ]));
}
