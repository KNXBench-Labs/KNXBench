import { useEffect, useState } from "react";

export type Theme = "system" | "light" | "dark";

const ORDER: Theme[] = ["system", "light", "dark"];
const STORAGE_KEY = "knx-desktop:theme";

/** Cycles System -> Light -> Dark -> System. */
export function nextTheme(current: Theme): Theme {
  return ORDER[(ORDER.indexOf(current) + 1) % ORDER.length];
}

/**
 * Reads the persisted theme. Any stored value other than "light"/"dark"
 * (missing key, or a value from a future/incompatible version) resolves
 * to "system" — the always-safe default, never a hard failure.
 */
export function loadTheme(storage: Pick<Storage, "getItem">): Theme {
  const raw = storage.getItem(STORAGE_KEY);
  return raw === "light" || raw === "dark" ? raw : "system";
}

export function saveTheme(storage: Pick<Storage, "setItem">, theme: Theme): void {
  storage.setItem(STORAGE_KEY, theme);
}

/**
 * Reads the persisted theme on mount, applies it to `<html data-theme>`
 * (removed entirely for "system", so the `prefers-color-scheme` media
 * query in styles.css governs), and persists on every change. The
 * returned setter is a cycle-to-next function, not an arbitrary setter —
 * the toggle button is the only caller and only ever advances the cycle.
 */
export function useTheme(): [Theme, () => void] {
  const [theme, setTheme] = useState<Theme>(() => loadTheme(window.localStorage));

  useEffect(() => {
    if (theme === "system") {
      document.documentElement.removeAttribute("data-theme");
    } else {
      document.documentElement.setAttribute("data-theme", theme);
    }
    saveTheme(window.localStorage, theme);
  }, [theme]);

  return [theme, () => setTheme(nextTheme)];
}
