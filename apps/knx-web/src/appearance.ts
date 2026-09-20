/** Presentation preferences only; semantic status colors never depend on accent. */
import { useEffect, useState } from "react";
import { settingsStorage, useSettingsRevision } from "./settingsStore";
export const ACCENTS = ["violet", "mint", "blue", "amber", "rose"] as const;
export const DENSITIES = ["compact", "comfortable"] as const;
export type Accent = typeof ACCENTS[number];
export type Density = typeof DENSITIES[number];
/** Keys inside the settings document the server keeps, not `localStorage`
 * keys — `settingsStorage` is what resolves them. */
export const ACCENT_KEY = "accent";
export const DENSITY_KEY = "density";

export function loadAppearance(storage: Pick<Storage, "getItem">): { accent: Accent; density: Density } {
  let accent: string | null = null;
  let density: string | null = null;
  try {
    accent = storage.getItem(ACCENT_KEY);
    density = storage.getItem(DENSITY_KEY);
  } catch { /* Private/browser storage restrictions use defaults. */ }
  return {
    accent: ACCENTS.includes(accent as Accent) ? accent as Accent : "violet",
    density: DENSITIES.includes(density as Density) ? density as Density : "compact",
  };
}
export function useAppearance() {
  const revision = useSettingsRevision();
  const [preferences, setPreferences] = useState(() => loadAppearance(settingsStorage));
  const { accent, density } = preferences;
  // Re-read whenever the record changes — most importantly when the
  // server's answer lands after this mounted on the cached document.
  useEffect(() => { setPreferences(loadAppearance(settingsStorage)); }, [revision]);
  useEffect(() => {
    document.documentElement.dataset.accent = accent;
    document.documentElement.dataset.density = density;
    try {
      settingsStorage.setItem(ACCENT_KEY, accent);
      settingsStorage.setItem(DENSITY_KEY, density);
    } catch { /* Still apply within this session. */ }
  }, [accent, density]);
  return {
    accent, density,
    setAccent: (value: Accent) => setPreferences((p) => ({ ...p, accent: value })),
    setDensity: (value: Density) => setPreferences((p) => ({ ...p, density: value })),
  };
}
