/** Pure projection of a ProjectDiffReport into per-table entity rows the diff panel renders. */
import type {
  AmbiguityNote,
  AreaKey,
  BuildingPartKey,
  ComObjectKey,
  DeviceChange,
  DeviceKey,
  DeviceTable,
  EntityChange,
  FieldChange,
  GroupAddressKey,
  GroupRangeKey,
  InstallationDiff,
  LineKey,
  LogEntry,
  MatchKind,
  ParameterKey,
} from "./api";
import type { MessageKey } from "./messages/en";
import type { Translate } from "./i18n";

/** How many entity rows a table shows before its "show more" control. */
export const DIFF_PAGE_SIZE = 50;

export type EntryStatus = "added" | "removed" | "changed" | "ambiguous";

// One rendered entity row. `keyLabel` is the entity's natural key as text
// (the identity the diff matched on, or failed to match on); `detail` is a
// secondary human label such as its name, never used for identity.
export interface DiffEntry {
  id: string;
  status: EntryStatus;
  keyLabel: string;
  detail: string | null;
  matchedBy: MatchKind | null;
  fieldChanges: FieldChange[];
  ambiguity: { leftCandidates: number; rightCandidates: number } | null;
  nested: DiffTableView[];
}

export interface DiffTableView {
  id: string;
  labelKey: MessageKey;
  entries: DiffEntry[];
}

// Everything a key label may need that is not in the report itself: the
// user's group-address notation and the word for a device without one.
export interface KeyFormat {
  groupAddress: (address: string) => string;
  unaddressed: string;
}

interface Describe<K, F> {
  key: (key: K) => string;
  detail: (fields: F) => string | null;
}

type EntryTable<K, F, C extends EntityChange<K, F>> = {
  added: [K, F][];
  removed: [K, F][];
  changed: C[];
  ambiguous: AmbiguityNote<K>[];
};

function baseEntry(id: string, status: EntryStatus, keyLabel: string, detail: string | null): DiffEntry {
  return { id, status, keyLabel, detail, matchedBy: null, fieldChanges: [], ambiguity: null, nested: [] };
}

// Order is fixed (added, removed, changed, ambiguous) and each group keeps
// the server's own order, so the same report always renders the same way.
function entriesOf<K, F, C extends EntityChange<K, F>>(
  prefix: string,
  table: EntryTable<K, F, C>,
  describe: Describe<K, F>,
  nested: (change: C, id: string) => DiffTableView[] = () => [],
): DiffEntry[] {
  const entries: DiffEntry[] = [];
  table.added.forEach(([key, fields], i) => {
    entries.push(baseEntry(`${prefix}.a${i}`, "added", describe.key(key), describe.detail(fields)));
  });
  table.removed.forEach(([key, fields], i) => {
    entries.push(baseEntry(`${prefix}.r${i}`, "removed", describe.key(key), describe.detail(fields)));
  });
  table.changed.forEach((change, i) => {
    const id = `${prefix}.c${i}`;
    entries.push({
      ...baseEntry(id, "changed", describe.key(change.key), describe.detail(change.right)),
      matchedBy: change.matchedBy,
      fieldChanges: change.fieldChanges,
      nested: nested(change, id),
    });
  });
  table.ambiguous.forEach((note, i) => {
    entries.push({
      ...baseEntry(`${prefix}.x${i}`, "ambiguous", describe.key(note.key), null),
      ambiguity: { leftCandidates: note.leftCandidates, rightCandidates: note.rightCandidates },
    });
  });
  return entries;
}

function tableView<K, F, C extends EntityChange<K, F>>(
  id: string,
  labelKey: MessageKey,
  table: EntryTable<K, F, C>,
  describe: Describe<K, F>,
  nested?: (change: C, id: string) => DiffTableView[],
): DiffTableView {
  return { id, labelKey, entries: entriesOf(id, table, describe, nested) };
}

const named = (fields: { name: string }) => fields.name || null;

function areaKey(key: AreaKey): string {
  return String(key.address);
}

function lineKey(key: LineKey): string {
  return `${key.areaAddress}.${key.lineAddress}`;
}

function deviceKey(key: DeviceKey, format: KeyFormat): string {
  const primary = key.address ?? format.unaddressed;
  return key.etsId ? `${primary} (${key.etsId})` : primary;
}

function groupRangeKey(key: GroupRangeKey): string {
  return `${key.start}–${key.end}`;
}

function groupAddressKey(key: GroupAddressKey, format: KeyFormat): string {
  const address = format.groupAddress(key.address);
  return key.etsId ? `${address} (${key.etsId})` : address;
}

function buildingKey(key: BuildingPartKey): string {
  return key.path.join(" › ");
}

function comObjectKey(key: ComObjectKey): string {
  return `#${key.number}`;
}

function parameterKey(key: ParameterKey): string {
  return key.etsId;
}

function deviceNested(change: DeviceChange, id: string): DiffTableView[] {
  return [
    tableView(`${id}.co`, "projectDiff.entity.comObjects", change.comObjects, {
      key: comObjectKey,
      detail: (fields) => fields.text,
    }),
    tableView(`${id}.pa`, "projectDiff.entity.parameters", change.parameters, {
      key: parameterKey,
      detail: (fields) => fields.raw,
    }),
  ].filter((view) => view.entries.length > 0);
}

function deviceTableView(id: string, table: DeviceTable, format: KeyFormat): DiffTableView {
  return tableView(
    id,
    "projectDiff.entity.devices",
    table,
    { key: (key) => deviceKey(key, format), detail: named },
    deviceNested,
  );
}

/** Every non-empty entity table of one installation, in the summary's order. */
export function installationTables(installation: InstallationDiff, format: KeyFormat): DiffTableView[] {
  const id = `i${installation.id}`;
  const views: DiffTableView[] = [
    tableView(`${id}.areas`, "projectDiff.entity.areas", installation.areas, { key: areaKey, detail: named }),
    tableView(`${id}.lines`, "projectDiff.entity.lines", installation.lines, { key: lineKey, detail: named }),
    deviceTableView(`${id}.devices`, installation.devices, format),
    tableView(`${id}.groupRanges`, "projectDiff.entity.groupRanges", installation.groupRanges, {
      key: groupRangeKey,
      detail: named,
    }),
    tableView(`${id}.groupAddresses`, "projectDiff.entity.groupAddresses", installation.groupAddresses, {
      key: (key) => groupAddressKey(key, format),
      detail: named,
    }),
    tableView(`${id}.buildings`, "projectDiff.entity.buildings", installation.buildings, {
      key: buildingKey,
      detail: named,
    }),
  ];
  return views.filter((view) => view.entries.length > 0);
}

/** One file-dialog filter, the shape `pickOpenPath` takes. */
export interface CompareFilter {
  name: string;
  extensions: string[];
}

/**
 * The comparison picker's filters: both accepted kinds first, so neither
 * the native dialog nor the mount picker hides one by default, then each
 * kind on its own. Built per call because the names follow the UI language.
 */
export function compareFilters(t: Translate): CompareFilter[] {
  return [
    { name: t("projectDiff.anyFilterName"), extensions: ["knxdb", "knxproj"] },
    { name: t("projectDiff.compareFilterName"), extensions: ["knxdb"] },
    { name: t("projectDiff.etsFilterName"), extensions: ["knxproj"] },
  ];
}

/**
 * The import-diagnostics summary line: the total, then error and warning
 * counts when non-zero. Info entries (preserved opaque data) count toward
 * the total only.
 */
export function importDiagnosticsSummary(t: Translate, diagnostics: LogEntry[]): string {
  const errors = diagnostics.filter((entry) => entry.severity === "error").length;
  const warnings = diagnostics.filter((entry) => entry.severity === "warning").length;
  const parts: string[] = [];
  if (errors > 0) parts.push(t("projectDiff.importErrors", { count: errors }));
  if (warnings > 0) parts.push(t("projectDiff.importWarnings", { count: warnings }));
  const total = t("projectDiff.importSummary", { count: diagnostics.length });
  return parts.length === 0 ? total : `${total} (${parts.join(", ")})`;
}
