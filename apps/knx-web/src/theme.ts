/** The registry of selectable themes, and where the choice is persisted and read back. */
import { useEffect, useState } from "react";

/** `hasAccentVariations` is whether styles.css declares any
 * `[data-accent="…"]` variation for this theme (ADR-0022: three of five
 * treat the accent as identity and declare none). `themeTokens.test.ts`
 * checks this flag against the stylesheet so it cannot go stale. */
export interface ThemeDef { id: string; name: string; hasAccentVariations: boolean; }
export const THEMES: readonly ThemeDef[] = [
  { id: "system", name: "System", hasAccentVariations: true },
  { id: "porcelain", name: "Porcelain", hasAccentVariations: true },
  { id: "graphite", name: "Graphite", hasAccentVariations: true },
  { id: "cupertino", name: "Cupertino", hasAccentVariations: false },
  { id: "neon-grid", name: "Neon Grid", hasAccentVariations: false },
  { id: "bitcoin-defi", name: "Bitcoin DeFi", hasAccentVariations: false },
];
const STORAGE_KEY = "knx-desktop:theme";

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
  const [id, setId] = useState(() => loadThemeId(window.localStorage));
  useEffect(() => {
    const query = window.matchMedia("(prefers-color-scheme: dark)");
    const apply = () => { document.documentElement.dataset.theme = resolveThemeId(id, query.matches); };
    apply();
    saveThemeId(window.localStorage, id);
    query.addEventListener("change", apply);
    return () => query.removeEventListener("change", apply);
  }, [id]);
  return [id, setId];
}
