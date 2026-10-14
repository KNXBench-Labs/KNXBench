import { useEffect, useState } from "react";
import { settingsStorage, useSettingsRevision } from "./settingsStore";

export interface MotionLevelDef {
  id: string;
  name: string;
}

export interface MotionStyleDef {
  id: string;
  name: string;
}

// index.html's inline pre-mount bootstrap script duplicates these ids
// (it cannot import this module — it runs before any module loads). Keep
// both lists in sync when a level or style is added or removed.
export const MOTION_LEVELS: readonly MotionLevelDef[] = [
  { id: "off", name: "Off" },
  { id: "subtle", name: "Subtle" },
  { id: "standard", name: "Standard" },
];

export const MOTION_STYLES: readonly MotionStyleDef[] = [
  { id: "apple", name: "Smooth" },
  { id: "glitch", name: "Glitch" },
  { id: "crt", name: "CRT" },
];

const DEFAULT_MOTION_LEVEL_ID = "standard";
const DEFAULT_MOTION_STYLE_ID = "apple";
// Keys inside the settings document the server keeps, not `localStorage`
// keys: `settingsStorage` is what resolves them against the record.
const LEVEL_STORAGE_KEY = "motionLevel";
const STYLE_STORAGE_KEY = "motionStyle";

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
  const revision = useSettingsRevision();
  const [level, setLevel] = useState<string>(() => loadMotionLevel(settingsStorage));
  const [style, setStyle] = useState<string>(() => loadMotionStyle(settingsStorage));

  // The record can land after this mounted on the cached document; the
  // saves below write nothing when a re-read changes nothing.
  useEffect(() => {
    setLevel(loadMotionLevel(settingsStorage));
    setStyle(loadMotionStyle(settingsStorage));
  }, [revision]);

  useEffect(() => {
    for (const category of MOTION_CATEGORIES) document.documentElement.setAttribute(
      `data-motion-${category}`, effectiveMotionLevel(level, loadMotionCategory(settingsStorage, category)));
  }, [revision, level]);

  useEffect(() => {
    document.documentElement.setAttribute("data-motion-level", level);
    saveMotionLevel(settingsStorage, level);
  }, [level]);

  useEffect(() => {
    document.documentElement.setAttribute("data-motion-style", style);
    saveMotionStyle(settingsStorage, style);
  }, [style]);

  return { level, setLevel, style, setStyle };
}

export type MotionCategory = "toasts" | "achievements" | "flow" | "lines";
export type CategoryMotion = "inherit" | "off" | "subtle";
// `lines`: the project explorer's structure lines (owner feedback P2):
// standard grows and pulses them, subtle only grows them, off draws them still.
export const MOTION_CATEGORIES = ["toasts", "achievements", "flow", "lines"] as const;
const CATEGORY_KEYS: Record<MotionCategory, string> = { toasts: "motionToasts", achievements: "motionAchievements", flow: "motionFlow", lines: "motionLines" };
export function loadMotionCategory(storage: Pick<Storage, "getItem">, category: MotionCategory): CategoryMotion {
  const raw = storage.getItem(CATEGORY_KEYS[category]);
  return raw === "off" || raw === "subtle" ? raw : "inherit";
}
/** Categories can only reduce; global Off and the OS preference always win. */
export function effectiveMotionLevel(global: string, category: string, osReduce = false): string {
  if (osReduce || global === "off" || category === "off") return "off";
  if (global === "subtle" || category === "subtle") return "subtle";
  return "standard";
}
export function saveMotionCategory(storage: Pick<Storage, "setItem">, category: MotionCategory, value: CategoryMotion): void {
  storage.setItem(CATEGORY_KEYS[category], value);
}
