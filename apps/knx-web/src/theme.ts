import { useEffect, useState } from "react";

export interface ThemeDef {
  id: string;
  name: string;
}

export const THEMES: readonly ThemeDef[] = [{ id: "bitcoin-defi", name: "Bitcoin DeFi" }];

const DEFAULT_THEME_ID = "bitcoin-defi";
const STORAGE_KEY = "knx-desktop:theme";

function isThemeId(id: string): boolean {
  return THEMES.some((t) => t.id === id);
}

/**
 * Reads the persisted theme id. Anything that isn't a known theme id —
 * missing key, a value from a future/incompatible version, cycle 7's old
 * "system"/"light"/"dark" values, or a theme that's since been removed —
 * resolves to the default, the same always-safe-default philosophy as the
 * old `loadTheme`.
 */
export function loadThemeId(storage: Pick<Storage, "getItem">): string {
  const raw = storage.getItem(STORAGE_KEY);
  return raw && isThemeId(raw) ? raw : DEFAULT_THEME_ID;
}

export function saveThemeId(storage: Pick<Storage, "setItem">, id: string): void {
  storage.setItem(STORAGE_KEY, id);
}

/**
 * Reads the persisted theme id on mount, applies it to `<html
 * data-theme>`, and persists on every change. Unlike the old `useTheme`,
 * the attribute is always set — there is no "system"/unthemed state
 * anymore.
 */
export function useThemeId(): [string, (id: string) => void] {
  const [id, setId] = useState<string>(() => loadThemeId(window.localStorage));

  useEffect(() => {
    document.documentElement.setAttribute("data-theme", id);
    saveThemeId(window.localStorage, id);
  }, [id]);

  return [id, setId];
}
