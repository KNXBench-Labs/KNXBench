/** Tests for UI-language detection, storage, and the useUiLanguage/getActiveUiLanguage hooks. */
// @vitest-environment happy-dom
//
// happy-dom, not node: `useUiLanguage()`'s module-level store lazily reads
// `window.localStorage`/`window.navigator` on its first call in a given
// test run (same reasoning as `productLanguage.test.tsx`), and the
// hook-broadcast test below renders real components through
// `react-dom/client`'s `createRoot`.
import { act } from "react";
import { createRoot } from "react-dom/client";
import { afterEach, describe, expect, it, vi } from "vitest";
import {
  UI_LANGUAGE_STORAGE_KEY,
  detectUiLanguage,
  getActiveUiLanguage,
  loadUiLanguage,
  resetUiLanguageForTests,
  saveUiLanguage,
  useUiLanguage,
} from "./uiLanguage";

afterEach(() => {
  window.localStorage.removeItem(UI_LANGUAGE_STORAGE_KEY);
  document.documentElement.removeAttribute("lang");
  resetUiLanguageForTests();
});

function fakeStorage(initial: Record<string, string> = {}) {
  const store = { ...initial };
  return {
    getItem: vi.fn((key: string) => store[key] ?? null),
    setItem: vi.fn((key: string, value: string) => {
      store[key] = value;
    }),
  };
}

describe("detectUiLanguage", () => {
  it("matches a region subtag to its language: de-AT -> de", () => {
    expect(detectUiLanguage({ language: "de-AT" })).toBe("de");
  });

  it("falls back to en for a language with no catalogue: fr-FR -> en", () => {
    expect(detectUiLanguage({ language: "fr-FR" })).toBe("en");
  });

  it("falls back to en when there is nothing to detect from", () => {
    expect(detectUiLanguage(undefined)).toBe("en");
  });
});

describe("loadUiLanguage", () => {
  it("detects from navigator when nothing is stored", () => {
    expect(loadUiLanguage(fakeStorage(), { language: "de-CH" })).toBe("de");
  });

  it("an explicit stored choice beats detection", () => {
    const storage = fakeStorage({ [UI_LANGUAGE_STORAGE_KEY]: "en" });
    expect(loadUiLanguage(storage, { language: "de-DE" })).toBe("en");
  });

  it("an unknown stored value falls through to detection", () => {
    const storage = fakeStorage({ [UI_LANGUAGE_STORAGE_KEY]: "xx-not-a-language" });
    expect(loadUiLanguage(storage, { language: "de-AT" })).toBe("de");
  });
});

describe("saveUiLanguage", () => {
  it("a saved language round-trips", () => {
    const storage = fakeStorage();
    saveUiLanguage(storage, "de");
    expect(loadUiLanguage(storage)).toBe("de");
  });
});

describe("useUiLanguage / getActiveUiLanguage", () => {
  // Regression coverage for the shared-store shape, mirroring
  // `productLanguage.test.tsx`'s writer/reader test: a plain per-component
  // `useState` would let a `Reader` and a `Writer` drift apart, and
  // `getActiveUiLanguage()` (the non-hook accessor for module-level code
  // such as `commandRegistry.ts`) needs to observe the same broadcast.
  it("a writer's change reaches an already-mounted reader and the non-hook accessor", async () => {
    function Reader() {
      const [language] = useUiLanguage();
      return <div data-testid="reader">{language}</div>;
    }

    function Writer() {
      const [, setLanguage] = useUiLanguage();
      return (
        <button type="button" onClick={() => setLanguage("de")}>
          set de
        </button>
      );
    }

    const host = document.createElement("div");
    document.body.appendChild(host);
    const root = createRoot(host);
    try {
      await act(async () => {
        root.render(
          <>
            <Reader />
            <Writer />
          </>,
        );
      });

      expect(host.querySelector('[data-testid="reader"]')?.textContent).toBe("en");
      expect(getActiveUiLanguage()).toBe("en");

      const button = host.querySelector("button");
      await act(async () => {
        button?.dispatchEvent(new MouseEvent("click", { bubbles: true, cancelable: true }));
      });

      expect(host.querySelector('[data-testid="reader"]')?.textContent).toBe("de");
      expect(getActiveUiLanguage()).toBe("de");
    } finally {
      root.unmount();
      host.remove();
    }
  });

  it("sets document.documentElement's lang attribute to the active language", async () => {
    function Component() {
      const [, setLanguage] = useUiLanguage();
      return (
        <button type="button" onClick={() => setLanguage("de")}>
          set de
        </button>
      );
    }

    const host = document.createElement("div");
    document.body.appendChild(host);
    const root = createRoot(host);
    try {
      await act(async () => {
        root.render(<Component />);
      });

      expect(document.documentElement.getAttribute("lang")).toBe("en");

      const button = host.querySelector("button");
      await act(async () => {
        button?.dispatchEvent(new MouseEvent("click", { bubbles: true, cancelable: true }));
      });

      expect(document.documentElement.getAttribute("lang")).toBe("de");
    } finally {
      root.unmount();
      host.remove();
    }
  });
});
