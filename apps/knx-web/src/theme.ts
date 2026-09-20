/** The registry of selectable themes, and where the choice is persisted and read back. */
import { useEffect, useState } from "react";
import { settingsStorage, useSettingsRevision } from "./settingsStore";

/** `hasAccentVariations` is whether styles.css declares any
 * `[data-accent="…"]` variation for this theme (ADR-0022: three of five
 * treat the accent as identity and declare none). `themeTokens.test.ts`
 * checks this flag against the stylesheet so it cannot go stale. */
export interface ThemeDef { id: string; name: string; hasAccentVariations: boolean; }

/** The palettes: one `:root[data-theme="<id>"]` block each in styles.css. */
const PALETTE_THEMES: readonly ThemeDef[] = [
  { id: "porcelain", name: "Porcelain", hasAccentVariations: true },
  { id: "graphite", name: "Graphite", hasAccentVariations: true },
  { id: "cupertino", name: "Cupertino", hasAccentVariations: false },
  { id: "neon-grid", name: "Neon Grid", hasAccentVariations: false },
  { id: "bitcoin-defi", name: "Bitcoin DeFi", hasAccentVariations: false },
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
export function loadThemeId(storage: Pick<Storage, "getItem">): string {
  let raw: string | null = null;
  try { raw = storage.getItem(STORAGE_KEY); } catch { /* Storage can be unavailable. */ }
  if (raw === "light") return "porcelain";
  if (raw === "dark") return "graphite";
  return THEMES.some((theme) => theme.id === raw) ? raw! : "system";
}
export function resolveThemeId(id: string, dark: boolean): string {
  return id === "system" ? (dark ? "graphite" : "porcelain") : id;
}
export function saveThemeId(storage: Pick<Storage, "setItem">, id: string): void {
  try { storage.setItem(STORAGE_KEY, id); } catch { /* Session preference still works. */ }
}
export function useThemeId(): [string, (id: string) => void] {
  const revision = useSettingsRevision();
  const [id, setId] = useState(() => loadThemeId(settingsStorage));
  // The settings file arrives after the first paint, so the cached theme
  // this mounted with may not be the recorded one. Re-reading on every
  // revision is safe rather than circular: the save below writes nothing
  // when the value has not changed.
  useEffect(() => { setId(loadThemeId(settingsStorage)); }, [revision]);
  useEffect(() => {
    const query = window.matchMedia("(prefers-color-scheme: dark)");
    const apply = () => { document.documentElement.dataset.theme = resolveThemeId(id, query.matches); };
    apply();
    saveThemeId(settingsStorage, id);
    query.addEventListener("change", apply);
    return () => query.removeEventListener("change", apply);
  }, [id]);
  return [id, setId];
}
