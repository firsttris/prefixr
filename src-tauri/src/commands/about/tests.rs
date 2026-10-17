use super::*;

#[test]
fn links_the_release_of_a_version() {
    assert_eq!(
        release_url("0.1.4"),
        "https://github.com/firsttris/prefixr/releases/tag/v0.1.4"
    );
}
