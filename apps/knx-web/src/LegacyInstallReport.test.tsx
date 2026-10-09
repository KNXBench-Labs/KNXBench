/** Tests that the legacy import report folds every diagnostic by kind, dropping none. */
// @vitest-environment happy-dom
import { act } from "react";
import { createRoot } from "react-dom/client";
import { afterEach, describe, expect, it } from "vitest";
import type { LegacyInstallReport } from "./api";
import { groupByKind, LegacyInstallReportView } from "./LegacyInstallReport";

let host: HTMLDivElement | undefined;
afterEach(() => {
  host?.remove();
  host = undefined;
});

const report = (diagnostics: LegacyInstallReport["diagnostics"]): LegacyInstallReport => ({
  payloadSha256: "p", originalSha256: "o", namespace: "LX0000AAAA", skipped: false, programs: ["A"],
  catalogItems: 1, parameters: 1, parameterRefs: 1, comObjectRefs: 0, translations: 0, diagnostics,
  password: "given", remembered: false, rememberProblem: null,
});

describe("LegacyInstallReportView", () => {
  it("groups diagnostics by kind in first-seen order and keeps every detail", () => {
    const diagnostics = [
      { kind: "unmapped-table", detail: "t1" }, { kind: "dangling-reference", detail: "d1" },
      { kind: "unmapped-table", detail: "t2" }, { kind: "unread-member", detail: "m1" },
    ];
    expect(groupByKind(diagnostics)).toEqual([
      ["unmapped-table", ["t1", "t2"]], ["dangling-reference", ["d1"]], ["unread-member", ["m1"]],
    ]);
  });

  it("renders one folded group per kind with its count, all details inside", async () => {
    const many = Array.from({ length: 1500 }, (_, i) => ({ kind: "dangling-reference", detail: `d${i}` }));
    host = document.createElement("div");
    document.body.appendChild(host);
    const root = createRoot(host);
    await act(async () => root.render(<LegacyInstallReportView report={report([...many, { kind: "unread-member", detail: "mask" }])} />));
    const groups = [...host.querySelectorAll("details.legacy-report-kind")];
    expect(groups.map((g) => g.querySelector("summary")!.textContent)).toEqual([
      "dangling-reference (1500)", "unread-member (1)",
    ]);
    expect(groups.every((g) => !(g as HTMLDetailsElement).open)).toBe(true);
    expect(host.querySelectorAll("details.legacy-report-kind li")).toHaveLength(1501);
    expect(host.textContent).toContain("1501 import notes");
    root.unmount();
  });
});
