/** Applies validated theme data through a reversible, bounded inline-property lease. */
// SPDX-License-Identifier: AGPL-3.0-or-later
import type { ThemePackDiagnostic } from "./themePack";
import { THEME_PACK_ACCENTS, validateThemePack } from "./themePack";
export type ThemePackApplication = { ok: true; release: () => void } | { ok: false; diagnostic: ThemePackDiagnostic };
export function applyThemePack(root: HTMLElement, raw: unknown, accent: string): ThemePackApplication {
  const checked = validateThemePack(raw);
  if (!checked.ok) return checked;
  const variation = THEME_PACK_ACCENTS.find((id) => id === accent);
  const values: Record<string, string> = { ...checked.pack.tokens,
    ...(variation ? checked.pack.accents?.[variation] : {}), "color-scheme": checked.pack.colorScheme };
  const original = Object.keys(values).map((key) => ({
    key, value: root.style.getPropertyValue(key), priority: root.style.getPropertyPriority(key),
  }));
  for (const [key, value] of Object.entries(values)) root.style.setProperty(key, value);
  let released = false;
  return { ok: true, release: () => {
    if (released) return;
    released = true;
    for (const previous of original) {
      if (previous.value) root.style.setProperty(previous.key, previous.value, previous.priority);
      else root.style.removeProperty(previous.key);
    }
  } };
}
