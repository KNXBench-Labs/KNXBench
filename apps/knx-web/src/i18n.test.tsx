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

afterEach(() => {
  window.localStorage.removeItem(UI_LANGUAGE_STORAGE_KEY);
  resetUiLanguageForTests();
});

describe("translateFor", () => {
  it("looks up the active language's string", () => {
    expect(translateFor("en", "toolbar.save")).toBe("Save");
    expect(translateFor("de", "toolbar.save")).toBe("Speichern");
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
