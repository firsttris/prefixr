use super::*;

fn url(s: &str) -> Url {
    s.parse().unwrap()
}

#[test]
fn gog_ends_on_its_success_page_with_a_code() {
    let done = url("https://embed.gog.com/on_login_success?origin=client&code=abc_DEF-123");
    let result = login_result(StoreKind::Gog, &done).unwrap();
    // What the pasted address goes through too.
    assert_eq!(
        crate::commands::stores::gog::auth_code(&result).as_deref(),
        Some("abc_DEF-123")
    );
    for going_on in [
        "https://auth.gog.com/auth?client_id=1&redirect_uri=https%3A%2F%2Fembed.gog.com%2Fon_login_success&response_type=code",
        "https://login.gog.com/login",
        "https://embed.gog.com/on_login_success?origin=client",
        "https://embed.gog.com/on_login_success?code=",
        "http://embed.gog.com/on_login_success?code=abc",
        "https://evil.example/on_login_success?code=abc",
    ] {
        assert_eq!(login_result(StoreKind::Gog, &url(going_on)), None, "{going_on}");
    }
}

#[test]
fn amazon_ends_on_an_amazon_address_with_the_code() {
    let done = url(
        "https://www.amazon.com/?openid.assoc_handle=amzn_sonic_games_launcher&openid.oa2.authorization_code=ANcode123456",
    );
    let result = login_result(StoreKind::Amazon, &done).unwrap();
    assert_eq!(
        crate::commands::stores::amazon::auth_code(&result).as_deref(),
        Some("ANcode123456")
    );
    for going_on in [
        "https://www.amazon.com/ap/signin?openid.return_to=https%3A%2F%2Fwww.amazon.com",
        "https://www.amazon.com/?openid.oa2.authorization_code=",
        "https://amazon.com.evil.example/?openid.oa2.authorization_code=x",
        "https://notamazon.com/?openid.oa2.authorization_code=x",
    ] {
        assert_eq!(
            login_result(StoreKind::Amazon, &url(going_on)),
            None,
            "{going_on}"
        );
    }
}

#[test]
fn epic_hands_its_code_over_through_the_script() {
    assert!(EPIC_SCRIPT.contains(&format!("https://{EPIC_CODE_HOST}/epic?code=")));
    let done = url("https://prefixr-login.invalid/epic?code=0123456789abcdef0123456789abcdef");
    let code = login_result(StoreKind::Epic, &done).unwrap();
    assert_eq!(
        crate::commands::stores::epic::auth_code(&code).as_deref(),
        Some("0123456789abcdef0123456789abcdef")
    );
    for going_on in [
        "https://legendary.gl/epiclogin",
        "https://www.epicgames.com/id/login",
        "https://www.epicgames.com/id/api/redirect?clientId=x&responseType=code",
        "https://prefixr-login.invalid/epic?code=",
        "https://prefixr-login.invalid/other?code=abc",
    ] {
        assert_eq!(
            login_result(StoreKind::Epic, &url(going_on)),
            None,
            "{going_on}"
        );
    }
    // A code only counts for the store whose window it ends.
    assert_eq!(login_result(StoreKind::Gog, &done), None);
}
