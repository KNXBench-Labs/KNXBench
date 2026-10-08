/** Pure presentation grouping; all source diagnostics and access metadata stay intact. */
import type { ParameterDiagnostic, ParameterField } from "./api";

export function isManufacturerRestricted(field: ParameterField): boolean {
  return field.access === "None" || field.access === "Read";
}

export function groupParameterDiagnostics(diagnostics: readonly ParameterDiagnostic[]) {
  const groups = new Map<string, ParameterDiagnostic[]>();
  for (const diagnostic of diagnostics) {
    const scope = diagnostic.scope;
    const key = JSON.stringify([
      diagnostic.kind, diagnostic.severity === "info" ? "info" : "warning", diagnostic.message,
      scope === null ? null : [scope.moduleNode, scope.moduleId, scope.moduleDefId],
    ]);
    const group = groups.get(key);
    if (group) group.push(diagnostic);
    else groups.set(key, [diagnostic]);
  }
  return [...groups.values()];
}
