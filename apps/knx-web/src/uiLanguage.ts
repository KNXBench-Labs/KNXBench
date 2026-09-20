/** Detects, stores, and exposes the active UI language, built-in or an imported pack's tag. */
import { useEffect, useSyncExternalStore } from "react";
import { isWellFormedBcp47Tag } from "./languagePack";
import { settingsStorage, subscribeToSettings } from "./settingsStore";

/** The key inside the settings document the server keeps, not a
 * `localStorage` key — `settingsStorage` resolves it against the record. */
export const UI_LANGUAGE_STORAGE_KEY = "uiLanguage";

/**
 * The closed set of UI chrome languages `messages/*.ts` actually ships a
 * compile-time catalogue for. This one indexes `i18n.ts`'s `catalogs`
 * record directly, so it has to stay a compile-time-known union, not an
 * arbitrary string — that safety over the two shipped catalogues is
 * exactly what `UiLanguage` below does not give up when it opens the
 * *active* language to more than these two.
 */
export const AVAILABLE_UI_LANGUAGES = ["en", "de"] as const;
export type BuiltInUiLanguage = (typeof AVAILABLE_UI_LANGUAGES)[number];

/**
 * The *active* UI language: a built-in id (`"en"`/`"de"`), or the `tag`
 * of a pack installed through `languagePack.ts` — Dutch, Klingon,
 * Bavarian, Sindarin, whatever a user imported (T25 task 6). Open by
 * design: unlike `BuiltInUiLanguage`, membership here isn't
 * compile-time-checked, because an imported pack's tag is only known at
 * runtime. `detectUiLanguage` below still only ever returns a
 * `BuiltInUiLanguage` — pack tags are never guessed at, only chosen.
 */
export type UiLanguage = string;

const DEFAULT_UI_LANGUAGE: BuiltInUiLanguage = "en";

function isBuiltInUiLanguage(value: string): value is BuiltInUiLanguage {
  return (AVAILABLE_UI_LANGUAGES as readonly string[]).includes(value);
}

/**
 * Matches `navigator.language` (e.g. `"de-AT"`, `"fr-FR"`) against the
 * available catalogues by primary subtag — `"de-AT"` becomes `"de"` — and
 * falls back to the default when the primary subtag has no catalogue of
 * its own (`"fr-FR"` has no French catalogue, so it becomes `"en"`).
 */
export function detectUiLanguage(nav: Pick<Navigator, "language"> | undefined): BuiltInUiLanguage {
  const raw = nav?.language;
  if (!raw) return DEFAULT_UI_LANGUAGE;
  const primary = raw.split("-")[0]?.toLowerCase() ?? "";
  return isBuiltInUiLanguage(primary) ? primary : DEFAULT_UI_LANGUAGE;
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
    cached = loadUiLanguage(settingsStorage, window.navigator);
  }
  return cached;
}

function subscribe(onStoreChange: () => void): () => void {
  subscribers.add(onStoreChange);
  return () => subscribers.delete(onStoreChange);
}

function setStoredUiLanguage(language: UiLanguage): void {
  saveUiLanguage(settingsStorage, language);
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
 * is stored. An explicit stored choice always wins over detection — not
 * just a built-in id, but *any well-formed BCP 47 tag*, because the
 * stored value may name an installed language pack rather than a
 * compiled-in catalogue. This is why a pack survives a reload: nothing
 * here checks whether a pack for that tag still exists (that's a render
 * concern — see `i18n.ts`'s `translateFor`, which falls back to English
 * for a tag naming a pack that's gone) — only whether the stored value is
 * *shaped* like a language tag at all. Only an empty/malformed stored
 * value falls through to `detectUiLanguage`.
 */
export function loadUiLanguage(
  storage: Pick<Storage, "getItem">,
  nav?: Pick<Navigator, "language">,
): UiLanguage {
  const raw = storage.getItem(UI_LANGUAGE_STORAGE_KEY);
  if (raw && isWellFormedBcp47Tag(raw)) return raw;
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

// The record can change under this cache — the settings file arriving from
// the server after first paint, or another preference module's write
// rewriting the document. Dropping the cache and telling subscribers is
// enough: `getSnapshot()` re-reads on demand, and a re-read that finds the
// same language hands back the same string, so `useSyncExternalStore` sees
// no change and nothing re-renders.
subscribeToSettings(() => {
  cached = undefined;
  for (const onStoreChange of subscribers) onStoreChange();
});
