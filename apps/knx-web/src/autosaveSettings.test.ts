/** Tests for the ISSUE-04 autosave preference readers and writers. */
// Covers: enabled by default, five-minute default, and safe fallbacks for
// anything out of range.
import { describe, expect, it, vi } from "vitest";
import {
  DEFAULT_AUTOSAVE_INTERVAL_MINUTES,
  loadAutosaveEnabled,
  loadAutosaveIntervalMinutes,
  saveAutosaveEnabled,
  saveAutosaveIntervalMinutes,
} from "./autosaveSettings";

function fakeStorage(initial: Record<string, string> = {}) {
  const store = { ...initial };
  return {
    getItem: vi.fn((key: string) => store[key] ?? null),
    setItem: vi.fn((key: string, value: string) => {
      store[key] = value;
    }),
  };
}

describe("DEFAULT_AUTOSAVE_INTERVAL_MINUTES", () => {
  it("is five minutes, per ISSUE-04", () => {
    expect(DEFAULT_AUTOSAVE_INTERVAL_MINUTES).toBe(5);
  });
});

describe("loadAutosaveEnabled / saveAutosaveEnabled", () => {
  it("is enabled by default when nothing is stored", () => {
    expect(loadAutosaveEnabled(fakeStorage())).toBe(true);
  });

  it("round-trips a disabled value through storage", () => {
    const storage = fakeStorage();
    saveAutosaveEnabled(storage, false);
    expect(loadAutosaveEnabled(storage)).toBe(false);
  });

  it("round-trips a re-enabled value through storage", () => {
    const storage = fakeStorage();
    saveAutosaveEnabled(storage, false);
    saveAutosaveEnabled(storage, true);
    expect(loadAutosaveEnabled(storage)).toBe(true);
  });

  it("stores the exact key and string value", () => {
    const storage = fakeStorage();
    saveAutosaveEnabled(storage, false);
    expect(storage.setItem).toHaveBeenCalledWith("autosaveEnabled", "false");
  });

  it("treats any value other than the literal string \"false\" as enabled", () => {
    expect(loadAutosaveEnabled(fakeStorage({ autosaveEnabled: "nonsense" }))).toBe(true);
  });
});

describe("loadAutosaveIntervalMinutes / saveAutosaveIntervalMinutes", () => {
  it("defaults to five minutes when nothing is stored", () => {
    expect(loadAutosaveIntervalMinutes(fakeStorage())).toBe(DEFAULT_AUTOSAVE_INTERVAL_MINUTES);
  });

  it("round-trips a saved value through storage", () => {
    const storage = fakeStorage();
    saveAutosaveIntervalMinutes(storage, 15);
    expect(loadAutosaveIntervalMinutes(storage)).toBe(15);
  });

  it("stores the exact key and value", () => {
    const storage = fakeStorage();
    saveAutosaveIntervalMinutes(storage, 30);
    expect(storage.setItem).toHaveBeenCalledWith("autosaveIntervalMinutes", "30");
  });

  it("falls back to the default for an unparsable value", () => {
    expect(loadAutosaveIntervalMinutes(fakeStorage({ autosaveIntervalMinutes: "not-a-number" }))).toBe(
      DEFAULT_AUTOSAVE_INTERVAL_MINUTES,
    );
  });

  it("falls back to the default below the minimum", () => {
    expect(loadAutosaveIntervalMinutes(fakeStorage({ autosaveIntervalMinutes: "0" }))).toBe(
      DEFAULT_AUTOSAVE_INTERVAL_MINUTES,
    );
  });

  it("falls back to the default above the maximum", () => {
    expect(loadAutosaveIntervalMinutes(fakeStorage({ autosaveIntervalMinutes: "9999" }))).toBe(
      DEFAULT_AUTOSAVE_INTERVAL_MINUTES,
    );
  });

  it("accepts the boundary values", () => {
    expect(loadAutosaveIntervalMinutes(fakeStorage({ autosaveIntervalMinutes: "1" }))).toBe(1);
    expect(loadAutosaveIntervalMinutes(fakeStorage({ autosaveIntervalMinutes: "120" }))).toBe(120);
  });
});
