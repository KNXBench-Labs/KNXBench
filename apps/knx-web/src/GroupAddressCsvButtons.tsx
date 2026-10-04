/** Buttons for exporting/importing group addresses in the project's own CSV format, not ETS's. */
import { useState } from "react";
import { pickOpenPath, pickSavePath } from "./filePicker";
import * as api from "./api";
import type { ProjectTree } from "./bindings/ProjectTree";
import { useTranslate } from "./i18n";
import type { Translate } from "./i18n";

// T12 (GAP_ANALYSIS_ETS.md C2) — export/import of group addresses as
// "KNXBench group-address CSV v1" (crates/knx-csv, design
// docs/superpowers/specs/2026-09-10-csv-group-address-exchange-design.md).
// This is a format this project defines and owns; §1 of that design is
// explicit that no verified sample of ETS's own group-address CSV export
// exists anywhere in this repository, so nothing here may claim, imply, or
// be labelled as ETS compatibility — hence "CSV", never "ETS CSV".
//
// A standalone component (not folded into ProjectExplorer.tsx) so it stays
// directly testable, following the same precedent as CatalogBrowser.tsx and
// BulkActionToolbar.tsx: it owns its `./api`/`./filePicker` calls outright
// rather than taking them as props, and the toast/tree plumbing it does
// need comes in as two narrow callbacks from `App.tsx`, which already owns
// both.
//
// The toast is deliberately one line — a summary, not a report viewer. The
// full detail (every warning, every row problem, every ignored column) is
// already in the session log via `session_log::from_csv_import_report`,
// which both routes populate server-side; the Log tab a previous task built
// is where that detail lives, not here.
//
// The filter's `name` is built from `t()` inside the component, not as a
// module-level constant, so it follows the active UI language rather than
// freezing at whichever one was active on module load.
//
// Task 5 review, round 2: an earlier controller ruling had scoped this
// file to that one field only, on the theory that nothing else here was
// user-visible English worth chasing. That ruling was wrong — the two
// button labels and both toast summaries below were every bit as English
// and every bit as reachable, so the gap it left is closed here.

// A successful (200) import can still carry `problems` of severity
// `"warning"` and non-empty `ignoredColumns` — a column the file had that
// import never applies. Both are already logged server-side, but the
// export button's toast surfaces its warning count and the import button's
// summary was silently dropping the equivalent counts, which is exactly
// the "information never silently discarded" rule this project holds
// itself to. Folded into the one-line summary, matching export's own
// "— see Log." pointer, rather than reopening a report viewer here.
function importSummary(t: Translate, report: api.CsvImportReport): string {
  const base = t("groupAddressCsv.importSummaryBase", {
    created: report.created,
    updated: report.updated,
    readdressed: report.readdressed ?? 0,
    deleted: report.deleted ?? 0,
    unchanged: report.unchanged,
  });
  const warningCount = report.problems.filter((p) => p.severity === "warning").length;
  const ignoredCount = report.ignoredColumns.length;
  const extras: string[] = [];
  if (warningCount > 0) {
    extras.push(t("groupAddressCsv.importSummaryWarnings", { count: warningCount }));
  }
  if (ignoredCount > 0) {
    extras.push(t("groupAddressCsv.importSummaryIgnoredColumns", { count: ignoredCount }));
  }
  if (extras.length === 0) return `${base}.`;
  return `${base}, ${extras.join(", ")} ${t("groupAddressCsv.importSummarySeeLog")}`;
}

export default function GroupAddressCsvButtons(props: {
  tree: ProjectTree | null;
  onTreeUpdate: (tree: ProjectTree) => void;
  onSummary: (message: string) => void;
  onError: (e: unknown) => void;
  onClearErrors: () => void;
}) {
  const { tree, onTreeUpdate, onSummary, onError, onClearErrors } = props;
  const t = useTranslate();
  const csvFilter = [{ name: t("groupAddressCsv.filterName"), extensions: ["csv"] }];
  // MODEL-01: with several installations the user names the one the CSV
  // exchange acts on; with one, the calls stay exactly as before.
  const [chosen, setChosen] = useState<number | null>(null);
  const installations = tree?.installations ?? [];
  const target = installations.length > 1
    ? (installations.find((installation) => installation.id === chosen) ?? installations[0]).id
    : undefined;

  async function exportCsv() {
    const path = await pickSavePath(csvFilter, "group-addresses.csv");
    if (!path) return;
    // Sequenced exactly like `App.tsx`'s own save/import handlers: clear
    // any leftover error toast from an earlier, unrelated
    // failure before this operation runs, not after — so a stale error
    // never sits on screen through a subsequent success.
    onClearErrors();
    try {
      const { warnings } = target === undefined
        ? await api.exportGroupAddressesCsv(path)
        : await api.exportGroupAddressesCsv(path, target);
      const n = warnings.length;
      onSummary(
        n === 0
          ? t("groupAddressCsv.exportSummaryNone")
          : t("groupAddressCsv.exportSummaryWithWarnings", { count: n }),
      );
    } catch (e) {
      onError(e);
    }
  }

  async function importCsv() {
    const path = await pickOpenPath(csvFilter);
    if (!path) return;
    onClearErrors();
    try {
      let response = target === undefined
        ? await api.importGroupAddressesCsv(path)
        : await api.importGroupAddressesCsv(path, undefined, target);
      if (!response.applied && response.confirmationToken) {
        const affectedLinks = response.report.destructiveChanges.reduce(
          (count, change) => count + change.affectedLinks.length,
          0,
        );
        const detail = response.report.destructiveChanges
          .map((change) =>
            t("groupAddressCsv.confirmDestructiveDetail", {
              action: t(
                change.action === "readdress"
                  ? "groupAddressCsv.actionReaddress"
                  : "groupAddressCsv.actionDelete",
              ),
              source: change.sourceAddress,
              target: change.targetAddress ?? "—",
              links:
                change.affectedLinks.length > 0
                  ? change.affectedLinks
                      .map(
                        (link) =>
                          `${link.comObject} (${t(
                            link.direction === "send"
                              ? "groupAddressCsv.directionSend"
                              : "groupAddressCsv.directionReceive",
                          )})`,
                      )
                      .join(", ")
                  : "—",
            }),
          )
          .join("\n");
        const confirmed = window.confirm(
          `${t("groupAddressCsv.confirmDestructive", {
              readdressed: response.report.readdressed,
              deleted: response.report.deleted,
              affectedLinks,
            })}\n\n${detail}`,
        );
        if (!confirmed) {
          onSummary(t("groupAddressCsv.confirmCancelled"));
          return;
        }
        response = target === undefined
          ? await api.importGroupAddressesCsv(path, response.confirmationToken)
          : await api.importGroupAddressesCsv(path, response.confirmationToken, target);
      }
      onTreeUpdate(response.tree);
      onSummary(importSummary(t, response.report));
    } catch (e) {
      // A rejected import (400 — a row-level problem) never reaches the
      // `.then` above: `onTreeUpdate` is not called, and the project the
      // rest of the app sees stays exactly as it was.
      onError(e);
    }
  }

  return (
    <>
      {target !== undefined && (
        <select aria-label={t("groupAddressCsv.installation")} value={target}
          onChange={(e) => setChosen(Number(e.target.value))}>
          {installations.map((installation) => (
            <option key={installation.id} value={installation.id}>{installation.name}</option>
          ))}
        </select>
      )}
      <button onClick={exportCsv} disabled={!tree}>
        {t("groupAddressCsv.exportButton")}
      </button>
      <button onClick={importCsv} disabled={!tree}>
        {t("groupAddressCsv.importButton")}
      </button>
    </>
  );
}
