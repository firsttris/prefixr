use super::*;

#[test]
fn round_trips_a_shortcuts_file() {
    let map: Map = vec![(
        "shortcuts".into(),
        Value::Map(vec![(
            "0".into(),
            Value::Map(vec![
                ("appid".into(), Value::Int(0x8000_1234)),
                ("AppName".into(), Value::String("Spiel ä".into())),
                ("LastPlayTime".into(), Value::Int(0)),
                ("tags".into(), Value::Map(vec![])),
                ("x".into(), Value::Other { tag: 0x07, bytes: vec![1; 8] }),
            ]),
        )]),
    )];
    let bytes = write(&map);
    assert_eq!(&bytes[bytes.len() - 2..], &[END, END]);
    assert_eq!(parse(&bytes).unwrap(), map);
}

#[test]
fn parses_an_empty_file_steam_wrote() {
    let map = parse(b"\x00shortcuts\x00\x08\x08").unwrap();
    assert_eq!(map, vec![("shortcuts".to_string(), Value::Map(vec![]))]);
}

#[test]
fn rejects_truncated_files() {
    assert!(parse(b"\x00shortcuts\x00\x01AppName\x00Half").is_err());
    assert!(parse(b"\x00shortcuts\x00\x02appid\x00\x01").is_err());
}

#[test]
fn keys_are_case_insensitive() {
    let mut map: Map = vec![("exe".into(), Value::String("a".into()))];
    set(&mut map, "Exe", Value::String("b".into()));
    assert_eq!(map, vec![("exe".to_string(), Value::String("b".into()))]);
    assert_eq!(get(&map, "EXE"), Some(&Value::String("b".into())));
}
