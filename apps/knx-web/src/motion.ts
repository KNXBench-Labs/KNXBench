import { useEffect, useState } from "react";

export interface MotionLevelDef {
  id: string;
  name: string;
}

export interface MotionStyleDef {
  id: string;
  name: string;
}

export const MOTION_LEVELS: readonly MotionLevelDef[] = [
  { id: "off", name: "Off" },
  { id: "subtle", name: "Subtle" },
  { id: "standard", name: "Standard" },
];

export const MOTION_STYLES: readonly MotionStyleDef[] = [
  { id: "apple", name: "Smooth" },
  { id: "glitch", name: "Glitch" },
];

const DEFAULT_MOTION_LEVEL_ID = "standard";
const DEFAULT_MOTION_STYLE_ID = "apple";
const LEVEL_STORAGE_KEY = "knx-desktop:motion-level";
const STYLE_STORAGE_KEY = "knx-desktop:motion-style";

function isMotionLevelId(id: string): boolean {
  return MOTION_LEVELS.some((l) => l.id === id);
}

function isMotionStyleId(id: string): boolean {
  return MOTION_STYLES.some((s) => s.id === id);
}

/**
 * Reads the persisted motion level id. Anything that isn't a known level
 * id — missing key, empty string, a value from a future/incompatible
 * version, or a level that's since been removed — resolves to the
 * default, the same always-safe-default philosophy as `loadThemeId`.
 */
export function loadMotionLevel(storage: Pick<Storage, "getItem">): string {
  const raw = storage.getItem(LEVEL_STORAGE_KEY);
  return raw && isMotionLevelId(raw) ? raw : DEFAULT_MOTION_LEVEL_ID;
}

/**
 * Reads the persisted motion style id. Same fall-back-to-default rule as
 * `loadMotionLevel`.
 */
export function loadMotionStyle(storage: Pick<Storage, "getItem">): string {
  const raw = storage.getItem(STYLE_STORAGE_KEY);
  return raw && isMotionStyleId(raw) ? raw : DEFAULT_MOTION_STYLE_ID;
}

export function saveMotionLevel(storage: Pick<Storage, "setItem">, id: string): void {
  storage.setItem(LEVEL_STORAGE_KEY, id);
}

export function saveMotionStyle(storage: Pick<Storage, "setItem">, id: string): void {
  storage.setItem(STYLE_STORAGE_KEY, id);
}

/**
 * Reads the persisted level and style on mount, applies them to
 * `<html data-motion-level>` and `<html data-motion-style>`, and persists
 * each on every change, exactly the way `useThemeId` handles `data-theme`.
 * `prefers-reduced-motion: reduce` is not consulted here — it is enforced
 * structurally in CSS so it stays the one, undefeatable source of truth.
 */
export function useMotion(): {
  level: string;
  setLevel: (id: string) => void;
  style: string;
  setStyle: (id: string) => void;
} {
  const [level, setLevel] = useState<string>(() => loadMotionLevel(window.localStorage));
  const [style, setStyle] = useState<string>(() => loadMotionStyle(window.localStorage));

  useEffect(() => {
    document.documentElement.setAttribute("data-motion-level", level);
    saveMotionLevel(window.localStorage, level);
  }, [level]);

  useEffect(() => {
    document.documentElement.setAttribute("data-motion-style", style);
    saveMotionStyle(window.localStorage, style);
  }, [style]);

  return { level, setLevel, style, setStyle };
}
