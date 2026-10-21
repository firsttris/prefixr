#!/bin/sh
# Regenerates package-lock.json and the offline sources for flatpak/io.github.firsttris.Prefixr.yml
# from bun.lock and src-tauri/Cargo.lock. Run it whenever either lock file changes; CI checks that
# the committed files match.
#
# Needs bun, npm and flatpak-builder-tools (https://github.com/flatpak/flatpak-builder-tools): set
# FBT to a checkout of it, with flatpak-node-generator on PATH (pipx install "$FBT/node") and the
# cargo generator's Python dependencies (aiohttp, tomlkit, PyYAML).
set -eu
cd "$(dirname "$0")/.."
: "${FBT:?set FBT to a flatpak-builder-tools checkout}"

# npm writes the tree bun installed (its exact versions), then lock-from-bun.py adds the download
# URLs and checksums from bun.lock.
bun install --frozen-lockfile
rm -f package-lock.json
npm install --package-lock-only --ignore-scripts --no-audit --no-fund
python3 flatpak/lock-from-bun.py

python3 "$FBT/cargo/flatpak-cargo-generator.py" -o flatpak/cargo-sources.json src-tauri/Cargo.lock
flatpak-node-generator --no-requests-cache -o flatpak/node-sources.json npm package-lock.json
