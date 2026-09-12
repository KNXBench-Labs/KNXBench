// @vitest-environment happy-dom
//
// happy-dom, not node: the two unit-test `describe` blocks below only ever
// touch an injected fake `storage`, but the third renders real components
// through `react-dom/client`'s `createRoot`, which needs a `window`/`document`
// (and `useProductLanguage()`'s module-level store lazily reads
// `window.localStorage` on its first call in a given test run).
import { act, useEffect } from "react";
import { createRoot } from "react-dom/client";
import { afterEach, describe, expect, it, vi } from "vitest";
import {
  PRODUCT_LANGUAGE_STORAGE_KEY,
  loadProductLanguage,
  resetProductLanguageForTests,
  saveProductLanguage,
  useProductLanguage,
} from "./productLanguage";

afterEach(() => {
  window.localStorage.removeItem(PRODUCT_LANGUAGE_STORAGE_KEY);
  resetProductLanguageForTests();
});

function fakeStorage(initial: Record<string, string> = {}) {
  const store = { ...initial };
  return {
    getItem: vi.fn((key: string) => store[key] ?? null),
    setItem: vi.fn((key: string, value: string) => {
      store[key] = value;
    }),
    removeItem: vi.fn((key: string) => {
      delete store[key];
    }),
  };
}

describe("loadProductLanguage", () => {
  it("the default is null when nothing is stored", () => {
    expect(loadProductLanguage(fakeStorage())).toBeNull();
  });
});

describe("saveProductLanguage", () => {
  it("a saved language round-trips", () => {
    const storage = fakeStorage();
    saveProductLanguage(storage, "de-DE");
    expect(loadProductLanguage(storage)).toBe("de-DE");
  });

  it('saving null removes the key rather than storing the string "null"', () => {
    const storage = fakeStorage({ [PRODUCT_LANGUAGE_STORAGE_KEY]: "de-DE" });
    saveProductLanguage(storage, null);
    expect(storage.removeItem).toHaveBeenCalledWith(PRODUCT_LANGUAGE_STORAGE_KEY);
    expect(loadProductLanguage(storage)).toBeNull();
  });
});

describe("useProductLanguage", () => {
  // Regression test for the whole-branch review's blocker: `App.tsx`'s
  // Settings select and `ParameterPanel.tsx` each call `useProductLanguage()`
  // independently, and `Inspector.tsx` renders the panel with no `key`, so it
  // is never remounted on a device switch. Two plain, unconnected
  // `useState`s (the original implementation) let Settings's write vanish
  // into its own copy — the open panel's copy never moves. Mounting a
  // "reader" and a "writer" side by side and changing the language only
  // through the writer is the smallest reproduction of that seam; asserting
  // the reader's own mount effect fired exactly once additionally rules out
  // "it worked because Inspector happened to remount", which isn't how the
  // real bug manifested.
  it("a writer's change reaches an already-mounted reader without remounting it", async () => {
    let readerMounts = 0;

    function Reader() {
      const [language] = useProductLanguage();
      useEffect(() => {
        readerMounts += 1;
      }, []);
      return <div data-testid="reader">{language ?? "(default)"}</div>;
    }

    function Writer() {
      const [, setLanguage] = useProductLanguage();
      return (
        <button type="button" onClick={() => setLanguage("de-DE")}>
          set de-DE
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

      expect(host.querySelector('[data-testid="reader"]')?.textContent).toBe("(default)");

      const button = host.querySelector("button");
      await act(async () => {
        button?.dispatchEvent(new MouseEvent("click", { bubbles: true, cancelable: true }));
      });

      expect(host.querySelector('[data-testid="reader"]')?.textContent).toBe("de-DE");
      expect(readerMounts).toBe(1);
    } finally {
      root.unmount();
      host.remove();
    }
  });
});
