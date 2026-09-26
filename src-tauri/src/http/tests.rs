use super::client;

#[test]
fn client_builds_https_requests() {
    let request = client().get("https://example.invalid/test").build().unwrap();

    assert_eq!(request.method(), reqwest::Method::GET);
    assert_eq!(request.url().as_str(), "https://example.invalid/test");
}

#[test]
fn client_is_reusable() {
    let first = client().get("https://example.invalid/a").build().unwrap();
    let second = client().get("https://example.invalid/b").build().unwrap();

    assert_eq!(first.url().as_str(), "https://example.invalid/a");
    assert_eq!(second.url().as_str(), "https://example.invalid/b");
}
