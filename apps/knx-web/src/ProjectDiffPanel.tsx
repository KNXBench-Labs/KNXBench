import { useState } from "react";
import { pickOpenPath } from "./filePicker";
import * as api from "./api";
import type { ProjectDiffReport } from "./api";
import type { ProjectTree } from "./bindings/ProjectTree";

// Lets the user pick an *existing* `.knxdb` file to compare the live,
// possibly edited, in-memory project against — "what would Save change"
// (design spec `docs/superpowers/specs/2026-09-10-project-diff-design.md`
// §7), never a claim of ETS parity. `pickOpenPath`, not `pickSavePath`:
// unlike `DocumentationExportButton`'s export target, this file must
// already exist.
const COMPARE_FILTER = [{ name: "KNXBench project", extensions: ["knxdb"] }];

// One non-generic entity table's shape, enough for the grouped-count
// renderer below — every table in a `ProjectDiffReport` (`api.ts`'s
// `EntityTable<K, F>` and its non-generic sibling `DeviceTable`) fits
// this shape, since only the four arrays' lengths are ever read here.
interface CountableTable {
  added: unknown[];
  removed: unknown[];
  changed: unknown[];
  ambiguous: unknown[];
}

// Builds the one-line grouped-count summary for a non-empty table (design
// spec §5/§9: grouped counts only, no tree view, no inline before/after
// highlighting). Returns `null` for an empty table so callers can filter
// those out without rendering an empty line — and, per CLAUDE.md's
// never-silently-discard rule, a table with *only* ambiguous entries is
// still non-empty and still gets a line.
function summarizeTable(label: string, table: CountableTable): string | null {
  const parts: string[] = [];
  if (table.added.length > 0) parts.push(`${table.added.length} added`);
  if (table.removed.length > 0) parts.push(`${table.removed.length} removed`);
  if (table.changed.length > 0) parts.push(`${table.changed.length} changed`);
  if (table.ambiguous.length > 0) parts.push(`${table.ambiguous.length} ambiguous`);
  if (parts.length === 0) return null;
  return `${label}: ${parts.join(", ")}`;
}

// One summary line per non-empty entity table, across every installation
// in the report. Installations are prefixed with their id only when the
// report has more than one — the common case (one installation) then
// reads as plain "Devices: 1 added" rather than "Installation 0:
// Devices: 1 added".
function summaryLines(report: ProjectDiffReport): string[] {
  const lines: string[] = [];
  if (report.infoChanges.length > 0) {
    lines.push(`Project info: ${report.infoChanges.length} field(s) changed`);
  }
  const multiple = report.installations.length > 1;
  for (const installation of report.installations) {
    const prefix = multiple ? `Installation ${installation.id}: ` : "";
    if (installation.status !== "matched") {
      lines.push(`${prefix}installation ${installation.status}`);
      continue;
    }
    if (installation.fieldChanges.length > 0) {
      lines.push(`${prefix}Installation info: ${installation.fieldChanges.length} field(s) changed`);
    }
    const tables: [string, CountableTable][] = [
      ["Areas", installation.areas],
      ["Lines", installation.lines],
      ["Devices", installation.devices],
      ["Group ranges", installation.groupRanges],
      ["Group addresses", installation.groupAddresses],
      ["Buildings", installation.buildings],
    ];
    for (const [label, table] of tables) {
      const line = summarizeTable(label, table);
      if (line) lines.push(`${prefix}${line}`);
    }
  }
  return lines;
}

// Self-contained result panel, closer in spirit to `LogPanel.tsx` than to
// `DocumentationExportButton.tsx`'s one-line toast: the comparison result
// is a list of grouped counts, not something a single summary string can
// carry. Owns its `./api`/`./filePicker` calls the same way
// `DocumentationExportButton` owns its own, but additionally owns the
// fetched `ProjectDiffReport` and an open/closed toggle for the panel.
export default function ProjectDiffPanel(props: {
  tree: ProjectTree | null;
  onError: (e: unknown) => void;
  onClearErrors: () => void;
}) {
  const { tree, onError, onClearErrors } = props;
  const [report, setReport] = useState<ProjectDiffReport | null>(null);
  const [open, setOpen] = useState(false);

  async function compare() {
    const path = await pickOpenPath(COMPARE_FILTER);
    if (!path) return;
    onClearErrors();
    try {
      const result = await api.diffProject(path);
      setReport(result);
      setOpen(true);
    } catch (e) {
      onError(e);
    }
  }

  const lines = report ? summaryLines(report) : [];

  return (
    <>
      <button onClick={compare} disabled={!tree}>
        Compare with…
      </button>
      {open && report && (
        <div className="project-diff-panel">
          <h2>Comparison result</h2>
          {lines.length === 0 ? (
            <p className="project-diff-panel-empty">No differences found.</p>
          ) : (
            <ul className="project-diff-panel-list">
              {lines.map((line, index) => (
                <li key={index}>{line}</li>
              ))}
            </ul>
          )}
          <button onClick={() => setOpen(false)}>Close</button>
        </div>
      )}
    </>
  );
}
