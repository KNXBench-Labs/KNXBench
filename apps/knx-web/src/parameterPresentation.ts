/** Pure presentation grouping; all source diagnostics and access metadata stay intact. */
import type { ParameterDiagnostic, ParameterField, ParameterSection, ParameterSeparator } from "./api";

const restricting = (access: string | null | undefined) => access === "None" || access === "Read";

/** The field's own `Access` or (ADR-0139) an enclosing block's is `None` or `Read`. */
export function isManufacturerRestricted(field: ParameterField): boolean {
  return restricting(field.access) || restricting(field.blockAccess);
}

/** ADR-0139: the access that restricts `field`, its own first; `null` when neither does. */
export function restrictingAccess(field: ParameterField): { access: "None" | "Read"; from: "field" | "block" } | null {
  if (restricting(field.access)) return { access: field.access as "None" | "Read", from: "field" };
  if (restricting(field.blockAccess)) return { access: field.blockAccess as "None" | "Read", from: "block" };
  return null;
}

/** ADR-0139: a separator the manufacturer hid or that sits in a restricted block. */
export function isSeparatorRestricted(separator: ParameterSeparator): boolean {
  return restricting(separator.access) || restricting(separator.blockAccess);
}

/**
 * Keeps the chosen fields and separators of a section; each kept separator's
 * `position` is renumbered to stand before the same kept field as before.
 */
export function filterParameterSection(section: ParameterSection, keepField: (field: ParameterField) => boolean,
  keepSeparator: (separator: ParameterSeparator) => boolean): ParameterSection {
  const keptBefore: number[] = [0];
  const fields: ParameterField[] = [];
  for (const field of section.fields) {
    if (keepField(field)) fields.push(field);
    keptBefore.push(fields.length);
  }
  const separators = (section.separators ?? []).filter(keepSeparator).map(separator => ({
    ...separator,
    position: keptBefore[Math.min(Math.max(separator.position, 0), section.fields.length)],
  }));
  return { ...section, fields, separators };
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
