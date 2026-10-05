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
  it("no longer offers Neon Grid or Bitcoin DeFi (removed 2026-10-05, ADR-0079)", () => {
    expect(THEMES.map((t) => t.id)).toEqual(["system", "porcelain", "graphite", "cupertino"]);
  });

  it("offers at least three palettes besides the System entry", () => {
    expect(THEMES.filter((t) => t.id !== "system").length).toBeGreaterThanOrEqual(3);
  });

  it("gives every theme a unique id and a name", () => {
    expect(new Set(THEMES.map((t) => t.id)).size).toBe(THEMES.length);
    expect(THEMES.every((t) => t.name.trim() !== "")).toBe(true);
  });

  // `themeTokens.test.ts` checks every palette's `hasAccentVariations`
  // against styles.css. It cannot check `system`'s, because `system` has no
  // block — which left that one entry hand-typed and unexamined. Setting it
  // to `false` disabled the accent control for the default theme and the
  // whole suite stayed green.
  it("keeps System's accent flag in step with the palettes it resolves to", () => {
    const system = THEMES.find((t) => t.id === "system");
    expect(system, "the registry no longer offers a System entry").toBeDefined();
    const resolved = [resolveThemeId("system", false), resolveThemeId("system", true)];
    for (const id of resolved) {
      const palette = THEMES.find((t) => t.id === id);
      expect(palette, `System resolves to "${id}", which is not in the registry`).toBeDefined();
      expect(
        system!.hasAccentVariations,
        `System offers accents ${system!.hasAccentVariations} but resolves to "${id}", which offers ${palette!.hasAccentVariations}`,
      ).toBe(palette!.hasAccentVariations);
    }
  });
});

describe("resolveThemeId", () => {
  it("resolves System by the OS preference", () => {
    expect(resolveThemeId("system", false)).toBe("porcelain");
    expect(resolveThemeId("system", true)).toBe("graphite");
  });

  it("leaves an explicit choice alone whatever the OS prefers", () => {
    expect(resolveThemeId("graphite", false)).toBe("graphite");
    expect(resolveThemeId("cupertino", true)).toBe("cupertino");
  });
});

describe("loadThemeId", () => {
  it("defaults to system when nothing is stored", () => {
    expect(loadThemeId(fakeStorage())).toBe("system");
  });

  it("returns a known stored id unchanged", () => {
    expect(loadThemeId(fakeStorage({ "theme": "cupertino" }))).toBe("cupertino");
  });

  it("shows a saved choice of a removed palette as System", () => {
    expect(loadThemeId(fakeStorage({ "theme": "neon-grid" }))).toBe("system");
    expect(loadThemeId(fakeStorage({ "theme": "bitcoin-defi" }))).toBe("system");
  });

  it("falls back to system for an unknown id", () => {
    expect(loadThemeId(fakeStorage({ "theme": "solarized" }))).toBe("system");
  });

  it("preserves the intent of old System/Light/Dark values", () => {
    expect(loadThemeId(fakeStorage({ "theme": "system" }))).toBe("system");
    expect(loadThemeId(fakeStorage({ "theme": "light" }))).toBe("porcelain");
    expect(loadThemeId(fakeStorage({ "theme": "dark" }))).toBe("graphite");
  });
});

describe("saveThemeId", () => {
  it("stores the exact key and value", () => {
    const storage = fakeStorage();
    saveThemeId(storage, "cupertino");
    expect(storage.setItem).toHaveBeenCalledWith("theme", "cupertino");
  });
});

describe("persistence", () => {
  // The acceptance criterion in prose: whatever was chosen comes back.
  it.each(THEMES.map((t) => t.id))("restores %s through the one store", (id) => {
    const storage = fakeStorage();
    saveThemeId(storage, id);
    expect(storage.setItem).toHaveBeenCalledWith("theme", id);
    expect(loadThemeId(storage)).toBe(id);
  });
});
