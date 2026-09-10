import { pickOpenPath, pickSavePath } from "./filePicker";
import * as api from "./api";
import type { ProjectTree } from "./bindings/ProjectTree";

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
const CSV_FILTER = [{ name: "Group-address CSV", extensions: ["csv"] }];

export default function GroupAddressCsvButtons(props: {
  tree: ProjectTree | null;
  onTreeUpdate: (tree: ProjectTree) => void;
  onSummary: (message: string) => void;
  onError: (e: unknown) => void;
}) {
  const { tree, onTreeUpdate, onSummary, onError } = props;

  async function exportCsv() {
    const path = await pickSavePath(CSV_FILTER, "group-addresses.csv");
    if (!path) return;
    try {
      const { warnings } = await api.exportGroupAddressesCsv(path);
      const n = warnings.length;
      onSummary(
        n === 0
          ? "Group addresses exported to CSV, no warnings."
          : `Group addresses exported to CSV, ${n} warning${n === 1 ? "" : "s"} — see Log.`,
      );
    } catch (e) {
      onError(e);
    }
  }

  async function importCsv() {
    const path = await pickOpenPath(CSV_FILTER);
    if (!path) return;
    try {
      const { tree: nextTree, report } = await api.importGroupAddressesCsv(path);
      onTreeUpdate(nextTree);
      onSummary(
        `Group addresses imported from CSV: ${report.created} created, ${report.updated} updated, ` +
          `${report.unchanged} unchanged.`,
      );
    } catch (e) {
      // A rejected import (400 — a row-level problem) never reaches the
      // `.then` above: `onTreeUpdate` is not called, and the project the
      // rest of the app sees stays exactly as it was.
      onError(e);
    }
  }

  return (
    <>
      <button onClick={exportCsv} disabled={!tree}>
        Export group addresses (CSV)…
      </button>
      <button onClick={importCsv} disabled={!tree}>
        Import group addresses (CSV)…
      </button>
    </>
  );
}
