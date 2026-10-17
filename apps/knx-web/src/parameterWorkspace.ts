/** Read-only parameter navigation derived exclusively from declared scopes and groups. */
import type { ModuleScope, ParameterField, ParameterSection } from "./api";

export interface ParameterWorkspaceGroup {
  key: string;
  sectionKey: string;
  scope: ModuleScope | null;
  label: string | null;
  /** Source-order group number within its scope, not a channel number. */
  number: number;
  ungrouped: boolean;
}
export interface ParameterWorkspaceRow {
  key: string;
  groupKey: string;
  field: ParameterField;
  scope: ModuleScope | null;
  groupLabel: string | null;
}

export function indexParameterWorkspace(sections: readonly ParameterSection[]) {
  const rows: ParameterWorkspaceRow[] = [];
  const groups: ParameterWorkspaceGroup[] = [];
  const sectionOccurrences = new Map<string, number>();
  for (const section of sections) {
    const scope = section.scope;
    const identity = JSON.stringify(scope === null ? null : [scope.moduleNode, scope.moduleId, scope.moduleDefId]);
    const occurrence = sectionOccurrences.get(identity) ?? 0;
    sectionOccurrences.set(identity, occurrence + 1);
    const sectionKey = JSON.stringify([identity, occurrence]);
    const declared = new Map<string, ParameterWorkspaceGroup>();
    const fieldOccurrences = new Map<string, number>();
    for (const field of section.fields) {
      const groupIdentity = JSON.stringify(field.group == null ? ["ungrouped"] : ["group", field.group.key]);
      let group = declared.get(groupIdentity);
      if (!group) {
        group = { key: JSON.stringify([sectionKey, groupIdentity]), sectionKey, scope,
          label: field.group?.label ?? null, number: declared.size + 1, ungrouped: field.group == null };
        declared.set(groupIdentity, group);
        groups.push(group);
      }
      const fieldOccurrence = fieldOccurrences.get(field.etsId) ?? 0;
      fieldOccurrences.set(field.etsId, fieldOccurrence + 1);
      rows.push({ key: JSON.stringify([sectionKey, field.etsId, fieldOccurrence]),
        groupKey: group.key, field, scope, groupLabel: group.label });
    }
  }
  return { rows, groups };
}

/** All terms must match; labels/options are searchable but never parsed for identity. */
export function matchesParameterSearch(row: ParameterWorkspaceRow, query: string): boolean {
  const field = row.field;
  const text = [field.etsId, field.name, field.text, field.value, row.groupLabel, row.scope?.moduleId,
    ...field.enumOptions.flatMap(option => [option.value, option.text])].join(" ").toLocaleLowerCase();
  return query.trim().toLocaleLowerCase().split(/\s+/u).every(term => text.includes(term));
}
