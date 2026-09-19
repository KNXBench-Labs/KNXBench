import { describe, expect, it, vi } from "vitest";
import { THEMES, loadThemeId, resolveThemeId, saveThemeId } from "./theme";

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

  it("offers at least three palettes besides the System entry", () => {
    expect(THEMES.filter((t) => t.id !== "system").length).toBeGreaterThanOrEqual(3);
  });

  it("gives every theme a unique id and a name", () => {
    expect(new Set(THEMES.map((t) => t.id)).size).toBe(THEMES.length);
    expect(THEMES.every((t) => t.name.trim() !== "")).toBe(true);
  });
});

describe("resolveThemeId", () => {
  it("resolves System by the OS preference", () => {
    expect(resolveThemeId("system", false)).toBe("porcelain");
    expect(resolveThemeId("system", true)).toBe("graphite");
  });

  it("leaves an explicit choice alone whatever the OS prefers", () => {
    expect(resolveThemeId("neon-grid", false)).toBe("neon-grid");
    expect(resolveThemeId("cupertino", true)).toBe("cupertino");
  });
});

describe("loadThemeId", () => {
  it("defaults to system when nothing is stored", () => {
    expect(loadThemeId(fakeStorage())).toBe("system");
  });

  it("returns a known stored id unchanged", () => {
    expect(loadThemeId(fakeStorage({ "knx-desktop:theme": "bitcoin-defi" }))).toBe("bitcoin-defi");
  });

  it("falls back to system for an unknown id", () => {
    expect(loadThemeId(fakeStorage({ "knx-desktop:theme": "solarized" }))).toBe("system");
  });

  it("preserves the intent of old System/Light/Dark values", () => {
    expect(loadThemeId(fakeStorage({ "knx-desktop:theme": "system" }))).toBe("system");
    expect(loadThemeId(fakeStorage({ "knx-desktop:theme": "light" }))).toBe("porcelain");
    expect(loadThemeId(fakeStorage({ "knx-desktop:theme": "dark" }))).toBe("graphite");
  });
});

describe("saveThemeId", () => {
  it("stores the exact key and value", () => {
    const storage = fakeStorage();
    saveThemeId(storage, "bitcoin-defi");
    expect(storage.setItem).toHaveBeenCalledWith("knx-desktop:theme", "bitcoin-defi");
  });
});

describe("persistence", () => {
  // The acceptance criterion in prose: whatever was chosen comes back.
  it.each(THEMES.map((t) => t.id))("restores %s through the one store", (id) => {
    const storage = fakeStorage();
    saveThemeId(storage, id);
    expect(storage.setItem).toHaveBeenCalledWith("knx-desktop:theme", id);
    expect(loadThemeId(storage)).toBe(id);
  });
});
