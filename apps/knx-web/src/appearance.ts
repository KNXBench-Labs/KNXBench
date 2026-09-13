/** Presentation preferences only; semantic status colors never depend on accent. */
import { useEffect, useState } from "react";
export const ACCENTS = ["violet", "mint", "blue", "amber", "rose"] as const;
export const DENSITIES = ["compact", "comfortable"] as const;
export type Accent = typeof ACCENTS[number];
export type Density = typeof DENSITIES[number];
export function loadAppearance(storage: Pick<Storage, "getItem">): { accent: Accent; density: Density } {
  let accent: string | null = null;
  let density: string | null = null;
  try {
    accent = storage.getItem("knx-desktop:accent");
    density = storage.getItem("knx-desktop:density");
  } catch { /* Private/browser storage restrictions use defaults. */ }
  return {
    accent: ACCENTS.includes(accent as Accent) ? accent as Accent : "violet",
    density: DENSITIES.includes(density as Density) ? density as Density : "compact",
  };
}
export function useAppearance() {
  const [preferences, setPreferences] = useState(() => loadAppearance(window.localStorage));
  const { accent, density } = preferences;
  useEffect(() => {
    document.documentElement.dataset.accent = accent;
    document.documentElement.dataset.density = density;
    try {
      localStorage.setItem("knx-desktop:accent", accent);
      localStorage.setItem("knx-desktop:density", density);
    } catch { /* Still apply within this session. */ }
  }, [accent, density]);
  return {
    accent, density,
    setAccent: (value: Accent) => setPreferences((p) => ({ ...p, accent: value })),
    setDensity: (value: Density) => setPreferences((p) => ({ ...p, density: value })),
  };
}
