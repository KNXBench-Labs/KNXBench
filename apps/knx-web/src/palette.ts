/**
 * User-customizable theme tokens, layered on top of the light/dark/system
 * base theme (see theme.ts). Independent of that cycle — a custom color
 * applies whichever base theme is active, by overriding one CSS custom
 * property at a time on the root element.
 */

import { useEffect, useState } from "react";

export type TokenName = "accent" | "bg" | "surface" | "text";

export const TOKENS: readonly TokenName[] = ["accent", "bg", "surface", "text"];

export type MotionLevel = "off" | "subtle" | "standard";

const MOTION_LEVELS: readonly MotionLevel[] = ["off", "subtle", "standard"];

/**
 * Transition duration per motion level. Only takes effect under
 * `prefers-reduced-motion: no-preference` (see styles.css) — the OS
 * setting always wins over this one, never the other way round.
 */
const MOTION_DURATIONS: Record<MotionLevel, string> = {
  off: "0ms",
  subtle: "100ms",
  standard: "250ms",
};

export type CustomPalette = Partial<Record<TokenName, string>>;

export interface PaletteSettings {
  colors: CustomPalette;
  motion: MotionLevel;
}

const STORAGE_KEY = "knx-desktop:palette";

const DEFAULT_SETTINGS: PaletteSettings = { colors: {}, motion: "standard" };

function isTokenName(key: string): key is TokenName {
  return (TOKENS as readonly string[]).includes(key);
}

/**
 * Reads the persisted palette. Anything that doesn't match the expected
 * shape — malformed JSON, unknown token keys, an unrecognized motion
 * level — is dropped rather than rejected outright, the same
 * always-safe-default philosophy as `theme.ts`'s `loadTheme`.
 */
export function loadPalette(storage: Pick<Storage, "getItem">): PaletteSettings {
  const raw = storage.getItem(STORAGE_KEY);
  if (!raw) return DEFAULT_SETTINGS;

  let parsed: unknown;
  try {
    parsed = JSON.parse(raw);
  } catch {
    return DEFAULT_SETTINGS;
  }
  if (typeof parsed !== "object" || parsed === null) return DEFAULT_SETTINGS;

  const candidate = parsed as { colors?: unknown; motion?: unknown };
  const colors: CustomPalette = {};
  if (typeof candidate.colors === "object" && candidate.colors !== null) {
    for (const [key, value] of Object.entries(candidate.colors as Record<string, unknown>)) {
      if (isTokenName(key) && typeof value === "string") colors[key] = value;
    }
  }
  const motion = MOTION_LEVELS.includes(candidate.motion as MotionLevel)
    ? (candidate.motion as MotionLevel)
    : "standard";

  return { colors, motion };
}

export function savePalette(storage: Pick<Storage, "setItem">, settings: PaletteSettings): void {
  storage.setItem(STORAGE_KEY, JSON.stringify(settings));
}

/** The subset of an HTMLElement this module needs — kept narrow for testability. */
type StyleTarget = { style: Pick<CSSStyleDeclaration, "setProperty" | "removeProperty"> };

/**
 * Applies palette settings to an element's inline style, one custom
 * property per token. A token without an override is removed rather than
 * left stale, so it falls back through to the base theme's own value.
 */
export function applyPalette(el: StyleTarget, settings: PaletteSettings): void {
  for (const token of TOKENS) {
    const value = settings.colors[token];
    if (value) el.style.setProperty(`--knx-${token}`, value);
    else el.style.removeProperty(`--knx-${token}`);
  }
  el.style.setProperty("--knx-transition-duration", MOTION_DURATIONS[settings.motion]);
}

/**
 * Reads the persisted palette on mount, applies it to `<html>`, and
 * persists on every change. Mirrors `theme.ts`'s `useTheme` shape, but
 * exposes setters for individual tokens/motion rather than a single
 * cycle-to-next function — the settings panel edits one field at a time.
 */
export function usePalette(): [PaletteSettings, (token: TokenName, value: string | undefined) => void, (motion: MotionLevel) => void, () => void] {
  const [settings, setSettings] = useState<PaletteSettings>(() => loadPalette(window.localStorage));

  useEffect(() => {
    applyPalette(document.documentElement, settings);
    savePalette(window.localStorage, settings);
  }, [settings]);

  function setColor(token: TokenName, value: string | undefined) {
    setSettings((prev) => {
      const colors = { ...prev.colors };
      if (value) colors[token] = value;
      else delete colors[token];
      return { ...prev, colors };
    });
  }

  function setMotion(motion: MotionLevel) {
    setSettings((prev) => ({ ...prev, motion }));
  }

  function resetAll() {
    setSettings({ colors: {}, motion: "standard" });
  }

  return [settings, setColor, setMotion, resetAll];
}
