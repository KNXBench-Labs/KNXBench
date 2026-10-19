/** Read-only parameter navigation derived exclusively from declared scopes and groups. */
import type { ModuleScope, ParameterField, ParameterSection, ParameterSeparator } from "./api";
import type { TextFill } from "./bindings/TextFill";
import { classifyGroupLayout, type GroupLayoutView } from "./parameterMatrix";
import { moduleScopeIdentity } from "./moduleScope";

export interface ParameterWorkspaceGroup {
  key: string;
  sectionKey: string;
  scope: ModuleScope | null;
  label: string | null;
  labelFill?: TextFill | null;
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

/**
 * One key per section: its scope identity (ADR-0063 chain, else the legacy
 * fields) plus its occurrence among equal identities in this response.
 */
export function parameterSectionKeys(sections: readonly ParameterSection[]): string[] {
  const occurrences = new Map<string, number>();
  return sections.map(section => {
    const identity = moduleScopeIdentity(section.scope);
    const occurrence = occurrences.get(identity) ?? 0;
    occurrences.set(identity, occurrence + 1);
    return JSON.stringify([identity, occurrence]);
  });
}

export function indexParameterWorkspace(sections: readonly ParameterSection[]) {
  const rows: ParameterWorkspaceRow[] = [];
  const groups: ParameterWorkspaceGroup[] = [];
  const items: ParameterWorkspaceItem[] = [];
  const sectionKeys = parameterSectionKeys(sections);
  for (const [sectionIndex, section] of sections.entries()) {
    const scope = section.scope;
    const sectionKey = sectionKeys[sectionIndex];
    const declared = new Map<string, ParameterWorkspaceGroup>();
    // A group is opened by whichever comes first, field or separator, so a
    // block holding only text is an area of its own (ADR-0139).
    const groupFor = (group: ParameterField["group"]) => {
      const groupIdentity = JSON.stringify(group == null ? ["ungrouped"] : ["group", group.key]);
      let found = declared.get(groupIdentity);
      if (!found) {
        found = { key: JSON.stringify([sectionKey, groupIdentity]), sectionKey, scope,
          label: group?.label ?? null, labelFill: group?.labelFill, number: declared.size + 1, ungrouped: group == null,
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
export function matchesParameterSearch(row: ParameterWorkspaceRow, query: string, displayedGroupLabel?: string): boolean {
  const field = row.field;
  const text = [field.etsId, field.name, field.text, field.value, displayedGroupLabel, row.groupLabel,
    field.group?.labelFill?.template, row.scope?.moduleId,
    ...field.enumOptions.flatMap(option => [option.value, option.text])].join(" ").toLocaleLowerCase();
  return query.trim().toLocaleLowerCase().split(/\s+/u).every(term => text.includes(term));
}
