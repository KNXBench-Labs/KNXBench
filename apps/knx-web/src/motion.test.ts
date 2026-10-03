import { describe, expect, it, vi } from "vitest";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { runInNewContext } from "node:vm";
import {
  MOTION_LEVELS,
  MOTION_STYLES,
  loadMotionLevel,
  loadMotionStyle,
  saveMotionLevel,
  saveMotionStyle,
} from "./motion";

function fakeStorage(initial: Record<string, string> = {}) {
  const store = { ...initial };
  return {
    getItem: vi.fn((key: string) => store[key] ?? null),
    setItem: vi.fn((key: string, value: string) => {
      store[key] = value;
    }),
  };
}

it("executes the real pre-mount script with a persisted CRT style", () => {
  const html = readFileSync(join(dirname(fileURLToPath(import.meta.url)), "../index.html"), "utf8");
  const script = html.match(/<script>([\s\S]*?)<\/script>/)![1];
  const attributes: Record<string, string> = {};
  runInNewContext(script, {
    localStorage: { getItem: () => JSON.stringify({ settings: { motionStyle: "crt", motionLevel: "standard" } }) },
    matchMedia: () => ({ matches: false }),
    document: { documentElement: { dataset: {}, setAttribute: (key: string, value: string) => { attributes[key] = value; } } },
  });
  expect(attributes["data-motion-style"]).toBe("crt");
});

describe("MOTION_LEVELS", () => {
  it("contains exactly the documented ids, in order", () => {
    expect(MOTION_LEVELS.map((l) => l.id)).toEqual(["off", "subtle", "standard"]);
  });

  it("carries the documented names", () => {
    expect(MOTION_LEVELS.map((l) => l.name)).toEqual(["Off", "Subtle", "Standard"]);
  });
});

describe("MOTION_STYLES", () => {
  it("contains exactly the documented ids, in order", () => {
    expect(MOTION_STYLES.map((s) => s.id)).toEqual(["apple", "glitch", "crt"]);
  });

  it("carries the documented names", () => {
    expect(MOTION_STYLES.map((s) => s.name)).toEqual(["Smooth", "Glitch", "CRT"]);
  });
});

describe("loadMotionLevel / saveMotionLevel", () => {
  it("round-trips a saved value through storage", () => {
    const storage = fakeStorage();
    saveMotionLevel(storage, "subtle");
    expect(loadMotionLevel(storage)).toBe("subtle");
  });

  it("stores the exact key and value", () => {
    const storage = fakeStorage();
    saveMotionLevel(storage, "off");
    expect(storage.setItem).toHaveBeenCalledWith("motionLevel", "off");
  });

  it("falls back to standard when nothing is stored", () => {
    expect(loadMotionLevel(fakeStorage())).toBe("standard");
  });

  it("falls back to standard for a missing key", () => {
    expect(loadMotionLevel(fakeStorage({ "some-other-key": "off" }))).toBe("standard");
  });

  it("falls back to standard for an empty string", () => {
    expect(loadMotionLevel(fakeStorage({ "motionLevel": "" }))).toBe("standard");
  });

  it("falls back to standard for an unknown id", () => {
    expect(loadMotionLevel(fakeStorage({ "motionLevel": "turbo" }))).toBe("standard");
  });
});

describe("loadMotionStyle / saveMotionStyle", () => {
  it("round-trips a saved value through storage", () => {
    const storage = fakeStorage();
    saveMotionStyle(storage, "glitch");
    expect(loadMotionStyle(storage)).toBe("glitch");
  });

  it("stores the exact key and value", () => {
    const storage = fakeStorage();
    saveMotionStyle(storage, "apple");
    expect(storage.setItem).toHaveBeenCalledWith("motionStyle", "apple");
  });

  it("falls back to apple when nothing is stored", () => {
    expect(loadMotionStyle(fakeStorage())).toBe("apple");
  });

  it("falls back to apple for a missing key", () => {
    expect(loadMotionStyle(fakeStorage({ "some-other-key": "glitch" }))).toBe("apple");
  });

  it("falls back to apple for an empty string", () => {
    expect(loadMotionStyle(fakeStorage({ "motionStyle": "" }))).toBe("apple");
  });

  it("falls back to apple for an unknown id", () => {
    expect(loadMotionStyle(fakeStorage({ "motionStyle": "neon" }))).toBe("apple");
  });
});
