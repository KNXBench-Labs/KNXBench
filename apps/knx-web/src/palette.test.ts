import { describe, expect, it } from "vitest";
import { applyPalette, loadPalette, savePalette, TOKENS } from "./palette";
import type { PaletteSettings } from "./palette";

function fakeStorage(initial: Record<string, string> = {}) {
  const store = { ...initial };
  return {
    getItem: (key: string) => (key in store ? store[key] : null),
    setItem: (key: string, value: string) => {
      store[key] = value;
    },
    _store: store,
  };
}

function fakeStyle() {
  const props: Record<string, string> = {};
  return {
    setProperty: (name: string, value: string) => {
      props[name] = value;
    },
    removeProperty: (name: string) => {
      const prev = props[name] ?? "";
      delete props[name];
      return prev;
    },
    _props: props,
  };
}

describe("loadPalette", () => {
  it("resolves to empty colors and standard motion when nothing is stored", () => {
    expect(loadPalette(fakeStorage())).toEqual({ colors: {}, motion: "standard" });
  });

  it("resolves stored colors and motion", () => {
    const stored = JSON.stringify({ colors: { accent: "#ff0000" }, motion: "subtle" });
    expect(loadPalette(fakeStorage({ "knx-desktop:palette": stored }))).toEqual({
      colors: { accent: "#ff0000" },
      motion: "subtle",
    });
  });

  it("drops unknown token keys from stored colors", () => {
    const stored = JSON.stringify({ colors: { accent: "#ff0000", solarized: "#00ff00" }, motion: "standard" });
    expect(loadPalette(fakeStorage({ "knx-desktop:palette": stored }))).toEqual({
      colors: { accent: "#ff0000" },
      motion: "standard",
    });
  });

  it("falls back to standard motion for an unrecognized stored value", () => {
    const stored = JSON.stringify({ colors: {}, motion: "turbo" });
    expect(loadPalette(fakeStorage({ "knx-desktop:palette": stored }))).toEqual({
      colors: {},
      motion: "standard",
    });
  });

  it("falls back to defaults for malformed JSON", () => {
    expect(loadPalette(fakeStorage({ "knx-desktop:palette": "not json" }))).toEqual({
      colors: {},
      motion: "standard",
    });
  });
});

describe("savePalette", () => {
  it("writes settings as JSON under the expected key", () => {
    const storage = fakeStorage();
    const settings: PaletteSettings = { colors: { accent: "#123456" }, motion: "off" };
    savePalette(storage, settings);
    expect(JSON.parse(storage._store["knx-desktop:palette"])).toEqual(settings);
  });
});

describe("applyPalette", () => {
  it("sets a CSS custom property for every overridden token", () => {
    const style = fakeStyle();
    applyPalette({ style }, { colors: { accent: "#ff0000" }, motion: "standard" });
    expect(style._props["--knx-accent"]).toBe("#ff0000");
  });

  it("removes the CSS custom property for tokens without an override", () => {
    const style = fakeStyle();
    style.setProperty("--knx-accent", "#leftover");
    applyPalette({ style }, { colors: {}, motion: "standard" });
    for (const token of TOKENS) {
      expect(style._props[`--knx-${token}`]).toBeUndefined();
    }
  });

  it("sets the transition duration for the given motion level", () => {
    const style = fakeStyle();
    applyPalette({ style }, { colors: {}, motion: "off" });
    expect(style._props["--knx-transition-duration"]).toBe("0ms");
  });
});
