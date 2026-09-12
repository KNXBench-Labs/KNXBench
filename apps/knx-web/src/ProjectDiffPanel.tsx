/** Panel comparing the open project against another file on disk and rendering the entity diff. */
import { useState } from "react";
import { pickOpenPath } from "./filePicker";
import * as api from "./api";
import type { ProjectDiffReport } from "./api";
import type { ProjectTree } from "./bindings/ProjectTree";
import { useTranslate } from "./i18n";
import type { Translate } from "./i18n";
import type { MessageKey } from "./messages/en";

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

// The grouped-count words ("added", "removed", "changed", "ambiguous") are
// invariant participles/adjectives in both English and German — no
// `.one`/`.other` declension needed here, unlike the two genuine
// "field(s) changed" sentences below.
const ENTITY_STATUS_KEYS: Record<keyof CountableTable, MessageKey> = {
  added: "projectDiff.entityStatus.added",
  removed: "projectDiff.entityStatus.removed",
  changed: "projectDiff.entityStatus.changed",
  ambiguous: "projectDiff.entityStatus.ambiguous",
};

type EntityTableKey = "areas" | "lines" | "devices" | "groupRanges" | "groupAddresses" | "buildings";

// `"buildings"` renders as "Buildings"/"Gebäude" — the literal word, even
// though the underlying data is actually building-*part* entities (a
// pre-existing naming quirk in `InstallationDiff`, not something a
// translation-only task should silently "fix").
const ENTITY_LABEL_KEYS: Record<EntityTableKey, MessageKey> = {
  areas: "projectDiff.entity.areas",
  lines: "projectDiff.entity.lines",
  devices: "projectDiff.entity.devices",
  groupRanges: "projectDiff.entity.groupRanges",
  groupAddresses: "projectDiff.entity.groupAddresses",
  buildings: "projectDiff.entity.buildings",
};

// `installation.status` is only ever `"added"` or `"removed"` by the time
// this is called (the `"matched"` case is handled by its caller before
// reaching here) — reuses the same status-word keys as the entity tables
// above, since both are the same two invariant participles.
const INSTALLATION_STATUS_KEYS: Record<"added" | "removed", MessageKey> = {
  added: "projectDiff.entityStatus.added",
  removed: "projectDiff.entityStatus.removed",
};

// Builds the one-line grouped-count summary for a non-empty table (design
// spec §5/§9: grouped counts only, no tree view, no inline before/after
// highlighting). Returns `null` for an empty table so callers can filter
// those out without rendering an empty line — and, per CLAUDE.md's
// never-silently-discard rule, a table with *only* ambiguous entries is
// still non-empty and still gets a line.
function summarizeTable(t: Translate, label: string, table: CountableTable): string | null {
  const parts: string[] = [];
  (Object.keys(ENTITY_STATUS_KEYS) as (keyof CountableTable)[]).forEach((status) => {
    if (table[status].length > 0) {
      parts.push(`${table[status].length} ${t(ENTITY_STATUS_KEYS[status])}`);
    }
  });
  if (parts.length === 0) return null;
  return `${label}: ${parts.join(", ")}`;
}

// One summary line per non-empty entity table, across every installation
// in the report. Installations are prefixed with their id only when the
// report has more than one — the common case (one installation) then
// reads as plain "Devices: 1 added" rather than "Installation 0:
// Devices: 1 added".
function summaryLines(t: Translate, report: ProjectDiffReport): string[] {
  const lines: string[] = [];
  if (report.infoChanges.length > 0) {
    lines.push(t("projectDiff.projectInfoChanged", { count: report.infoChanges.length }));
  }
  const multiple = report.installations.length > 1;
  for (const installation of report.installations) {
    const prefix = multiple ? t("projectDiff.installationPrefix", { id: installation.id }) : "";
    if (installation.status !== "matched") {
      lines.push(
        `${prefix}${t("projectDiff.installationStatusLine", {
          status: t(INSTALLATION_STATUS_KEYS[installation.status]),
        })}`,
      );
      continue;
    }
    if (installation.fieldChanges.length > 0) {
      lines.push(
        `${prefix}${t("projectDiff.installationInfoChanged", { count: installation.fieldChanges.length })}`,
      );
    }
    const tables: [EntityTableKey, CountableTable][] = [
      ["areas", installation.areas],
      ["lines", installation.lines],
      ["devices", installation.devices],
      ["groupRanges", installation.groupRanges],
      ["groupAddresses", installation.groupAddresses],
      ["buildings", installation.buildings],
    ];
    for (const [key, table] of tables) {
      const line = summarizeTable(t, t(ENTITY_LABEL_KEYS[key]), table);
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
  const t = useTranslate();
  const [report, setReport] = useState<ProjectDiffReport | null>(null);
  const [open, setOpen] = useState(false);

  async function compare() {
    // Not module-level (see the removed `COMPARE_FILTER` constant): the
    // filter name shown in the native file dialog must follow the active
    // UI language, so it is built fresh from `t()` on every click instead
    // of once at module load.
    const path = await pickOpenPath([
      { name: t("projectDiff.compareFilterName"), extensions: ["knxdb"] },
    ]);
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

  const lines = report ? summaryLines(t, report) : [];

  return (
    <>
      <button onClick={compare} disabled={!tree}>
        {t("projectDiff.compareButton")}
      </button>
      {open && report && (
        <div className="project-diff-panel">
          <h2>{t("projectDiff.title")}</h2>
          {lines.length === 0 ? (
            <p className="project-diff-panel-empty">{t("projectDiff.noDifferences")}</p>
          ) : (
            <ul className="project-diff-panel-list">
              {lines.map((line, index) => (
                <li key={index}>{line}</li>
              ))}
            </ul>
          )}
          <button onClick={() => setOpen(false)}>{t("projectDiff.close")}</button>
        </div>
      )}
    </>
  );
}
