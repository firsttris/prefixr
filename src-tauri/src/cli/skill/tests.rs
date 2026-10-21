use super::*;
use crate::test_util::TestDir;

/// The words of a shell line, quotes taken off; enough for the skill's
/// examples (no escapes, no variables).
fn words(line: &str) -> Vec<String> {
    let mut words = Vec::new();
    let mut word = String::new();
    let mut quote = None;
    let mut started = false;
    for c in line.chars() {
        match (quote, c) {
            (None, '"' | '\'') => {
                quote = Some(c);
                started = true;
            }
            (Some(q), c) if c == q => quote = None,
            (None, ' ') => {
                if started {
                    words.push(std::mem::take(&mut word));
                    started = false;
                }
            }
            _ => {
                word.push(c);
                started = true;
            }
        }
    }
    if started {
        words.push(word);
    }
    words
}

#[test]
fn the_skill_says_what_it_is_for() {
    let front = SKILL
        .strip_prefix("---\n")
        .and_then(|rest| rest.split_once("\n---\n"))
        .map(|(front, _)| front)
        .expect("SKILL.md starts with front matter");
    assert!(front.lines().any(|l| l == "name: prefixr"));
    let description = front
        .lines()
        .find_map(|l| l.strip_prefix("description: "))
        .expect("a description");
    // Claude Code's limit for a skill's description.
    assert!(description.len() <= 1024);
}

/// The commands in the skill's examples: the lines of its code blocks, and
/// the inline `prefixr …` ones without a placeholder such as `<game>`.
fn examples() -> Vec<String> {
    let mut found = Vec::new();
    let mut in_block = false;
    for line in SKILL.lines() {
        if line.starts_with("```") {
            in_block = !in_block;
        } else if in_block {
            if let Some(command) = line.strip_prefix("prefixr ") {
                found.push(command.trim_end_matches('&').trim().to_string());
            }
        } else {
            for (i, span) in line.split('`').enumerate() {
                let inline = i % 2 == 1;
                if let Some(command) = span.strip_prefix("prefixr ").filter(|_| inline) {
                    if !command.contains('<') && !command.contains('…') {
                        found.push(command.trim_end_matches('&').trim().to_string());
                    }
                }
            }
        }
    }
    found
}

/// Every command in the skill's examples is one this Prefixr has, with
/// options it takes: an agent copying one never gets a usage error.
#[test]
fn every_example_in_the_skill_parses() {
    use clap::Parser;
    let examples = examples();
    assert!(
        examples.len() >= 20,
        "only {} examples found",
        examples.len()
    );
    for command in examples {
        let mut args = vec!["prefixr".to_string()];
        args.extend(words(&command));
        match super::super::Cli::try_parse_from(&args) {
            Ok(_) => {}
            // `--help` "fails" with the help text: a working example too.
            Err(e) if e.kind() == clap::error::ErrorKind::DisplayHelp => {}
            Err(e) => panic!("`prefixr {command}` in SKILL.md doesn't parse: {e}"),
        }
    }
}

#[test]
fn installing_writes_the_skill_and_replaces_an_older_one() {
    let dir = TestDir::new("skill-install");
    let target = dir.join("skills/prefixr");
    fs::create_dir_all(&target).unwrap();
    fs::write(target.join("SKILL.md"), "old").unwrap();
    let path = install(&target).unwrap();
    assert_eq!(path, target.join("SKILL.md"));
    assert_eq!(fs::read_to_string(&path).unwrap(), SKILL);
}

#[test]
fn skill_commands_parse_and_make_it_a_command_line() {
    use clap::Parser;
    assert!(super::super::is_cli(&["prefixr".into(), "skill".into()]));
    for line in ["prefixr skill show", "prefixr skill install --dir /tmp/x"] {
        super::super::Cli::try_parse_from(line.split(' ')).unwrap();
    }
}

#[test]
fn a_flatpak_skill_says_how_to_run_it() {
    let text = skill_text(Some("io.github.firsttris.Prefixr"));
    let title_end = SKILL.find("\n# Prefixr\n").unwrap() + "\n# Prefixr\n".len();
    // Right below the title, with the rest of the skill unchanged.
    assert!(text.starts_with(&SKILL[..title_end]));
    assert!(text[title_end..].starts_with("\n> **This Prefixr is a Flatpak.**"));
    assert!(text.ends_with(&SKILL[title_end..]));
    assert!(text.contains("`flatpak run io.github.firsttris.Prefixr game list --json`"));
}

#[test]
fn the_native_skill_is_unchanged() {
    assert_eq!(skill_text(None), SKILL);
}
