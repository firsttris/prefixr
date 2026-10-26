// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

// CEF starts this same executable again for its renderer, GPU and utility
// processes; the macro runs those and returns before Prefixr's own command
// line is even looked at (tauri-runtime-cef, examples/cef/src-tauri/src/main.rs).
#[cfg_attr(feature = "cef", tauri_runtime_cef::cef_entry_point)]
fn main() {
    prefixr_lib::run()
}
