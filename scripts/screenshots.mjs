// Takes the screenshots in docs/ from the demo library (scripts/screenshots-demo.mjs), in English.
//   bun run screenshots (scripts/screenshots.sh builds the frontend and runs this script), or by hand:
//   bun run build && node scripts/screenshots.mjs [docs]
// The frontend runs in a plain browser: a stand-in for Tauri's IPC answers its commands, and a small
// server here serves build/ and the demo covers. CHROMIUM_PATH picks another Chromium.

import { readFile } from "node:fs/promises";
import { createServer } from "node:http";
import { extname, join, normalize } from "node:path";
import { chromium } from "@playwright/test";
import { COVERS, coverSvg, demoData, installDemo } from "./screenshots-demo.mjs";

const OUT = process.argv[2] ?? "docs";
const BUILD = "build";
const TYPES = {
  ".html": "text/html",
  ".js": "text/javascript",
  ".css": "text/css",
  ".json": "application/json",
  ".png": "image/png",
  ".svg": "image/svg+xml",
  ".woff2": "font/woff2",
};

// build/ as the app sees it, index.html for every other path (adapter-static's fallback), and the
// covers under /demo/artwork/<name>.svg.
const server = createServer(async (req, res) => {
  const path = decodeURIComponent(new URL(req.url, "http://localhost").pathname);
  const cover = path.match(/^\/demo\/artwork\/(.+)\.svg$/);
  if (cover) {
    const known = COVERS[cover[1]];
    res.writeHead(known ? 200 : 404, { "content-type": TYPES[".svg"] });
    res.end(known ? coverSvg(known) : "");
    return;
  }
  const file = join(BUILD, normalize(path).replace(/^(\.\.[/\\])+/, ""));
  const body = await readFile(file).catch(() => null);
  if (body) {
    res.writeHead(200, { "content-type": TYPES[extname(file)] ?? "application/octet-stream" });
    res.end(body);
  } else {
    res.writeHead(200, { "content-type": TYPES[".html"] });
    res.end(await readFile(join(BUILD, "index.html")));
  }
});
await new Promise((resolve) => server.listen(0, "127.0.0.1", resolve));
const base = `http://127.0.0.1:${server.address().port}`;

const { version } = JSON.parse(await readFile("package.json", "utf8"));
const browser = await chromium.launch({ executablePath: process.env.CHROMIUM_PATH || undefined });
// The size of the app's window (src-tauri/tauri.conf.json is 1180×780), a little wider.
const context = await browser.newContext({
  viewport: { width: 1280, height: 800 },
  deviceScaleFactor: 1,
  locale: "en-US",
  timezoneId: "UTC",
  colorScheme: "dark",
  reducedMotion: "reduce",
});
await context.addInitScript(installDemo, demoData(version));
const page = await context.newPage();
// Nothing depends on the time of day; Date.now() is still fixed, so a picture never does.
await page.clock.setFixedTime(new Date("2026-06-15T12:00:00Z"));

let problems = 0;
page.on("console", (msg) => {
  if (msg.type() === "warning" || msg.type() === "error") {
    problems++;
    console.log(`page ${msg.type()}: ${msg.text()}`);
  }
});
page.on("pageerror", (error) => {
  problems++;
  console.log(`page error: ${error.message}`);
});

/** Waits until the page is still and every image in view has loaded. */
async function settle() {
  await page.waitForLoadState("networkidle");
  await page.evaluate(async () => {
    await document.fonts.ready;
    const inView = [...document.images].filter((img) => {
      const box = img.getBoundingClientRect();
      return box.bottom > 0 && box.top < innerHeight && box.width > 0;
    });
    await Promise.all(inView.map((img) => img.decode().catch(() => {})));
  });
  await page.waitForTimeout(300);
}

async function shot(name) {
  await settle();
  await page.screenshot({ path: `${OUT}/screenshot-${name}.png`, animations: "disabled" });
  console.log("shot", name);
}

// A sidebar entry, by its label (Downloads has its count after it).
const nav = (label) =>
  page.locator("nav.sidebar").getByRole("button", { name: new RegExp(`^${label}\\b`) });

await page.goto(`${base}/`);
await page.getByRole("heading", { name: "Library" }).waitFor();
await page.locator(".cover-hit").first().waitFor();
await shot("library");

// The details panel of the first game.
await page.getByRole("button", { name: "Show details of Starfall Odyssey" }).first().click();
await page.getByRole("dialog", { name: "Starfall Odyssey" }).waitFor();
await shot("game");
await page.keyboard.press("Escape");

await nav("Runners").click();
await page.getByText("GE-Proton10-4").first().waitFor();
await shot("runners");

// The runner downloads, over the runners page.
await page.getByRole("button", { name: "Download runner" }).click();
await page.getByText("GE-Proton10-6").waitFor();
await shot("runner-downloads");
await page.keyboard.press("Escape");

// The store installs in progress (the library page above picked them up).
await nav("Downloads").click();
await page.getByText("Skyforge Arena").waitFor();
await shot("downloads");

await nav("Overlay").click();
await page.locator("main .save-row").waitFor();
await shot("settings");

await browser.close();
server.close();
if (problems > 0) {
  console.error(`${problems} warning(s) or error(s) in the page, see above`);
  process.exit(1);
}
