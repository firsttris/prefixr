//! Signing in to a store in a window of Prefixr's own instead of the
//! browser, so nobody has to copy a code: the window watches where the
//! store's login goes and takes the code from there. GOG and Amazon end on
//! an address carrying it; Epic ends on a JSON page holding it, which
//! `EPIC_SCRIPT` hands over by going on to `EPIC_CODE_URL`.
//!
//! The pages in the window get none of the app's commands (the
//! capabilities only cover the main window), and it keeps no cookies, so
//! nothing of the login stays behind but what the store's tool saves. The
//! browser login stays for password managers and "sign in with Google",
//! which don't work in an embedded window.

use std::sync::{Arc, Mutex};

use tauri::{AppHandle, Manager, Url, WebviewUrl, WebviewWindowBuilder, WindowEvent};
use tokio::sync::oneshot;

use crate::error::AppError;
use crate::locale::{self, LocaleState};
use crate::lock::LockExt;
use crate::models::StoreKind;

/// Where `EPIC_SCRIPT` sends the code: a name that never resolves, so the
/// window only ever sees it as a navigation to stop.
const EPIC_CODE_HOST: &str = "prefixr-login.invalid";

/// Run on every page in Epic's login window: on the page Epic's login
/// ends on (`{"authorizationCode": "…", …}`), it goes on to the address
/// the window catches.
const EPIC_SCRIPT: &str = r#"
(() => {
  if (location.hostname !== "www.epicgames.com" || !location.pathname.startsWith("/id/api/redirect")) return;
  document.addEventListener("DOMContentLoaded", () => {
    try {
      const code = JSON.parse(document.body.innerText).authorizationCode;
      if (code) location.replace("https://prefixr-login.invalid/epic?code=" + encodeURIComponent(code));
    } catch (_) {}
  });
})();
"#;

/// The query parameter Amazon's login sends the code back in.
const AMAZON_CODE_PARAMETER: &str = "openid.oa2.authorization_code";

fn has_query(url: &Url, key: &str) -> bool {
    url.query_pairs().any(|(k, v)| k == key && !v.is_empty())
}

/// Whether the login ended at `url`, and if so what `connect_store` takes:
/// GOG's and Amazon's address with the code in it, or the code Epic's page
/// showed.
pub fn login_result(store: StoreKind, url: &Url) -> Option<String> {
    if url.scheme() != "https" {
        return None;
    }
    let host = url.host_str()?;
    match store {
        StoreKind::Epic => (host == EPIC_CODE_HOST && url.path() == "/epic")
            .then(|| {
                url.query_pairs()
                    .find(|(key, _)| key == "code")
                    .map(|(_, code)| code.into_owned())
            })
            .flatten()
            .filter(|code| !code.is_empty()),
        StoreKind::Gog => {
            (host == "embed.gog.com" && url.path() == "/on_login_success" && has_query(url, "code"))
                .then(|| url.to_string())
        }
        StoreKind::Amazon => ((host == "amazon.com" || host.ends_with(".amazon.com"))
            && has_query(url, AMAZON_CODE_PARAMETER))
        .then(|| url.to_string()),
    }
}

/// Hands the window's outcome over once: the first of a code, or the
/// window closing without one.
#[derive(Clone)]
struct Handoff(Arc<Mutex<Option<oneshot::Sender<Option<String>>>>>);

impl Handoff {
    fn give(&self, result: Option<String>) {
        if let Some(sender) = self.0.locked().take() {
            let _ = sender.send(result);
        }
    }
}

/// Opens `login_url` in a window of its own and waits until the login
/// ends there: the code, or `None` when the user closed the window. A login
/// window already open for the store is closed first.
pub async fn sign_in(
    app: &AppHandle,
    store: StoreKind,
    login_url: &str,
) -> Result<Option<String>, AppError> {
    let url: Url = login_url
        .parse()
        .map_err(|e| format!("Invalid login address {login_url}: {e}"))?;
    let label = format!("store-login-{}", store.slug());
    if let Some(open) = app.get_webview_window(&label) {
        let _ = open.destroy();
    }
    let (sender, receiver) = oneshot::channel();
    let handoff = Handoff(Arc::new(Mutex::new(Some(sender))));
    let title = locale::text(
        app.state::<LocaleState>().get(),
        "native_storeLoginTitle",
        &[("store", store.display_name())],
    );
    let (on_navigation, on_load) = (handoff.clone(), handoff.clone());
    let mut builder = WebviewWindowBuilder::new(app, &label, WebviewUrl::External(url))
        .title(title)
        .inner_size(520.0, 760.0)
        .incognito(true)
        // Stopped there: the page the code is on needn't load.
        .on_navigation(move |url| match login_result(store, url) {
            Some(code) => {
                on_navigation.give(Some(code));
                false
            }
            None => true,
        })
        // Also on loads, in case a redirect wasn't a navigation of its own.
        .on_page_load(move |_, payload| {
            if let Some(code) = login_result(store, payload.url()) {
                on_load.give(Some(code));
            }
        });
    if store == StoreKind::Epic {
        builder = builder.initialization_script(EPIC_SCRIPT);
    }
    let window = builder
        .build()
        .map_err(|e| format!("Could not open the login window: {e}"))?;
    window.on_window_event(move |event| {
        if matches!(event, WindowEvent::Destroyed) {
            handoff.give(None);
        }
    });
    let result = receiver.await.unwrap_or(None);
    let _ = window.destroy();
    Ok(result)
}

#[cfg(test)]
mod tests;
