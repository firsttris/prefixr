use std::sync::OnceLock;
use std::time::Duration;

/// The one HTTP client every request goes through. Its timeouts matter most
/// for the downloads a game launch waits on (umu, DXVK/VKD3D, wine-mono): a
/// stalled connection would otherwise hang the launch forever, with the game
/// held in `LaunchingGames` until Prefixr restarts. `read_timeout` applies
/// per read rather than to the whole request, so a slow but steady
/// multi-hundred-MB runner download still finishes.
pub fn client() -> reqwest::Client {
    static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    CLIENT
        .get_or_init(|| {
            reqwest::Client::builder()
                .user_agent("prefixr")
                .connect_timeout(Duration::from_secs(15))
                .read_timeout(Duration::from_secs(60))
                .build()
                .unwrap_or_default()
        })
        .clone()
}

#[cfg(test)]
mod tests {
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
}
