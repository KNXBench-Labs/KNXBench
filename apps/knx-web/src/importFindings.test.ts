/** Import-count/log-entry matching and the status card's overall reading. */
import { describe, expect, it } from "vitest";
import { countImportFindings, importReportState, isImportFinding } from "./importFindings";

const at = (severity: "error" | "warning" | "info", source: string) => ({ severity, source });

describe("import findings in the session log", () => {
  it("matches every import-report source and nothing that merely mentions imports", () => {
    for (const source of ["import:parse", "import:unknown", "import:conflict", "import:unsupported", "import:opaque"]) {
      expect(isImportFinding({ source })).toBe(true);
    }
    // The success summary, CSV imports, saves and the log's own drop notice
    // are not import-report findings.
    for (const source of ["import", "csv-import:ignored-column", "save", "log", "ui:toast:error"]) {
      expect(isImportFinding({ source })).toBe(false);
    }
  });

  it("counts exactly the severity a dashboard counter counts", () => {
    const entries = [
      at("warning", "import:unknown"), at("warning", "import:conflict"), at("warning", "import:parse"),
      at("error", "import:parse"), at("info", "import:opaque"), at("warning", "csv-import:problem"),
      at("warning", "log"),
    ];
    expect(countImportFindings(entries, "warning")).toBe(3);
    expect(countImportFindings(entries, "error")).toBe(1);
    expect(countImportFindings([], "warning")).toBe(0);
  });
});

describe("import report state", () => {
  const tree = { errors: 0, warnings: 0 };
  it("puts errors before warnings and never calls missing evidence clean", () => {
    expect(importReportState({ ...tree, errors: 2, warnings: 7 })).toBe("errors");
    expect(importReportState({ ...tree, warnings: 7 })).toBe("warnings");
    expect(importReportState({ ...tree, import_report_available: true })).toBe("clean");
    expect(importReportState({ ...tree, import_report_available: false })).toBe("unavailable");
    // Counted current-session findings stay visible even without the report.
    expect(importReportState({ ...tree, warnings: 1, import_report_available: false })).toBe("warnings");
    // An older server that omits the flag is not reported as missing evidence.
    expect(importReportState(tree)).toBe("clean");
  });
});
