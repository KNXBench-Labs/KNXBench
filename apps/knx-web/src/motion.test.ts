import { describe, expect, it, vi } from "vitest";
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
    expect(MOTION_STYLES.map((s) => s.id)).toEqual(["apple", "glitch"]);
  });

  it("carries the documented names", () => {
    expect(MOTION_STYLES.map((s) => s.name)).toEqual(["Smooth", "Glitch"]);
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
    expect(storage.setItem).toHaveBeenCalledWith("knx-desktop:motion-level", "off");
  });

  it("falls back to standard when nothing is stored", () => {
    expect(loadMotionLevel(fakeStorage())).toBe("standard");
  });

  it("falls back to standard for a missing key", () => {
    expect(loadMotionLevel(fakeStorage({ "some-other-key": "off" }))).toBe("standard");
  });

  it("falls back to standard for an empty string", () => {
    expect(loadMotionLevel(fakeStorage({ "knx-desktop:motion-level": "" }))).toBe("standard");
  });

  it("falls back to standard for an unknown id", () => {
    expect(loadMotionLevel(fakeStorage({ "knx-desktop:motion-level": "turbo" }))).toBe("standard");
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
    expect(storage.setItem).toHaveBeenCalledWith("knx-desktop:motion-style", "apple");
  });

  it("falls back to apple when nothing is stored", () => {
    expect(loadMotionStyle(fakeStorage())).toBe("apple");
  });

  it("falls back to apple for a missing key", () => {
    expect(loadMotionStyle(fakeStorage({ "some-other-key": "glitch" }))).toBe("apple");
  });

  it("falls back to apple for an empty string", () => {
    expect(loadMotionStyle(fakeStorage({ "knx-desktop:motion-style": "" }))).toBe("apple");
  });

  it("falls back to apple for an unknown id", () => {
    expect(loadMotionStyle(fakeStorage({ "knx-desktop:motion-style": "neon" }))).toBe("apple");
  });
});
