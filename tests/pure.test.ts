import { describe, expect, it } from "vitest";
import { prettifyExeName } from "$lib/gameName";
import { formatEnvVars, parseEnvVars } from "$lib/envVars";
import { sameBlock, sameFields } from "$lib/settings";
import { loadAll } from "$lib/load";
import { backendError, setLocale } from "$lib/i18n/index.svelte";

describe("prettifyExeName", () => {
  it("turns exe file names into titles", () => {
    expect(prettifyExeName("SomeGameTitle.exe")).toBe("Some Game Title");
    expect(prettifyExeName("DOOMEternal.EXE")).toBe("DOOM Eternal");
    expect(prettifyExeName("witcher3_launcher.exe")).toBe("witcher3 launcher");
    expect(prettifyExeName("Game-Of.The_Year")).toBe("Game Of The Year");
    expect(prettifyExeName("  Already Clean  ")).toBe("Already Clean");
  });
});

describe("env vars", () => {
  it("parses one KEY=value per line", () => {
    expect(
      parseEnvVars("DXVK_HUD=fps\n\n  WINEDLLOVERRIDES = d3d11=n,b \nnot a var\n=nokey\nEMPTY="),
    ).toEqual({ DXVK_HUD: "fps", WINEDLLOVERRIDES: "d3d11=n,b", EMPTY: "" });
  });

  it("round-trips through the text form", () => {
    const vars = { A: "1", B: "x=y" };
    expect(parseEnvVars(formatEnvVars(vars))).toEqual(vars);
  });
});

describe("override folding", () => {
  it("compares numbers with a tolerance and everything else exactly", () => {
    expect(sameFields({ a: 0.1 + 0.2, b: "x" }, { a: 0.3, b: "x" })).toBe(true);
    expect(sameFields({ a: 1, b: "x" }, { a: 1, b: "y" })).toBe(false);
    expect(sameFields({ a: null }, { a: null })).toBe(true);
  });

  it("treats two disabled blocks as the same whatever their values", () => {
    const gamescope = { enabled: false, width: 1920, fullscreen: true };
    expect(sameBlock(gamescope, { enabled: false, width: 1280, fullscreen: false })).toBe(true);
    expect(sameBlock({ ...gamescope, enabled: true }, { enabled: true, width: 1280, fullscreen: true })).toBe(false);
    expect(sameBlock({ ...gamescope, enabled: true }, { ...gamescope, enabled: true })).toBe(true);
  });
});

describe("loadAll", () => {
  it("waits for every load and rejects with the first failure", async () => {
    let finished = false;
    const slow = new Promise((resolve) => setTimeout(() => resolve((finished = true)), 10));
    await expect(loadAll(Promise.reject("first"), slow, Promise.reject("second"))).rejects.toBe("first");
    expect(finished).toBe(true);
    await expect(loadAll(Promise.resolve(1))).resolves.toBeUndefined();
  });
});

describe("backendError", () => {
  it("renders structured errors in the current language", () => {
    setLocale("en");
    expect(backendError({ code: "prefix_in_use", game_name: "Doom" })).toBe(
      "“Doom” is still running in this prefix. Close the game first.",
    );
    setLocale("de");
    expect(backendError({ code: "prefix_in_use", game_name: "Doom" })).toBe(
      "„Doom“ läuft noch in diesem Prefix. Beende das Spiel zuerst.",
    );
  });

  it("passes plain strings through and shows unknown codes as data", () => {
    expect(backendError("Could not read X")).toBe("Could not read X");
    expect(backendError({ code: "from_a_newer_backend", x: 1 })).toBe(
      '{"code":"from_a_newer_backend","x":1}',
    );
    expect(backendError(null)).toBe("");
    expect(backendError(undefined)).toBe("");
  });
});
