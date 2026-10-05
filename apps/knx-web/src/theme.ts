/** The registry of selectable themes, and where the choice is persisted and read back. */
// SPDX-License-Identifier: AGPL-3.0-or-later
import { useEffect } from "react";
import { getSetting, settingsStorage, useSettingsRevision } from "./settingsStore";
import { ACCENTS, loadAppearance, type Accent } from "./appearance";
import { readThemePackStore } from "./themePack";
import type { ThemePack, ThemePackStore } from "./themePack";
import { applyThemePack } from "./themePackDom";
import { BUNDLED_THEME_PACKS } from "./bundledThemes";

/** `hasAccentVariations` is whether styles.css declares any
 * `[data-accent="…"]` variation for this theme (ADR-0022: Cupertino
 * treats the accent as identity and declares none). `themeTokens.test.ts`
 * checks this flag against the stylesheet so it cannot go stale. */
export interface ThemeDef { id: string; name: string; hasAccentVariations: boolean; }

/** The palettes: one `:root[data-theme="<id>"]` block each in styles.css.
 * Neon Grid and Bitcoin DeFi were removed on the user's decision of
 * 2026-10-05 (ADR-0079). A saved choice of either is kept as written and
 * reported as an unavailable selection, like any other unknown id. */
const PALETTE_THEMES: readonly ThemeDef[] = [
  { id: "porcelain", name: "Porcelain", hasAccentVariations: true },
  { id: "graphite", name: "Graphite", hasAccentVariations: true },
  { id: "cupertino", name: "Cupertino", hasAccentVariations: false },
];

/**
 * `system` is not a palette — it is whichever palette `resolveThemeId`
 * picks from the OS preference, and that can flip under the user mid-session.
 * So its accent flag is *derived*: the accent control is offered under
 * `system` only when every palette it can resolve to varies by accent.
 * Hard-coding it was the one registry entry nothing checked; a hand-typed
 * `false` there disabled the accent control for the default theme and the
 * whole suite stayed green.
 *
 * `every` rather than `some` is the conservative reading — offer the
 * control only when it works in both directions — and it is deliberately
 * unpinned: both palettes `system` resolves to happen to vary by accent
 * today, so swapping the two leaves every test passing. The day a
 * light/dark pair disagrees, this line decides something, and the test
 * that should have been written first will have to be written then.
 */
const systemHasAccentVariations = [resolveThemeId("system", false), resolveThemeId("system", true)]
  .map((id) => PALETTE_THEMES.find((theme) => theme.id === id))
  .every((theme) => theme?.hasAccentVariations === true);

export const THEMES: readonly ThemeDef[] = [
  { id: "system", name: "System", hasAccentVariations: systemHasAccentVariations },
  ...PALETTE_THEMES,
];
/**
 * The key inside the settings document (`settings.json`), not a
 * `localStorage` key: `settingsStorage` resolves it against the record
 * the server keeps. The migration of the pre-file `"light"`/`"dark"` ids
 * below still matters — the server's v0 step normalizes what it adopts,
 * but a hand-written file can still say either.
 */
const STORAGE_KEY = "theme";

/** Preserve explicit old preferences; unknown preferences follow the OS. */
export function loadThemeId(storage: Pick<Storage, "getItem">, themes: readonly ThemeDef[] = THEMES): string {
  let raw: string | null = null;
  try { raw = storage.getItem(STORAGE_KEY); } catch { /* Storage can be unavailable. */ }
  if (raw === "light") return "porcelain";
  if (raw === "dark") return "graphite";
  return themes.some((theme) => theme.id === raw) ? raw! : "system";
}
/** The pack behind a theme id: an installed pack wins over a shipped one
 * with the same id — it is the user's own data (ADR-0079). */
export function findThemePack(store: ThemePackStore, id: string): ThemePack | undefined {
  return store.packs.find((pack) => pack.id === id) ?? BUNDLED_THEME_PACKS.find((pack) => pack.id === id);
}
/** Whether a theme id is a shipped pack that no installed pack replaces. */
export function isBundledThemeId(store: ThemePackStore, id: string): boolean {
  return !store.packs.some((pack) => pack.id === id) && BUNDLED_THEME_PACKS.some((pack) => pack.id === id);
}
/** Keep the built-in registry fixed; derive pack choices from validated data. */
export function getThemeDefinitions(store: ThemePackStore = readThemePackStore(getSetting("uiThemePacks"))): readonly ThemeDef[] {
  const bundled = BUNDLED_THEME_PACKS.filter((pack) => isBundledThemeId(store, pack.id));
  return [...THEMES, ...[...bundled, ...store.packs].map((pack) => ({ id: pack.id, name: pack.name,
    hasAccentVariations: Object.keys(pack.accents ?? {}).length > 0 }))];
}
export interface ThemeSelection {
  id: string;
  pack?: ThemePack;
  diagnostics: ThemePackStore["diagnostics"];
}
/** Presentation fallback never overwrites an unknown stored identity or pack. */
export function readThemeSelection(storage: Pick<Storage, "getItem">, rawPacks: unknown): ThemeSelection {
  const store = readThemePackStore(rawPacks), themes = getThemeDefinitions(store);
  let raw: string | null = null;
  try { raw = storage.getItem(STORAGE_KEY); } catch { /* Unavailable preference follows System. */ }
  const id = loadThemeId({ getItem: () => raw }, themes);
  const diagnostics = [...store.diagnostics];
  if (raw !== null && raw !== "light" && raw !== "dark" && !themes.some((theme) => theme.id === raw)) {
    diagnostics.push({ id: null, diagnostic: { kind: "missingSelection", path: "$.theme" } });
  }
  return { id, pack: findThemePack(store, id), diagnostics };
}
export function resolveThemeId(id: string, dark: boolean): string {
  return id === "system" ? (dark ? "graphite" : "porcelain") : id;
}
export function saveThemeId(storage: Pick<Storage, "setItem">, id: string): void {
  try { storage.setItem(STORAGE_KEY, id); } catch { /* Session preference still works. */ }
}
/** Advertise only the accent variations the selected theme can apply. */
export function getThemeAccentOptions(id: string): readonly Accent[] {
  const builtin = THEMES.find((theme) => theme.id === id);
  if (builtin) return builtin.hasAccentVariations ? ACCENTS : [];
  const pack = findThemePack(readThemePackStore(getSetting("uiThemePacks")), id);
  return ACCENTS.filter((accent) => Object.hasOwn(pack?.accents ?? {}, accent));
}
/** A theme consumer must not acquire another root DOM lease. */
export function useSavedThemeId(): string {
  useSettingsRevision();
  return readThemeSelection(settingsStorage, getSetting("uiThemePacks")).id;
}
export function useThemeId(): [string, (id: string) => void] {
  const revision = useSettingsRevision();
  const { id, pack } = readThemeSelection(settingsStorage, getSetting("uiThemePacks"));
  useEffect(() => {
    const query = window.matchMedia("(prefers-color-scheme: dark)");
    const root = document.documentElement;
    const application = pack ? applyThemePack(root, pack, loadAppearance(settingsStorage).accent) : undefined;
    const appliedId = application && !application.ok ? "system" : id;
    const apply = () => { root.dataset.theme = resolveThemeId(appliedId, query.matches); };
    apply();
    query.addEventListener("change", apply);
    return () => {
      query.removeEventListener("change", apply);
      if (application?.ok) application.release();
    };
  }, [id, revision]);
  return [id, (next) => {
    if (getThemeDefinitions().some((theme) => theme.id === next)) saveThemeId(settingsStorage, next);
  }];
}
