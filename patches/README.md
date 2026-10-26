# Patches for the Tauri CLI (temporary)

`scripts/tauri.mjs` builds the Tauri CLI from Tauri's sources at the version in `package.json` (tag
`tauri-cli-v<version>`) with these patches applied, and `bun run tauri …` uses it instead of the
published CLI. The Flatpak keeps the published one (`npx tauri`): it builds WebKitGTK, not CEF, and
bundles nothing.

## tauri-cli-cef-angle-optional.patch

The bundler copies `libEGL.so` and `libGLESv2.so` from the CEF distribution into every Linux package
(`crates/tauri-bundler/src/bundle/linux/debian.rs`, `rpm.rs`, `appimage/sharun_cef.rs`). CEF 154
no longer ships them on Linux (`cef_binary_154.0.34 linux64_minimal`: `libcef.so`,
`libvk_swiftshader.so`, `libvulkan.so.1`), so the .deb, the .rpm and the AppImage fail. The patch
skips the two when CEF has none; Chromium then uses the system's `libEGL.so.1` and
`libGLESv2.so.2`. Everything else CEF needs stays required. The same patch is in
[Schreibzeug](https://github.com/firsttris/schreibzeug/tree/main/apps/desktop/patches).

## Settings for the AppImage

`scripts/tauri.mjs` also sets `ANYLINUX_LIB=0` for quick-sharun, which the bundler runs to make the
AppImage (a value already set wins): its `anylinux.c` was removed from
pkgforge-dev/Anylinux-AppImages, so its download fails; the library is an optional compatibility
helper. Remove it once Tauri's quick-sharun no longer needs it.

## Remove it again

Once a Tauri release copies the two only when they exist:

- delete this folder and `scripts/tauri.mjs`;
- in `package.json`, call `tauri` directly again;
- in `.github/workflows/release.yml`, remove the cache step for the patched CLI;
- remove the section "Patched Tauri CLI" from `docs/development.md`.

`scripts/tauri.mjs` stops with a pointer here when a patch no longer applies to a new CLI version,
which is the usual sign that Tauri changed or fixed that code.
