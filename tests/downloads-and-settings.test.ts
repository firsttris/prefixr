import { beforeEach, describe, expect, it } from "vitest";
import { addSample, estimate, formatDuration } from "#lib/downloadRate.ts";
import { isNumberIn, isOneOf, isStringList, loadSetting, saveSetting } from "#lib/persisted.ts";

describe("download speed", () => {
  it("needs two samples a few seconds apart and a known size", () => {
    const samples = [
      { at: 0, percent: 10 },
      { at: 10_000, percent: 20 },
    ];
    expect(estimate(samples.slice(0, 1), 1e9)).toBeNull();
    expect(estimate(samples, null)).toBeNull();
    expect(
      estimate(
        [
          { at: 0, percent: 10 },
          { at: 1000, percent: 11 },
        ],
        1e9,
      ),
    ).toBeNull();
    // 10 % of 1 GB in 10 s: 10 MB/s, and 80 % left: 80 s.
    expect(estimate(samples, 1e9)).toEqual({ bytesPerSecond: 1e7, secondsLeft: 80 });
  });

  it("keeps only the last 20 seconds, and starts over when progress goes back", () => {
    let samples = addSample([], { at: 0, percent: 5 });
    samples = addSample(samples, { at: 15_000, percent: 10 });
    samples = addSample(samples, { at: 30_000, percent: 20 });
    expect(samples.map((s) => s.at)).toEqual([15_000, 30_000]);
    samples = addSample(samples, { at: 31_000, percent: 0 });
    expect(samples).toEqual([{ at: 31_000, percent: 0 }]);
  });

  it("formats the time left", () => {
    expect(formatDuration(30)).toBe("< 1 min");
    expect(formatDuration(5 * 60)).toBe("5 min");
    expect(formatDuration(3600 + 5 * 60)).toBe("1 h 5 min");
    expect(formatDuration(59.7 * 60)).toBe("1 h 0 min");
    expect(formatDuration(2 * 3600 - 10)).toBe("2 h 0 min");
  });
});

// The tests run in Node, without the browser's storage.
const storage = new Map<string, string>();
globalThis.localStorage = {
  getItem: (key: string) => storage.get(key) ?? null,
  setItem: (key: string, value: string) => void storage.set(key, value),
  clear: () => storage.clear(),
} as Storage;

describe("kept settings", () => {
  beforeEach(() => localStorage.clear());

  it("come back as saved, or as the default when missing or invalid", () => {
    const sort = isOneOf("name-asc", "playtime");
    expect(loadSetting("sort", "name-asc", sort)).toBe("name-asc");
    saveSetting("sort", "playtime");
    expect(loadSetting("sort", "name-asc", sort)).toBe("playtime");
    saveSetting("sort", "made-up");
    expect(loadSetting("sort", "name-asc", sort)).toBe("name-asc");
    localStorage.setItem("broken", "{not json");
    expect(loadSetting("broken", 1, isNumberIn(0, 5))).toBe(1);
  });

  it("check their shape", () => {
    expect(isNumberIn(130, 260)(170)).toBe(true);
    expect(isNumberIn(130, 260)(500)).toBe(false);
    expect(isNumberIn(130, 260)("170")).toBe(false);
    expect(isStringList(["a", "b"])).toBe(true);
    expect(isStringList(["a", 1])).toBe(false);
  });
});
