//! The browser engine, chosen by the build's feature (Cargo.toml): Chromium
//! (CEF) for the packages, WebKitGTK for the Flatpak.

/// Chromium as CEF builds it talks to Google on its own: secure DNS probes,
/// sign-in state, search, the translation model, network time and component
/// updates (seen in a net log, `--log-net-log`, of Schreibzeug, which takes
/// the same settings). Prefixr shows its own pages and, in the store login
/// window, the store's, so each is switched off, and Google's sign-in,
/// search, update and translation-model addresses point to a closed local
/// port. Switches need their dashes: without them and a value the runtime
/// appends them as plain arguments, which Chromium ignores (tauri-runtime-cef,
/// append_command_line_args).
#[cfg(feature = "cef")]
pub fn runtime() -> tauri_runtime_cef::Cef {
    const NOWHERE: &str = "http://127.0.0.1:9";
    const OFF: [&str; 6] = [
        "--disable-background-networking",
        "--disable-component-update",
        "--disable-domain-reliability",
        "--disable-sync",
        "--no-default-browser-check",
        "--no-pings",
    ];
    let elsewhere = [
        ("--gaia-url", NOWHERE.to_owned()),
        ("--google-base-url", NOWHERE.to_owned()),
        ("--component-updater", format!("url-source={NOWHERE}")),
        ("--translate-ranker-model-url", NOWHERE.to_owned()),
    ];
    let cef = tauri_runtime_cef::Cef::default()
        .profile_preference("safebrowsing.enabled", false)
        .profile_preference("signin.allowed", false)
        .profile_preference("translate.enabled", false)
        .global_preference("dns_over_https.mode", "off")
        .spell_checking(false)
        .disable_features(["DnsOverHttps", "NetworkTimeServiceQuerying"]);
    let cef = OFF.into_iter().fold(cef, |cef, switch| {
        cef.command_line_arg(switch, None::<String>)
    });
    elsewhere.into_iter().fold(cef, |cef, (switch, value)| {
        cef.command_line_arg(switch, Some(value))
    })
}

#[cfg(all(feature = "webkit", not(feature = "cef")))]
pub fn runtime() -> tauri_runtime_wry::Wry {
    tauri_runtime_wry::Wry::default()
}

#[cfg(not(any(feature = "cef", feature = "webkit")))]
compile_error!("build with the feature `cef` (the default) or `webkit` (the Flatpak)");
