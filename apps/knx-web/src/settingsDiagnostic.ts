/** Formats stable settings diagnostics through the active UI-language catalogue. */
import type { Translate } from "./i18n";
import type { SettingsDiagnostic } from "./settingsStore";

type DiagnosticInput = SettingsDiagnostic | { kind: string; [key: string]: unknown };

export function formatSettingsDiagnostic(
  t: Translate,
  diagnostic: DiagnosticInput | undefined,
  fallback: string,
): string {
  if (!diagnostic) return fallback;
  switch (diagnostic.kind) {
    case "migrated":
      if (typeof diagnostic.fromVersion !== "number" || typeof diagnostic.toVersion !== "number") return fallback;
      return t("settings.diagnostic.migrated", {
        fromVersion: diagnostic.fromVersion,
        toVersion: diagnostic.toVersion,
      });
    case "adopted":
      if (typeof diagnostic.toVersion !== "number") return fallback;
      return t("settings.diagnostic.adopted", { toVersion: diagnostic.toVersion });
    case "refusedNewer":
      if (typeof diagnostic.fileVersion !== "number" || typeof diagnostic.currentVersion !== "number") return fallback;
      return t("settings.diagnostic.refusedNewer", {
        fileVersion: diagnostic.fileVersion,
        currentVersion: diagnostic.currentVersion,
      });
    case "quarantined": {
      const reason = diagnostic.reason;
      if (
        reason !== "unreadable" && reason !== "invalidJson" && reason !== "notObject"
        && reason !== "missingSchemaVersion" && reason !== "settingsNotObject"
      ) return fallback;
      if (typeof diagnostic.movedTo !== "string") return fallback;
      return t(`settings.diagnostic.quarantined.${reason}`, { movedTo: diagnostic.movedTo });
    }
    default:
      return fallback;
  }
}
