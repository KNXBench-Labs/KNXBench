import { describe, expect, it, vi } from "vitest";
import { THEMES, loadThemeId, saveThemeId } from "./theme";

function fakeStorage(initial: Record<string, string> = {}) {
  const store = { ...initial };
  return {
    getItem: vi.fn((key: string) => store[key] ?? null),
    setItem: vi.fn((key: string, value: string) => {
      store[key] = value;
    }),
  };
}

describe("THEMES", () => {
  it("includes bitcoin-defi", () => {
    expect(THEMES.some((t) => t.id === "bitcoin-defi")).toBe(true);
  });
});

describe("loadThemeId", () => {
  it("defaults to bitcoin-defi when nothing is stored", () => {
    expect(loadThemeId(fakeStorage())).toBe("bitcoin-defi");
  });

  it("returns a known stored id unchanged", () => {
    expect(loadThemeId(fakeStorage({ "knx-desktop:theme": "bitcoin-defi" }))).toBe("bitcoin-defi");
  });

  it("falls back to bitcoin-defi for an unknown id", () => {
    expect(loadThemeId(fakeStorage({ "knx-desktop:theme": "solarized" }))).toBe("bitcoin-defi");
  });

  it("falls back to bitcoin-defi for cycle 7's old System/Light/Dark values", () => {
    expect(loadThemeId(fakeStorage({ "knx-desktop:theme": "system" }))).toBe("bitcoin-defi");
    expect(loadThemeId(fakeStorage({ "knx-desktop:theme": "light" }))).toBe("bitcoin-defi");
    expect(loadThemeId(fakeStorage({ "knx-desktop:theme": "dark" }))).toBe("bitcoin-defi");
  });
});

describe("saveThemeId", () => {
  it("stores the exact key and value", () => {
    const storage = fakeStorage();
    saveThemeId(storage, "bitcoin-defi");
    expect(storage.setItem).toHaveBeenCalledWith("knx-desktop:theme", "bitcoin-defi");
  });
});
