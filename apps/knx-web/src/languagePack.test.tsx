/** Tests for language-pack validation, import, storage, export, and the useLanguagePacks hook. */
// @vitest-environment happy-dom
//
// happy-dom, not node: the installed-packs store lazily reads
// `window.localStorage` on its first call in a given test run, the same
// reasoning as `uiLanguage.test.tsx`/`productLanguage.test.tsx`. The
// `useLanguagePacks` tests below also render real components through
// `react-dom/client`'s `createRoot`, same as `uiLanguage.test.tsx`'s
// writer/reader test.
import { act, useState } from "react";
import { createRoot } from "react-dom/client";
import { afterEach, describe, expect, it, vi } from "vitest";
import {
  BCP47_SHAPE_HINT,
  LANGUAGE_PACKS_STORAGE_KEY,
  type LanguagePack,
  exportEnglishTemplate,
  exportLanguagePack,
  getLanguagePack,
  grandfatheredHint,
  importLanguagePack,
  isWellFormedBcp47Tag,
  listLanguagePacks,
  parseLanguagePack,
  removeLanguagePack,
  resetLanguagePacksForTests,
  useLanguagePacks,
} from "./languagePack";
import { messages as enMessages } from "./messages/en";
import { getSetting, resetSettingsForTests } from "./settingsStore";

afterEach(() => {
  resetSettingsForTests();
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

// Moved here from `SettingsPanel.tsx` (fix round 1): a second UI surface
// should not need to keep its own copy of an RFC 5646 §4.5 lookup table.
describe("grandfatheredHint", () => {
  it("maps a known grandfathered tag to its modern replacement", () => {
    expect(grandfatheredHint("i-klingon")).toBe("tlh");
    expect(grandfatheredHint("sgn-be-nl")).toBe("vgt");
  });

  it("is case-insensitive", () => {
    expect(grandfatheredHint("I-Klingon")).toBe("tlh");
  });

  it("returns undefined for a tag that isn't a grandfathered form", () => {
    expect(grandfatheredHint("nl-NL")).toBeUndefined();
    expect(grandfatheredHint("xx-not-a-language")).toBeUndefined();
  });

  it("returns undefined for a non-string input", () => {
    expect(grandfatheredHint(undefined)).toBeUndefined();
    expect(grandfatheredHint(42)).toBeUndefined();
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

  // KNOWN_LIMITATIONS.md §67's fix: every rejection now carries a
  // structured `reason` alongside the English `error` prose, so a caller
  // (`SettingsPanel.tsx`) can translate it instead of rendering `error`
  // verbatim. One assertion per validation rule, checked here at the
  // `parseLanguagePack` level; §67 itself (translated *inside* a
  // translated sentence, in German) is `SettingsPanel.test.tsx`'s job.
  it("every rejection carries a structured reason, not just English prose", () => {
    // Fix round 1 (Q1): every branch below asserts `.ok === false` before
    // reading `.reason` — without that guard, a `parseLanguagePack` that
    // wrongly returned `{ ok: true, ... }` would skip every `if (!x.ok)`
    // block and the test would still report a pass. Re-run with
    // `parseLanguagePack` short-circuited to always return `{ ok: true }`
    // to see it fail instead: it does, on the first assertion.
    const notObject = parseLanguagePack("not an object");
    expect(notObject.ok).toBe(false);
    if (!notObject.ok) expect(notObject.reason).toEqual({ kind: "notObject" });

    const noFormatVersion = parseLanguagePack({ tag: "nl-NL", name: "X", messages: {} });
    expect(noFormatVersion.ok).toBe(false);
    if (!noFormatVersion.ok) expect(noFormatVersion.reason).toEqual({ kind: "formatVersionMissing" });

    const noTag = parseLanguagePack({ formatVersion: 1, name: "X", messages: {} });
    expect(noTag.ok).toBe(false);
    if (!noTag.ok) expect(noTag.reason).toEqual({ kind: "tagMissing" });

    const badTag = parseLanguagePack({
      formatVersion: 1,
      tag: "xx-not-a-language",
      name: "X",
      messages: {},
    });
    expect(badTag.ok).toBe(false);
    if (!badTag.ok) expect(badTag.reason).toEqual({ kind: "tagMalformed", tag: "xx-not-a-language" });

    const noName = parseLanguagePack({ formatVersion: 1, tag: "nl-NL", messages: {} });
    expect(noName.ok).toBe(false);
    if (!noName.ok) expect(noName.reason).toEqual({ kind: "nameMissing" });

    const noMessages = parseLanguagePack({ formatVersion: 1, tag: "nl-NL", name: "X" });
    expect(noMessages.ok).toBe(false);
    if (!noMessages.ok) expect(noMessages.reason).toEqual({ kind: "messagesMissing" });

    const badMessageValue = parseLanguagePack({
      formatVersion: 1,
      tag: "nl-NL",
      name: "X",
      messages: { "toolbar.save": 42 },
    });
    expect(badMessageValue.ok).toBe(false);
    if (!badMessageValue.ok) {
      expect(badMessageValue.reason).toEqual({
        kind: "messageValueNotString",
        key: "toolbar.save",
        valueType: "number",
      });
    }

    const badEnglishName = parseLanguagePack({
      formatVersion: 1,
      tag: "nl-NL",
      name: "X",
      messages: {},
      englishName: 42,
    });
    expect(badEnglishName.ok).toBe(false);
    if (!badEnglishName.ok) expect(badEnglishName.reason).toEqual({ kind: "englishNameNotString" });

    const badBasedOn = parseLanguagePack({
      formatVersion: 1,
      tag: "nl-NL",
      name: "X",
      messages: {},
      basedOn: 42,
    });
    expect(badBasedOn.ok).toBe(false);
    if (!badBasedOn.ok) expect(badBasedOn.reason).toEqual({ kind: "basedOnNotString" });

    const badPackVersion = parseLanguagePack({
      formatVersion: 1,
      tag: "nl-NL",
      name: "X",
      messages: {},
      packVersion: 42,
    });
    expect(badPackVersion.ok).toBe(false);
    if (!badPackVersion.ok) expect(badPackVersion.reason).toEqual({ kind: "packVersionNotString" });

    const badPluralCategories = parseLanguagePack({
      formatVersion: 1,
      tag: "nl-NL",
      name: "X",
      messages: {},
      pluralCategories: [1, 2],
    });
    expect(badPluralCategories.ok).toBe(false);
    if (!badPluralCategories.ok) {
      expect(badPluralCategories.reason).toEqual({ kind: "pluralCategoriesInvalid" });
    }
  });

  // Fix round 1 (Q2): `messages/en.ts`'s "languagePack.rejection.tagMalformed"
  // hard-codes the same sentence `rejectionReasonMessage` builds from
  // `BCP47_SHAPE_HINT` (both describe what a well-formed BCP 47 tag looks
  // like, for a user whose imported tag failed the check) — two sources
  // of one English truth, nothing pinning them together before this
  // test. It asserts the catalogue entry still contains the constant
  // verbatim, so an edit to one without the other fails here instead of
  // silently drifting apart in front of a user.
  it("the catalogue's tagMalformed hint stays in sync with BCP47_SHAPE_HINT", () => {
    expect(enMessages["languagePack.rejection.tagMalformed"]).toContain(BCP47_SHAPE_HINT);
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

// Final fix round: `persist()` (a bare `window.localStorage.setItem`)
// has no guaranteed success — `QuotaExceededError` is the realistic
// failure, since a pack accepts arbitrary user-supplied JSON of
// unbounded size and unknown fields are deliberately kept. Before this
// round, `importLanguagePack`/`removeLanguagePack` mutated the live
// cache *before* calling `persist()`, so a thrown `setItem` left the
// mutation standing: the exception escaped uncaught, and the very next
// unrelated `persist()` call (a later import or removal) would write the
// half-finished change to storage on the failed one's behalf. These
// tests throw from `setItem` for exactly one call to prove the mutation
// is rolled back instead.
describe("a persist() failure is rolled back, not smuggled in later", () => {
  function throwOnceFromSetItem() {
    return vi.spyOn(window.localStorage, "setItem").mockImplementationOnce(() => {
      throw new DOMException("The quota has been exceeded.", "QuotaExceededError");
    });
  }

  it("a fresh import whose persist() throws is rejected and never installed", () => {
    const spy = throwOnceFromSetItem();

    const result = importLanguagePack(dutchPack());
    expect(result.ok).toBe(false);
    if (!result.ok) {
      expect(result.error).toMatch(/storage/i);
      expect(result.reason.kind).toBe("storageFailure");
    }
    expect(listLanguagePacks()).toEqual([]);
    expect(getLanguagePack("nl-NL")).toBeUndefined();

    spy.mockRestore();

    // A later, unrelated successful import must not carry the rejected
    // pack into storage retroactively — it would if the cache mutation
    // from the failed import had never been rolled back.
    const second = importLanguagePack(dutchPack({ tag: "fr-FR", name: "Français" }));
    expect(second.ok).toBe(true);
    expect(listLanguagePacks().map((p) => p.tag)).toEqual(["fr-FR"]);

    const stored = (getSetting(LANGUAGE_PACKS_STORAGE_KEY) ?? {}) as Record<string, unknown>;
    expect(stored["nl-NL"]).toBeUndefined();
  });

  it("re-importing over an existing pack whose persist() throws leaves the old install unchanged", () => {
    importLanguagePack(dutchPack());
    const before = getLanguagePack("nl-NL");

    const spy = throwOnceFromSetItem();
    const result = importLanguagePack(dutchPack({ messages: { "toolbar.save": "Bewaren" } }));
    expect(result.ok).toBe(false);
    expect(getLanguagePack("nl-NL")).toEqual(before);

    spy.mockRestore();
  });

  it("a removal whose persist() throws restores the pack instead of leaving it half-deleted", () => {
    importLanguagePack(dutchPack());

    const spy = throwOnceFromSetItem();
    removeLanguagePack("nl-NL");
    expect(getLanguagePack("nl-NL")?.name).toBe("Nederlands");

    spy.mockRestore();

    // Same retroactive-smuggling check as the import case: an unrelated
    // successful persist() afterwards must not finish the removal that
    // failed.
    const second = importLanguagePack(dutchPack({ tag: "fr-FR", name: "Français" }));
    expect(second.ok).toBe(true);
    const stored = (getSetting(LANGUAGE_PACKS_STORAGE_KEY) ?? {}) as Record<string, unknown>;
    expect(stored["nl-NL"]).toBeDefined();
  });
});

// Fix round 1: the packs store gained its own subscriber set and
// `getSnapshot`, mirroring `uiLanguage.ts`'s shape, so that
// `i18n.ts`'s `useTranslate()` (and anything else) can react to an
// import or a removal without `SettingsPanel.tsx` having to rewrite an
// unrelated store (`setUiLanguage`) to fake the same effect.
describe("useLanguagePacks", () => {
  it("an already-mounted reader observes an import and a removal without remounting", async () => {
    function Reader() {
      const packs = useLanguagePacks();
      return <div data-testid="reader">{packs.map((p) => p.tag).join(",")}</div>;
    }

    const host = document.createElement("div");
    document.body.appendChild(host);
    const root = createRoot(host);
    try {
      await act(async () => {
        root.render(<Reader />);
      });
      expect(host.querySelector('[data-testid="reader"]')?.textContent).toBe("");

      await act(async () => {
        importLanguagePack(dutchPack());
      });
      expect(host.querySelector('[data-testid="reader"]')?.textContent).toBe("nl-NL");

      await act(async () => {
        removeLanguagePack("nl-NL");
      });
      expect(host.querySelector('[data-testid="reader"]')?.textContent).toBe("");
    } finally {
      root.unmount();
      host.remove();
    }
  });

  it("returns the same array reference across renders when nothing changed, so React never loops", async () => {
    const seenSnapshots: (readonly LanguagePack[])[] = [];

    function Reader() {
      const packs = useLanguagePacks();
      const [, forceRerender] = useState(0);
      seenSnapshots.push(packs);
      return (
        <button type="button" onClick={() => forceRerender((n) => n + 1)}>
          rerender
        </button>
      );
    }

    const host = document.createElement("div");
    document.body.appendChild(host);
    const root = createRoot(host);
    try {
      await act(async () => {
        root.render(<Reader />);
      });

      const button = host.querySelector("button")!;
      // A render triggered by something else entirely (local state, not
      // the packs store) must not see a new `packs` reference — a fresh
      // array on every `getSnapshot()` call is exactly what would trip
      // `useSyncExternalStore`'s "getSnapshot should be cached" loop
      // protection.
      await act(async () => {
        button.dispatchEvent(new MouseEvent("click", { bubbles: true, cancelable: true }));
      });

      expect(seenSnapshots.length).toBeGreaterThanOrEqual(2);
      expect(seenSnapshots[0]).toBe(seenSnapshots[seenSnapshots.length - 1]);
    } finally {
      root.unmount();
      host.remove();
    }
  });
});
