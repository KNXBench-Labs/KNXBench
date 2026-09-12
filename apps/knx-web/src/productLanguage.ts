import { useEffect, useState } from "react";

export const PRODUCT_LANGUAGE_STORAGE_KEY = "knx-desktop:product-language";

/**
 * Reads the persisted product-data language, or `null` for "package
 * default" — the untranslated text the installed product package ships
 * with, and what everyone who has never opened Settings gets.
 *
 * Unlike `theme.ts`'s `loadThemeId`, there is no compile-time list of
 * valid ids to fall back from: which languages exist depends entirely on
 * which product packages happen to be installed, so whatever was last
 * saved is trusted and sent to the server as-is. If the product database
 * no longer has that language (a package was reinstalled, or never had
 * it), the server simply returns untranslated text for the strings it
 * lacks a translation row for — per string, not as an all-or-nothing
 * failure — so there is nothing here to validate against.
 */
export function loadProductLanguage(storage: Pick<Storage, "getItem">): string | null {
  return storage.getItem(PRODUCT_LANGUAGE_STORAGE_KEY);
}

/**
 * Persists the product-data language. `null` ("package default") removes
 * the key rather than writing the string `"null"` — otherwise a later
 * `loadProductLanguage` would read back the four-character string
 * `"null"` and send `?language=null` to the server instead of omitting
 * the parameter.
 */
export function saveProductLanguage(
  storage: Pick<Storage, "setItem" | "removeItem">,
  language: string | null,
): void {
  if (language === null) {
    storage.removeItem(PRODUCT_LANGUAGE_STORAGE_KEY);
  } else {
    storage.setItem(PRODUCT_LANGUAGE_STORAGE_KEY, language);
  }
}

/**
 * Reads the persisted product-data language on mount and persists on
 * every change, like `useThemeId`. Unlike `useThemeId`, it sets no
 * `document` attribute — a product-data language selects server-side
 * translation text, not anything CSS needs to react to.
 */
export function useProductLanguage(): [string | null, (language: string | null) => void] {
  const [language, setLanguage] = useState<string | null>(() => loadProductLanguage(window.localStorage));

  useEffect(() => {
    saveProductLanguage(window.localStorage, language);
  }, [language]);

  return [language, setLanguage];
}
