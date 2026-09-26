use super::sanitized_token;

#[test]
fn trims_configured_tokens() {
    assert_eq!(sanitized_token(Some("  secret-token  ")), Some("secret-token".to_string()));
}

#[test]
fn empty_or_missing_tokens_become_none() {
    assert_eq!(sanitized_token(Some("   \t\n  ")), None);
    assert_eq!(sanitized_token(None), None);
}
