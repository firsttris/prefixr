#!/usr/bin/env python3
"""Completes package-lock.json from bun.lock, for flatpak-node-generator.

The Flatpak builds the frontend with npm (flatpak-node-generator doesn't read bun.lock), but it has
to use exactly the packages bun.lock pins. `npm install --package-lock-only` next to bun's
node_modules writes the same tree, though without the `resolved` and `integrity` fields the
offline sources need. This fills them in from bun.lock and fails when a version differs from
bun.lock or a package is missing there. See flatpak/update-sources.sh.
"""

import json
import re
import sys
from pathlib import Path

REGISTRY = "https://registry.npmjs.org"


def read_bun_lock(path: Path) -> dict:
    # JSON with trailing commas.
    text = re.sub(r",(\s*[}\]])", r"\1", path.read_text(encoding="utf-8"))
    return json.loads(text)["packages"]


def bun_key(npm_key: str) -> str:
    # node_modules/a/node_modules/@s/b -> a/@s/b
    return "/".join(part for part in npm_key.split("node_modules/") if part).replace("//", "/").strip("/")


def main() -> int:
    root = Path(sys.argv[1] if len(sys.argv) > 1 else ".")
    lock_path = root / "package-lock.json"
    lock = json.loads(lock_path.read_text(encoding="utf-8"))
    bun = read_bun_lock(root / "bun.lock")
    errors = []
    for key, entry in lock["packages"].items():
        if not key or entry.get("link"):
            continue
        # bun and npm can nest a package differently (bun hoists `yaml`, npm keeps it below
        # postcss-load-config): then its entry under the plain name.
        package = key.rsplit("node_modules/", 1)[-1]
        found = bun.get(bun_key(key)) or bun.get(package)
        if found is None:
            errors.append(f"{key}: not in bun.lock")
            continue
        spec, _registry, _meta, integrity = found[:4]
        name, _, version = spec.rpartition("@")
        if version != entry.get("version"):
            # A package bun didn't install here (an optional one for another platform, like
            # @napi-rs/wasm-runtime) is resolved anew by npm, possibly to a newer version:
            # bun.lock's wins. One bun did install has to match.
            if (root / key).exists():
                errors.append(f"{key}: {entry.get('version')} in package-lock.json, {version} in bun.lock")
                continue
            entry["version"] = version
        basename = name.split("/")[-1]
        entry["resolved"] = f"{REGISTRY}/{name}/-/{basename}-{version}.tgz"
        entry["integrity"] = integrity
    if errors:
        print("package-lock.json doesn't match bun.lock:", *errors, sep="\n  ", file=sys.stderr)
        return 1
    lock_path.write_text(json.dumps(lock, indent=2) + "\n", encoding="utf-8")
    return 0


if __name__ == "__main__":
    sys.exit(main())
