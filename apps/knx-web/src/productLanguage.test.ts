import { describe, expect, it, vi } from "vitest";
import { PRODUCT_LANGUAGE_STORAGE_KEY, loadProductLanguage, saveProductLanguage } from "./productLanguage";

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
