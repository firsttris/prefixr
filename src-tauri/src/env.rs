use std::ffi::OsString;

/// Where a function reads environment variables from: the process's own
/// environment in the app (`process`), a fixed set in a test — which then
/// needn't change the environment every test thread shares.
pub type Env<'a> = &'a dyn Fn(&str) -> Option<OsString>;

pub fn process(key: &str) -> Option<OsString> {
    std::env::var_os(key)
}

/// An environment of just these variables, for tests.
#[cfg(test)]
pub fn fake(vars: &[(&str, &str)]) -> impl Fn(&str) -> Option<OsString> {
    let vars: Vec<(String, OsString)> = vars
        .iter()
        .map(|(key, value)| (key.to_string(), OsString::from(value)))
        .collect();
    move |key| vars.iter().find(|(k, _)| k == key).map(|(_, v)| v.clone())
}
