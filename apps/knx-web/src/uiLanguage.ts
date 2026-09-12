import { useEffect, useSyncExternalStore } from "react";

export const UI_LANGUAGE_STORAGE_KEY = "knx-desktop:ui-language";

/**
 * The closed set of UI chrome languages `messages/*.ts` actually ships a
 * catalogue for. Unlike `productLanguage.ts`'s language string — an
 * open-ended value naming whatever product package happens to be
 * installed — this one indexes `i18n.ts`'s `catalogs` record directly, so
 * it has to stay a compile-time-known union, not an arbitrary string.
 */
export const AVAILABLE_UI_LANGUAGES = ["en", "de"] as const;
export type UiLanguage = (typeof AVAILABLE_UI_LANGUAGES)[number];

const DEFAULT_UI_LANGUAGE: UiLanguage = "en";

function isUiLanguage(value: string): value is UiLanguage {
  return (AVAILABLE_UI_LANGUAGES as readonly string[]).includes(value);
}

/**
 * Matches `navigator.language` (e.g. `"de-AT"`, `"fr-FR"`) against the
 * available catalogues by primary subtag — `"de-AT"` becomes `"de"` — and
 * falls back to the default when the primary subtag has no catalogue of
 * its own (`"fr-FR"` has no French catalogue, so it becomes `"en"`).
 */
export function detectUiLanguage(nav: Pick<Navigator, "language"> | undefined): UiLanguage {
  const raw = nav?.language;
  if (!raw) return DEFAULT_UI_LANGUAGE;
  const primary = raw.split("-")[0]?.toLowerCase() ?? "";
  return isUiLanguage(primary) ? primary : DEFAULT_UI_LANGUAGE;
}

// The store behind `useUiLanguage()`, structured exactly like
// `productLanguage.ts`'s store: a module-level cache broadcast to every
// subscriber through `useSyncExternalStore`, rather than a per-component
// `useState` that would let two mounted call sites drift apart. `undefined`
// means "not yet seeded from `localStorage`", distinct from any real
// `UiLanguage` value. Seeding is lazy — on first `getSnapshot()`, not at
// module-evaluation time — because `happy-dom` only wires up
// `window.localStorage` (and a usable `window.navigator`) per test file,
// and module evaluation can happen before that.
let cached: UiLanguage | undefined;
const subscribers = new Set<() => void>();

function getSnapshot(): UiLanguage {
  if (cached === undefined) {
    cached = loadUiLanguage(window.localStorage, window.navigator);
  }
  return cached;
}

function subscribe(onStoreChange: () => void): () => void {
  subscribers.add(onStoreChange);
  return () => subscribers.delete(onStoreChange);
}

function setStoredUiLanguage(language: UiLanguage): void {
  saveUiLanguage(window.localStorage, language);
  cached = language;
  for (const onStoreChange of subscribers) onStoreChange();
}

/**
 * Resets the module-level cache seeded by `getSnapshot()`, the same
 * reason `productLanguage.ts` needs `resetProductLanguageForTests()`:
 * clearing `localStorage` in `afterEach` doesn't un-seed `cached`, so a
 * later test in the same file would still observe whatever an earlier
 * test last set.
 */
export function resetUiLanguageForTests(): void {
  cached = undefined;
}

/**
 * Reads the persisted UI language, or detects one from `nav` when nothing
 * is stored. An explicit stored choice always wins over detection — only
 * an empty/unknown stored value falls through to `detectUiLanguage`.
 */
export function loadUiLanguage(
  storage: Pick<Storage, "getItem">,
  nav?: Pick<Navigator, "language">,
): UiLanguage {
  const raw = storage.getItem(UI_LANGUAGE_STORAGE_KEY);
  if (raw && isUiLanguage(raw)) return raw;
  return detectUiLanguage(nav);
}

export function saveUiLanguage(storage: Pick<Storage, "setItem">, language: UiLanguage): void {
  storage.setItem(UI_LANGUAGE_STORAGE_KEY, language);
}

/**
 * Reads the persisted UI language through a shared, module-level store and
 * persists on every change, the same pattern as `useProductLanguage()`:
 * setting it in one component is observed immediately by every other
 * mounted component that reads it, with no remount required.
 *
 * Also applies `language` to `<html lang>`, the same
 * `document.documentElement`-side-effect-lives-inside-the-hook convention
 * `useThemeId`'s `data-theme` and `useMotion`'s `data-motion-*` use — so
 * the browser and assistive technology agree with whatever `messages/*.ts`
 * catalogue is currently on screen, without a separate effect anywhere
 * else. Unlike those two, this hook has no local `useState` of its own to
 * hang the effect on (the value comes from `useSyncExternalStore`
 * instead), but the placement rule — react to this hook's own current
 * value, right here — is the same one.
 */
export function useUiLanguage(): [UiLanguage, (language: UiLanguage) => void] {
  const language = useSyncExternalStore(subscribe, getSnapshot);

  useEffect(() => {
    document.documentElement.setAttribute("lang", language);
  }, [language]);

  return [language, setStoredUiLanguage];
}

/**
 * Reads the current UI language outside of React — for module-level code
 * such as `commandRegistry.ts` that builds data at import time or inside a
 * plain function, not a component render. Goes through the same lazy-seeded
 * cache as `useUiLanguage()`, so it agrees with whatever the last render
 * observed; it just doesn't subscribe to future changes.
 */
export function getActiveUiLanguage(): UiLanguage {
  return getSnapshot();
}
