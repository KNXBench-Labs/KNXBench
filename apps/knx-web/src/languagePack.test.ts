// @vitest-environment happy-dom
//
// happy-dom, not node: the installed-packs store lazily reads
// `window.localStorage` on its first call in a given test run, the same
// reasoning as `uiLanguage.test.tsx`/`productLanguage.test.tsx`.
import { afterEach, describe, expect, it } from "vitest";
import {
  LANGUAGE_PACKS_STORAGE_KEY,
  type LanguagePack,
  exportEnglishTemplate,
  exportLanguagePack,
  getLanguagePack,
  importLanguagePack,
  isWellFormedBcp47Tag,
  listLanguagePacks,
  parseLanguagePack,
  removeLanguagePack,
  resetLanguagePacksForTests,
} from "./languagePack";

afterEach(() => {
  window.localStorage.removeItem(LANGUAGE_PACKS_STORAGE_KEY);
  resetLanguagePacksForTests();
});

function dutchPack(overrides: Partial<LanguagePack> = {}): LanguagePack {
  return {
    formatVersion: 1,
    tag: "nl-NL",
    name: "Nederlands",
    englishName: "Dutch",
    basedOn: "en",
    packVersion: "1.0.0",
    messages: {
      "toolbar.save": "Opslaan",
    },
    ...overrides,
  };
}

describe("isWellFormedBcp47Tag", () => {
  it("accepts unregistered-but-well-formed tags: Klingon, Bavarian, private-use Sindarin", () => {
    expect(isWellFormedBcp47Tag("tlh")).toBe(true);
    expect(isWellFormedBcp47Tag("bar")).toBe(true);
    expect(isWellFormedBcp47Tag("art-x-sindarin")).toBe(true);
    expect(isWellFormedBcp47Tag("nl-NL")).toBe(true);
  });

  it("rejects a tag that isn't even shaped like one", () => {
    expect(isWellFormedBcp47Tag("")).toBe(false);
    expect(isWellFormedBcp47Tag("xx-not-a-language")).toBe(false);
    expect(isWellFormedBcp47Tag("en_US")).toBe(false);
    expect(isWellFormedBcp47Tag("en US")).toBe(false);
  });
});

describe("parseLanguagePack", () => {
  it("accepts a minimal valid pack", () => {
    const result = parseLanguagePack({
      formatVersion: 1,
      tag: "tlh",
      name: "tlhIngan Hol",
      messages: { "toolbar.save": "polD" },
    });
    expect(result.ok).toBe(true);
  });

  it("rejects a non-object", () => {
    const result = parseLanguagePack("not an object");
    expect(result.ok).toBe(false);
  });

  it("rejects a missing required field with a useful message", () => {
    const result = parseLanguagePack({ formatVersion: 1, name: "X", messages: {} });
    expect(result.ok).toBe(false);
    if (!result.ok) expect(result.error).toMatch(/tag/i);
  });

  it("rejects a malformed tag and says what a well-formed one looks like", () => {
    const result = parseLanguagePack({
      formatVersion: 1,
      tag: "xx-not-a-language",
      name: "X",
      messages: {},
    });
    expect(result.ok).toBe(false);
    if (!result.ok) {
      expect(result.error).toMatch(/tag/i);
      expect(result.error).toMatch(/BCP 47/i);
    }
  });

  it("rejects a non-string message value", () => {
    const result = parseLanguagePack({
      formatVersion: 1,
      tag: "nl-NL",
      name: "Nederlands",
      messages: { "toolbar.save": 42 },
    });
    expect(result.ok).toBe(false);
  });

  it("preserves an unknown top-level field", () => {
    const result = parseLanguagePack({
      formatVersion: 2,
      tag: "nl-NL",
      name: "Nederlands",
      messages: {},
      futureFeature: { some: "nested value from a later format version" },
    });
    expect(result.ok).toBe(true);
    if (result.ok) {
      expect(result.pack.futureFeature).toEqual({
        some: "nested value from a later format version",
      });
    }
  });
});

describe("importLanguagePack", () => {
  it("a valid pack imports and installs", () => {
    const result = importLanguagePack(dutchPack());
    expect(result.ok).toBe(true);
    expect(getLanguagePack("nl-NL")?.messages["toolbar.save"]).toBe("Opslaan");
  });

  it("a pack that fails validation is rejected with the reason, and is not installed", () => {
    const result = importLanguagePack({ formatVersion: 1, name: "X", messages: {} });
    expect(result.ok).toBe(false);
    expect(getLanguagePack("nl-NL")).toBeUndefined();
  });

  it("reports keys the pack has that this build doesn't know, without dropping them", () => {
    const result = importLanguagePack(
      dutchPack({ messages: { "toolbar.save": "Opslaan", "some.future.key": "???" } }),
    );
    expect(result.ok).toBe(true);
    if (result.ok) {
      expect(result.report.unknownKeys).toContain("some.future.key");
      expect(result.report.appliedKeyCount).toBe(1);
    }
    // Not dropped: still installed on the pack itself.
    expect(getLanguagePack("nl-NL")?.messages["some.future.key"]).toBe("???");
  });

  it("reports keys this build has that the pack doesn't translate, as a count plus a sample", () => {
    const result = importLanguagePack(dutchPack());
    expect(result.ok).toBe(true);
    if (result.ok) {
      expect(result.report.missingKeyCount).toBeGreaterThan(0);
      expect(result.report.missingKeysSample.length).toBeGreaterThan(0);
      expect(result.report.missingKeysSample.length).toBeLessThanOrEqual(5);
    }
  });

  it("reports whether plural rules are available for the tag", () => {
    const supported = importLanguagePack(dutchPack());
    expect(supported.ok).toBe(true);
    if (supported.ok) expect(supported.report.pluralRulesSupported).toBe(true);

    const unsupported = importLanguagePack(
      dutchPack({ tag: "art-x-sindarin", name: "Sindarin" }),
    );
    expect(unsupported.ok).toBe(true);
    if (unsupported.ok) expect(unsupported.report.pluralRulesSupported).toBe(false);
  });

  it("a pack that translates a handful of keys is legitimate, not an error", () => {
    const result = importLanguagePack(dutchPack());
    expect(result.ok).toBe(true);
  });

  it("re-importing the same tag replaces the previous install", () => {
    importLanguagePack(dutchPack());
    importLanguagePack(dutchPack({ messages: { "toolbar.save": "Bewaren" } }));
    expect(getLanguagePack("nl-NL")?.messages["toolbar.save"]).toBe("Bewaren");
  });
});

describe("the installed-packs store", () => {
  it("lists installed packs", () => {
    importLanguagePack(dutchPack());
    expect(listLanguagePacks().map((p) => p.tag)).toEqual(["nl-NL"]);
  });

  it("removing a pack is a no-op when it isn't installed", () => {
    expect(() => removeLanguagePack("nl-NL")).not.toThrow();
  });

  it("removing an installed pack removes it", () => {
    importLanguagePack(dutchPack());
    removeLanguagePack("nl-NL");
    expect(getLanguagePack("nl-NL")).toBeUndefined();
  });

  it("a pack persists across a store reset (simulating a reload)", () => {
    importLanguagePack(dutchPack());
    resetLanguagePacksForTests();
    expect(getLanguagePack("nl-NL")?.name).toBe("Nederlands");
  });
});

describe("export", () => {
  it("exports the English catalogue as a template pack that re-imports cleanly", () => {
    const template = exportEnglishTemplate();
    // Round-trip through JSON, the same as writing it to a file and
    // reading it back.
    const roundTripped = JSON.parse(JSON.stringify(template)) as unknown;
    const parsed = parseLanguagePack(roundTripped);
    expect(parsed.ok).toBe(true);
    expect(Object.keys(template.messages).length).toBeGreaterThan(0);
  });

  it("exports an installed pack back out, unknown fields and all", () => {
    importLanguagePack(dutchPack({ futureField: "keep me" } as unknown as LanguagePack));
    const exported = exportLanguagePack("nl-NL");
    expect(exported?.futureField).toBe("keep me");
  });

  it("exporting a pack that isn't installed returns undefined", () => {
    expect(exportLanguagePack("xx")).toBeUndefined();
  });
});
