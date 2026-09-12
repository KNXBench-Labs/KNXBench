// @vitest-environment happy-dom
//
// happy-dom, not node: `useTranslate()`'s hook test below renders a real
// component through `react-dom/client`'s `createRoot`, and reads/writes
// the UI language through `useUiLanguage()`'s `window.localStorage`-backed
// store.
import { act } from "react";
import { createRoot } from "react-dom/client";
import { afterEach, describe, expect, it } from "vitest";
import { formatTemplate, resolveFromCatalog, translateFor, useTranslate } from "./i18n";
import { UI_LANGUAGE_STORAGE_KEY, resetUiLanguageForTests, useUiLanguage } from "./uiLanguage";
import {
  LANGUAGE_PACKS_STORAGE_KEY,
  importLanguagePack,
  resetLanguagePacksForTests,
} from "./languagePack";

afterEach(() => {
  window.localStorage.removeItem(UI_LANGUAGE_STORAGE_KEY);
  window.localStorage.removeItem(LANGUAGE_PACKS_STORAGE_KEY);
  resetUiLanguageForTests();
  resetLanguagePacksForTests();
});

describe("translateFor", () => {
  it("looks up the active language's string", () => {
    expect(translateFor("en", "toolbar.save")).toBe("Save");
    expect(translateFor("de", "toolbar.save")).toBe("Speichern");
  });

  it("resolves an installed pack's translated key", () => {
    importLanguagePack({
      formatVersion: 1,
      tag: "nl-NL",
      name: "Nederlands",
      messages: { "toolbar.save": "Opslaan" },
    });
    expect(translateFor("nl-NL", "toolbar.save")).toBe("Opslaan");
  });

  it("falls back to English for a key the pack omits — never to German, even when basedOn says de", () => {
    // Bavarian, plausibly translated from German (basedOn: "de"), but
    // missing toolbar.save entirely. The rule is not negotiable: the
    // fallback chain is [active language, English], full stop.
    importLanguagePack({
      formatVersion: 1,
      tag: "bar",
      name: "Boarisch",
      basedOn: "de",
      messages: { "toolbar.saveAs": "Speichan untern..." },
    });
    expect(translateFor("bar", "toolbar.save")).toBe("Save");
    expect(translateFor("bar", "toolbar.save")).not.toBe("Speichern");
  });

  it("an active tag naming a pack that was never installed resolves to English, not an error", () => {
    expect(translateFor("xx-nonexistent-pack", "toolbar.save")).toBe("Save");
  });

  it("an active tag naming a pack that was removed resolves to English", () => {
    importLanguagePack({
      formatVersion: 1,
      tag: "nl-NL",
      name: "Nederlands",
      messages: { "toolbar.save": "Opslaan" },
    });
    expect(translateFor("nl-NL", "toolbar.save")).toBe("Opslaan");

    resetLanguagePacksForTests();
    window.localStorage.removeItem(LANGUAGE_PACKS_STORAGE_KEY);
    expect(translateFor("nl-NL", "toolbar.save")).toBe("Save");
  });

  it("a fantasy tag with no native plural data still resolves and degrades plural selection to 'other'", () => {
    importLanguagePack({
      formatVersion: 1,
      tag: "art-x-sindarin",
      name: "Eledhrim",
      messages: {
        "inspector.deviceCount.one": "{count} vân (one)",
        "inspector.deviceCount.other": "{count} vain (other)",
      },
    });
    // Never throws, and count === 1 — which real plural data (e.g.
    // English's) would route to the .one category — resolves to .other
    // instead, since Intl.PluralRules has no data for this tag at all.
    expect(translateFor("art-x-sindarin", "inspector.deviceCount", { count: 1 })).toBe(
      "1 vain (other)",
    );
    expect(translateFor("art-x-sindarin", "inspector.deviceCount", { count: 5 })).toBe(
      "5 vain (other)",
    );
  });
});

describe("formatTemplate", () => {
  it("substitutes {name}-style placeholders", () => {
    expect(formatTemplate("Hello {name}!", { name: "Marvin" })).toBe("Hello Marvin!");
  });

  it("substitutes a numeric param by its string form", () => {
    expect(formatTemplate("{count} items", { count: 3 })).toBe("3 items");
  });

  it("leaves a placeholder with no matching param untouched", () => {
    expect(formatTemplate("Hello {name}!", {})).toBe("Hello {name}!");
  });

  it("passes a template through unchanged when there are no params", () => {
    expect(formatTemplate("Save")).toBe("Save");
  });
});

describe("resolveFromCatalog (plural branches)", () => {
  const catalog = {
    "device.count.one": "{count} device",
    "device.count.other": "{count} devices",
  };

  it("selects the singular category", () => {
    const template = resolveFromCatalog(catalog, "device.count", { count: 1 }, "en");
    expect(template && formatTemplate(template, { count: 1 })).toBe("1 device");
  });

  it("selects the plural category", () => {
    const template = resolveFromCatalog(catalog, "device.count", { count: 5 }, "en");
    expect(template && formatTemplate(template, { count: 5 })).toBe("5 devices");
  });

  it("defaults count to 0 when params omit it, which English/German both call 'other'", () => {
    const template = resolveFromCatalog(catalog, "device.count", undefined, "en");
    expect(template).toBe("{count} devices");
  });

  it("returns undefined for a key the catalogue has neither directly nor as a plural pair", () => {
    expect(resolveFromCatalog(catalog, "nope", undefined, "en")).toBeUndefined();
  });
});

describe("useTranslate", () => {
  it("returns the active language's t(), switching when the language store changes", async () => {
    function Harness() {
      const t = useTranslate();
      const [, setLanguage] = useUiLanguage();
      return (
        <>
          <div data-testid="label">{t("toolbar.save")}</div>
          <button type="button" onClick={() => setLanguage("de")}>
            switch
          </button>
        </>
      );
    }

    const host = document.createElement("div");
    document.body.appendChild(host);
    const root = createRoot(host);
    try {
      await act(async () => {
        root.render(<Harness />);
      });

      expect(host.querySelector('[data-testid="label"]')?.textContent).toBe("Save");

      const button = host.querySelector("button");
      await act(async () => {
        button?.dispatchEvent(new MouseEvent("click", { bubbles: true, cancelable: true }));
      });

      expect(host.querySelector('[data-testid="label"]')?.textContent).toBe("Speichern");
    } finally {
      root.unmount();
      host.remove();
    }
  });
});
