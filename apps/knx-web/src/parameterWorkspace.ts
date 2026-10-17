/** Read-only parameter navigation derived exclusively from declared scopes and groups. */
import type { ModuleScope, ParameterField, ParameterSection, ParameterSeparator } from "./api";
import { classifyGroupLayout, type GroupLayoutView } from "./parameterMatrix";

export interface ParameterWorkspaceGroup {
  key: string;
  sectionKey: string;
  scope: ModuleScope | null;
  label: string | null;
  /** Source-order group number within its scope, not a channel number. */
  number: number;
  ungrouped: boolean;
  /** Declared grid or the reason the row view stays (ADR-0137). */
  layout: GroupLayoutView;
}
export interface ParameterWorkspaceRow {
  key: string;
  groupKey: string;
  field: ParameterField;
  scope: ModuleScope | null;
  groupLabel: string | null;
}
/** ADR-0139: an activated separator, display only, in its declared group. */
export interface ParameterWorkspaceSeparator {
  key: string;
  groupKey: string;
  separator: ParameterSeparator;
  scope: ModuleScope | null;
}
/** Fields and separators in source order, the order the workspace renders. */
export type ParameterWorkspaceItem =
  | { kind: "field"; row: ParameterWorkspaceRow }
  | { kind: "separator"; separator: ParameterWorkspaceSeparator };

export function indexParameterWorkspace(sections: readonly ParameterSection[]) {
  const rows: ParameterWorkspaceRow[] = [];
  const groups: ParameterWorkspaceGroup[] = [];
  const items: ParameterWorkspaceItem[] = [];
  const sectionOccurrences = new Map<string, number>();
  for (const section of sections) {
    const scope = section.scope;
    const identity = JSON.stringify(scope === null ? null : [scope.moduleNode, scope.moduleId, scope.moduleDefId]);
    const occurrence = sectionOccurrences.get(identity) ?? 0;
    sectionOccurrences.set(identity, occurrence + 1);
    const sectionKey = JSON.stringify([identity, occurrence]);
    const declared = new Map<string, ParameterWorkspaceGroup>();
    // A group is opened by whichever comes first, field or separator, so a
    // block holding only text is an area of its own (ADR-0139).
    const groupFor = (group: ParameterField["group"]) => {
      const groupIdentity = JSON.stringify(group == null ? ["ungrouped"] : ["group", group.key]);
      let found = declared.get(groupIdentity);
      if (!found) {
        found = { key: JSON.stringify([sectionKey, groupIdentity]), sectionKey, scope,
          label: group?.label ?? null, number: declared.size + 1, ungrouped: group == null,
          layout: classifyGroupLayout(section.groupLayouts, group?.key ?? null) };
        declared.set(groupIdentity, found);
        groups.push(found);
      }
      return found;
    };
    const separators = [...(section.separators ?? [])].sort((a, b) => a.position - b.position);
    let nextSeparator = 0;
    const separatorsBefore = (position: number) => {
      while (nextSeparator < separators.length && separators[nextSeparator].position <= position) {
        const separator = separators[nextSeparator];
        items.push({ kind: "separator", separator: { key: JSON.stringify([sectionKey, "separator", nextSeparator]),
          groupKey: groupFor(separator.group).key, separator, scope } });
        nextSeparator += 1;
      }
    };
    const fieldOccurrences = new Map<string, number>();
    section.fields.forEach((field, index) => {
      separatorsBefore(index);
      const group = groupFor(field.group);
      const fieldOccurrence = fieldOccurrences.get(field.etsId) ?? 0;
      fieldOccurrences.set(field.etsId, fieldOccurrence + 1);
      const row = { key: JSON.stringify([sectionKey, field.etsId, fieldOccurrence]),
        groupKey: group.key, field, scope, groupLabel: group.label };
      rows.push(row);
      items.push({ kind: "field", row });
    });
    separatorsBefore(Number.POSITIVE_INFINITY);
  }
  return { rows, groups, items };
}

/** All terms must match; labels/options are searchable but never parsed for identity. */
export function matchesParameterSearch(row: ParameterWorkspaceRow, query: string): boolean {
  const field = row.field;
  const text = [field.etsId, field.name, field.text, field.value, row.groupLabel, row.scope?.moduleId,
    ...field.enumOptions.flatMap(option => [option.value, option.text])].join(" ").toLocaleLowerCase();
  return query.trim().toLocaleLowerCase().split(/\s+/u).every(term => text.includes(term));
}
