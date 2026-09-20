import { useSyncExternalStore } from "react";
import { settingsStorage, subscribeToSettings } from "./settingsStore";

/** The key inside the settings document the server keeps, not a
 * `localStorage` key — `settingsStorage` resolves it against the record. */
export const PRODUCT_LANGUAGE_STORAGE_KEY = "productLanguage";

// The store behind `useProductLanguage()`. A product-data language is read
// by two independent call sites (`App.tsx`'s Settings select and
// `ParameterPanel.tsx`) and `Inspector.tsx` never remounts the panel on a
// device switch, so two separate `useState`s (the original design) go out
// of sync the moment one of them changes — Settings would update, the open
// panel wouldn't. `useSyncExternalStore` gives every caller the same
// module-level value instead, the same way `document.documentElement`'s
// `data-theme`/`data-motion-*` attributes broadcast `useThemeId`/
// `useMotion`'s state to CSS — except a product-data language isn't a CSS
// concern, so the broadcast happens through this module's own subscriber
// set rather than the DOM.
//
// `undefined` means "not yet seeded from `localStorage`" — distinct from
// `null` ("package default"), which is a legitimate seeded value. Seeding
// happens lazily, on first `getSnapshot()` call, not at module-evaluation
// time: `happy-dom` only sets up `window.localStorage` per test file, and
// module evaluation can happen before that.
let cached: string | null | undefined;
const subscribers = new Set<() => void>();

function getSnapshot(): string | null {
  if (cached === undefined) {
    cached = loadProductLanguage(settingsStorage);
  }
  return cached;
}

function subscribe(onStoreChange: () => void): () => void {
  subscribers.add(onStoreChange);
  return () => subscribers.delete(onStoreChange);
}

function setStoredProductLanguage(language: string | null): void {
  saveProductLanguage(settingsStorage, language);
  cached = language;
  for (const onStoreChange of subscribers) onStoreChange();
}

/**
 * Resets the module-level cache seeded by `getSnapshot()`. Without this,
 * module state leaks between tests in the same file: clearing
 * `localStorage` in `afterEach` doesn't un-seed `cached`, so a later test
 * that renders `useProductLanguage()` would still observe whatever an
 * earlier test last set, regardless of what's actually in storage.
 */
export function resetProductLanguageForTests(): void {
  cached = undefined;
}

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
 * Reads the persisted product-data language through a shared, module-level
 * store and persists on every change, like `useThemeId`. Unlike
 * `useThemeId`, it sets no `document` attribute — a product-data language
 * selects server-side translation text, not anything CSS needs to react
 * to — so instead of the DOM, `subscribe`/`getSnapshot` broadcast the
 * value to every call site: setting it in one component (Settings) is
 * observed immediately by every other mounted component that reads it
 * (the open `ParameterPanel`), with no remount required.
 */
export function useProductLanguage(): [string | null, (language: string | null) => void] {
  const language = useSyncExternalStore(subscribe, getSnapshot);
  return [language, setStoredProductLanguage];
}

// Same invalidation as `uiLanguage.ts`: the record can change under this
// cache, and re-reading on demand is cheaper and less wrong than trying to
// predict which write touched which key.
subscribeToSettings(() => {
  cached = undefined;
  for (const onStoreChange of subscribers) onStoreChange();
});
