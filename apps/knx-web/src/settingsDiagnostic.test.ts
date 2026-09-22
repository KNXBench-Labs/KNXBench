/** Verifies typed settings diagnostics select catalogue keys rather than server prose. */
import { expect, it } from "vitest";
import type { Translate } from "./i18n";
import { formatSettingsDiagnostic } from "./settingsDiagnostic";

const t = ((key: string, params?: Record<string, unknown>) =>
  `${key}:${JSON.stringify(params ?? {})}`) as Translate;

it.each([
  [{ kind: "migrated", fromVersion: 0, toVersion: 1 }, "settings.diagnostic.migrated"],
  [{ kind: "adopted", fromVersion: 0, toVersion: 1 }, "settings.diagnostic.adopted"],
  [{ kind: "refusedNewer", fileVersion: 4, currentVersion: 1 }, "settings.diagnostic.refusedNewer"],
  [{ kind: "quarantined", reason: "unreadable", movedTo: "settings.damaged.json" }, "settings.diagnostic.quarantined.unreadable"],
  [{ kind: "quarantined", reason: "invalidJson", movedTo: "settings.damaged.json" }, "settings.diagnostic.quarantined.invalidJson"],
  [{ kind: "quarantined", reason: "notObject", movedTo: "settings.damaged.json" }, "settings.diagnostic.quarantined.notObject"],
  [{ kind: "quarantined", reason: "missingSchemaVersion", movedTo: "settings.damaged.json" }, "settings.diagnostic.quarantined.missingSchemaVersion"],
  [{ kind: "quarantined", reason: "settingsNotObject", movedTo: "settings.damaged.json" }, "settings.diagnostic.quarantined.settingsNotObject"],
] as const)("maps %j", (diagnostic, key) => {
  expect(formatSettingsDiagnostic(t, diagnostic, "server fallback")).toContain(key);
  expect(formatSettingsDiagnostic(t, diagnostic, "server fallback")).not.toContain("server fallback");
});

it("uses fallback for absent and future diagnostics", () => {
  expect(formatSettingsDiagnostic(t, undefined, "server fallback")).toBe("server fallback");
  expect(formatSettingsDiagnostic(t, { kind: "future" }, "server fallback")).toBe("server fallback");
});
