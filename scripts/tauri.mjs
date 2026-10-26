// The Tauri CLI for Prefixr: built from Tauri's sources at the version in package.json, with the
// patches in ../patches applied (bun run tauri …).
//
// Why: the CLI's bundler copies libEGL.so and libGLESv2.so from CEF into every Linux package
// (crates/tauri-bundler/src/bundle/linux/{debian.rs,rpm.rs,appimage/sharun_cef.rs}), and CEF 154
// no longer ships them, so .deb, .rpm and AppImage all fail. The patch skips the two when CEF has
// none. Drop this script once a Tauri release carries the fix (patches/README.md).
//
// The patched CLI is built once per version and patch set, into src-tauri/target/patched-cli/.
import { spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import {
  copyFileSync,
  existsSync,
  mkdirSync,
  mkdtempSync,
  readdirSync,
  readFileSync,
  rmSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const patches = join(root, "patches");
const cache = join(root, "src-tauri/target/patched-cli");

function run(cmd, cwd = root) {
  const result = spawnSync(cmd[0], cmd.slice(1), { cwd, stdio: "inherit" });
  if (result.status !== 0) throw new Error(`failed: ${cmd.join(" ")}`);
}

function cliVersion() {
  const manifest = JSON.parse(readFileSync(join(root, "package.json"), "utf8"));
  const version = manifest.devDependencies?.["@tauri-apps/cli"];
  if (!version) throw new Error("package.json names no @tauri-apps/cli version");
  return version;
}

function patchFiles() {
  return readdirSync(patches)
    .filter((name) => name.endsWith(".patch"))
    .sort()
    .map((name) => join(patches, name));
}

/** One folder per version and patch set, so a changed patch builds the CLI anew. */
function cachedBinary(version, files) {
  const hash = createHash("sha256");
  for (const file of files) hash.update(readFileSync(file));
  return join(cache, `${version}-${hash.digest("hex").slice(0, 12)}`, "cargo-tauri");
}

/**
 * A patch that no longer applies most likely means Tauri changed or fixed that code: then it is
 * time to check the release and remove the patch (and this script), see ../patches/README.md.
 */
function applyPatch(file, source, version) {
  const check = spawnSync("git", ["apply", "--check", file], { cwd: source, encoding: "utf8" });
  if (check.status !== 0) {
    throw new Error(
      `${file} no longer applies to the Tauri CLI ${version}. If this Tauri release fixes the ` +
        "missing libEGL.so/libGLESv2.so in the Linux bundles, remove the patch and this script " +
        `(patches/README.md).\n${check.stderr}`,
    );
  }
  run(["git", "apply", file], source);
}

function buildPatchedCli(version, files, binary) {
  const source = mkdtempSync(join(tmpdir(), "tauri-cli-"));
  try {
    const tag = `tauri-cli-v${version}`;
    run([
      "git",
      "clone",
      "--quiet",
      "--depth",
      "1",
      "--branch",
      tag,
      "https://github.com/tauri-apps/tauri",
      source,
    ]);
    for (const file of files) applyPatch(file, source, version);
    run(
      ["cargo", "build", "--release", "--locked", "-p", "tauri-cli", "--bin", "cargo-tauri"],
      source,
    );
    mkdirSync(dirname(binary), { recursive: true });
    copyFileSync(join(source, "target/release/cargo-tauri"), binary);
  } finally {
    rmSync(source, { recursive: true, force: true });
  }
}

function patchedCli() {
  const version = cliVersion();
  const files = patchFiles();
  const binary = cachedBinary(version, files);
  if (!existsSync(binary)) {
    console.log(`tauri: building the patched CLI ${version} (once) into ${binary}`);
    buildPatchedCli(version, files, binary);
  }
  return binary;
}

// ANYLINUX_LIB=0 for quick-sharun, which the bundler runs to make the AppImage: its anylinux.c was
// removed from pkgforge-dev/Anylinux-AppImages, so the download fails; the library is an optional
// compatibility helper. A value already set wins.
const result = spawnSync(patchedCli(), process.argv.slice(2), {
  cwd: root,
  env: { ANYLINUX_LIB: "0", ...process.env },
  stdio: "inherit",
});
process.exit(result.status ?? 1);
