/** Regression checks for shipped playful catalogues, placeholders, fallback and user overrides. */
// @vitest-environment happy-dom
import { afterEach, describe, expect, it } from "vitest";
import { BUNDLED_LANGUAGE_PACKS } from "./bundledLanguagePacks";
import { translateFor } from "./i18n";
import { exportLanguagePack, getLanguagePack, importLanguagePack, listLanguagePacks, parseLanguagePack, removeLanguagePack, resetLanguagePacksForTests } from "./languagePack";
import { messages as en, type MessageKey } from "./messages/en";
import { resetSettingsForTests } from "./settingsStore";

const placeholders = (value: string) => [...value.matchAll(/\{(\w+)\}/g)].map((match) => match[1]).sort();

afterEach(() => { resetSettingsForTests(); resetLanguagePacksForTests(); });

describe.each(BUNDLED_LANGUAGE_PACKS)("shipped $tag pack", (pack) => {
  it("is a valid, substantial partial pack whose keys and placeholders match English", () => {
    expect(parseLanguagePack(pack).ok).toBe(true);
    expect(Object.keys(pack.messages).length).toBeGreaterThanOrEqual(250);
    for (const [key, value] of Object.entries(pack.messages)) {
      expect(en, key).toHaveProperty(key);
      expect(value.trim(), key).not.toBe("");
      expect(placeholders(value), key).toEqual(placeholders(en[key as MessageKey]));
    }
  });

  it("works without a settings write or import, including placeholder substitution", () => {
    expect(listLanguagePacks()).toEqual([]);
    expect(getLanguagePack(pack.tag)).toEqual(pack);
    expect(translateFor(pack.tag, "toolbar.save")).toBe(pack.messages["toolbar.save"]);
    expect(translateFor(pack.tag, "statusBar.lastSaved", { time: "12:34" })).toContain("12:34");
    expect(listLanguagePacks()).toEqual([]);
  });

  it("leaves detailed safety and provenance notices on the exact English fallback", () => {
    for (const key of ["activityHistory.validationBoundary", "settings.debugServiceControlWarning", "newProject.conflictBody", "flow.explain"] as const) {
      expect(pack.messages[key]).toBeUndefined();
      expect(translateFor(pack.tag, key)).toBe(en[key]);
    }
  });

  it("roundtrips as JSON through the existing pack format", () => {
    const exported = exportLanguagePack(pack.tag);
    const parsed = parseLanguagePack(JSON.parse(JSON.stringify(exported)));
    expect(parsed.ok).toBe(true);
    if (parsed.ok) expect(parsed.pack).toEqual(pack);
  });

  it("honors an imported replacement without merging missing keys from the shipped pack", () => {
    expect(importLanguagePack({ ...pack, name: "My custom pack", messages: { "toolbar.save": "Custom save" }, futureField: { preserve: true } }).ok).toBe(true);
    expect(translateFor(pack.tag, "toolbar.save")).toBe("Custom save");
    expect(translateFor(pack.tag, "toolbar.settings")).toBe(en["toolbar.settings"]);
    expect(exportLanguagePack(pack.tag)?.futureField).toEqual({ preserve: true });
    resetLanguagePacksForTests();
    expect(translateFor(pack.tag, "toolbar.save")).toBe("Custom save");
    removeLanguagePack(pack.tag);
    expect(translateFor(pack.tag, "toolbar.save")).toBe(pack.messages["toolbar.save"]);
    expect(listLanguagePacks()).toEqual([]);
  });
});
