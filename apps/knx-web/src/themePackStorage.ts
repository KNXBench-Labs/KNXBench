/** Theme plans use the existing authoritative settings client and conditional queue. */
// SPDX-License-Identifier: AGPL-3.0-or-later
import type { SettingsAcknowledgementReason } from "./settingsStore";
import { getAcknowledgedSettings, patchAcknowledgedSettings } from "./settingsStore";
import { MAX_THEME_STORE_BYTES, MAX_INSTALLED_THEME_PACKS, readThemePackStore, validateThemePack } from "./themePack";
import type { ThemePackDiagnostic } from "./themePack";
import { isBundledThemeId, THEMES } from "./theme";
export interface ThemeMutationPlan {
  readonly id: string;
  readonly requiresReplacementConsent: boolean;
  readonly settings: Record<string, unknown>;
  readonly expectedSettings: Record<string, unknown>;
}
export type ThemePlanResult = { ok: true; plan: ThemeMutationPlan } | { ok: false; diagnostic: {
  kind: ThemePackDiagnostic["kind"] | "settingsUnavailable" | "invalidStore" | "missingTheme";
  reason?: SettingsAcknowledgementReason; path: string;
}};
export class ThemeMutationError extends Error {
  constructor(readonly kind: "replacementRequired") { super(kind); }
}
function freezePlan(plan: ThemeMutationPlan): ThemeMutationPlan {
  const clone = JSON.parse(JSON.stringify(plan)) as ThemeMutationPlan;
  function freeze(value: unknown): void {
    if (value !== null && typeof value === "object") {
      for (const nested of Object.values(value)) freeze(nested);
      Object.freeze(value);
    }
  }
  freeze(clone);
  return clone;
}
export function planThemeInstallation(raw: unknown, select = false): ThemePlanResult {
  const admitted = validateThemePack(raw);
  if (!admitted.ok) return admitted;
  const snapshot = getAcknowledgedSettings(["theme", "uiThemePacks"]);
  if (!snapshot.ok) return { ok: false, diagnostic: { kind: "settingsUnavailable", reason: snapshot.reason, path: "$" } };
  const map = (snapshot.settings.uiThemePacks ?? {}) as Record<string, unknown>;
  if (typeof map !== "object" || Array.isArray(map)) return { ok: false, diagnostic: { kind: "invalidStore", path: "$" } };
  const packs = { ...map, [admitted.pack.id]: admitted.pack };
  if (Object.keys(packs).length > MAX_INSTALLED_THEME_PACKS
      || new TextEncoder().encode(JSON.stringify(packs)).byteLength > MAX_THEME_STORE_BYTES) {
    return { ok: false, diagnostic: { kind: "sizeLimit", path: "$" } };
  }
  return { ok: true, plan: freezePlan({ id: admitted.pack.id, requiresReplacementConsent: Object.hasOwn(map, admitted.pack.id),
    expectedSettings: snapshot.settings, settings: {
      uiThemePacks: packs, ...(select ? { theme: admitted.pack.id } : {}),
    } }) };
}
export function planThemeRemoval(id: string): ThemePlanResult {
  const snapshot = getAcknowledgedSettings(["theme", "uiThemePacks"]);
  if (!snapshot.ok) return { ok: false, diagnostic: { kind: "settingsUnavailable", reason: snapshot.reason, path: "$" } };
  const map = (snapshot.settings.uiThemePacks ?? {}) as Record<string, unknown>;
  if (typeof map !== "object" || Array.isArray(map)) return { ok: false, diagnostic: { kind: "invalidStore", path: "$" } };
  if (!Object.hasOwn(map, id)) return { ok: false, diagnostic: { kind: "missingTheme", path: "$" } };
  const packs = { ...map };
  delete packs[id];
  return { ok: true, plan: freezePlan({ id, requiresReplacementConsent: false, expectedSettings: snapshot.settings,
    settings: { uiThemePacks: packs, ...(snapshot.settings.theme === id ? { theme: "system" } : {}) } }) };
}
export function planThemeSelection(id: string): ThemePlanResult {
  const snapshot = getAcknowledgedSettings(["theme", "uiThemePacks"]);
  if (!snapshot.ok) return { ok: false, diagnostic: { kind: "settingsUnavailable", reason: snapshot.reason, path: "$" } };
  const shipped = isBundledThemeId(readThemePackStore(snapshot.settings.uiThemePacks), id)
    && !(typeof snapshot.settings.uiThemePacks === "object" && snapshot.settings.uiThemePacks !== null
      && Object.hasOwn(snapshot.settings.uiThemePacks, id));
  if (!THEMES.some((theme) => theme.id === id) && !shipped) {
    const map = snapshot.settings.uiThemePacks;
    if (typeof map !== "object" || map === null || Array.isArray(map)) return { ok: false, diagnostic: { kind: "missingTheme", path: "$" } };
    if (!Object.hasOwn(map, id)) return { ok: false, diagnostic: { kind: "missingTheme", path: "$" } };
    const admitted = validateThemePack((map as Record<string, unknown>)[id]);
    if (!admitted.ok) return admitted;
    if (admitted.pack.id !== id) return { ok: false, diagnostic: { kind: "invalidMetadata", path: "$.id" } };
    const store = readThemePackStore(map);
    if (!store.packs.some((pack) => pack.id === id)) {
      return { ok: false, diagnostic: store.diagnostics[0]?.diagnostic ?? { kind: "storeLimit", path: "$.uiThemePacks" } };
    }
  }
  return { ok: true, plan: freezePlan({ id, requiresReplacementConsent: false,
    expectedSettings: snapshot.settings, settings: { theme: id } }) };
}
export async function commitThemeMutation(plan: ThemeMutationPlan, replacementConsent = false): Promise<{ cacheError: boolean }> {
  if (plan.requiresReplacementConsent && !replacementConsent) throw new ThemeMutationError("replacementRequired");
  return patchAcknowledgedSettings(plan.settings, plan.expectedSettings);
}
