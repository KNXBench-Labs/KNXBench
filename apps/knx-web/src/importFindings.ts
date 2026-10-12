/** Pure rules linking project-status import counts to session-log entries. */
// The server counts a project's import findings in `ProjectTree.errors` /
// `ProjectTree.warnings` (`domain.rs`, `apply_report_counts`) and writes the
// same import report into the session log, one entry per finding, under an
// `import:*` source (`session_log.rs`, `from_import_report`): genuine
// errors at severity `error`; warning-severity items, unknown constructs,
// DPT conflicts and unsupported capabilities at severity `warning`.
// Opaque/observation notes are `info` and are counted by neither number.
//
// Nothing here assumes the two always agree: the log is capped and lives
// only for this server run, so callers compare and say so when they differ.
import type { LogEntry } from "./api";
import type { ProjectTree } from "./bindings/ProjectTree";

export type ImportFindingSeverity = "error" | "warning";

/** Source prefix of every entry produced from a project import report. */
export const IMPORT_FINDING_SOURCE_PREFIX = "import:";

export function isImportFinding(entry: Pick<LogEntry, "source">): boolean {
  return entry.source.startsWith(IMPORT_FINDING_SOURCE_PREFIX);
}

/** Import-report entries of one severity, i.e. what one counter counts. */
export function countImportFindings(entries: readonly Pick<LogEntry, "source" | "severity">[], severity: ImportFindingSeverity): number {
  return entries.filter((entry) => entry.severity === severity && isImportFinding(entry)).length;
}

/**
 * One overall reading of the project's import counters for the status card.
 * `unavailable` is only used when there is nothing counted *and* the server
 * says the historical report is missing: zero then means "not assessed",
 * never "clean". A response without the flag (older server) is not
 * treated as missing evidence.
 */
export type ImportReportState = "errors" | "warnings" | "clean" | "unavailable";

export function importReportState(tree: Pick<ProjectTree, "errors" | "warnings" | "import_report_available">): ImportReportState {
  if (tree.errors > 0) return "errors";
  if (tree.warnings > 0) return "warnings";
  return tree.import_report_available === false ? "unavailable" : "clean";
}
