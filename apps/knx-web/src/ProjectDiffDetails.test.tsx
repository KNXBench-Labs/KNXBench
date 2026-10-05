/** KL-60: ProjectDiffDetails remembers opened tables for one mount, and a remount starts closed. */
// @vitest-environment happy-dom
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, describe, expect, it } from "vitest";
import type { ProjectDiffReport } from "./api";
import ProjectDiffDetails from "./ProjectDiffDetails";

let host: HTMLDivElement | undefined;
let root: Root | undefined;

afterEach(async () => {
  if (root) await act(async () => root!.unmount());
  root = undefined;
  host?.remove();
  host = undefined;
});

const empty = { added: [], removed: [], changed: [], ambiguous: [] };

function report(): ProjectDiffReport {
  return {
    inputKind: "knxdb",
    importReport: null,
    importDiagnostics: [],
    infoChanges: [],
    installations: [
      {
        id: 0,
        status: "matched",
        fieldChanges: [],
        areas: empty,
        lines: empty,
        devices: empty,
        groupRanges: empty,
        buildings: empty,
        groupAddresses: {
          ...empty,
          added: [[{ etsId: null, address: "1/1/1" }, { name: "Light", central: false, unfiltered: false, range: null }]],
        },
      },
    ],
  };
}

const toggle = () => host!.querySelector<HTMLButtonElement>("button.project-diff-toggle")!;

describe("ProjectDiffDetails table memory", () => {
  it("keeps a table open across re-renders, and a remount (a new comparison) starts collapsed", async () => {
    host = document.createElement("div");
    document.body.appendChild(host);
    root = createRoot(host);
    const first = report();
    await act(async () => root!.render(<ProjectDiffDetails key={1} report={first} />));
    await act(async () => toggle().click());
    expect(toggle().getAttribute("aria-expanded")).toBe("true");
    await act(async () => root!.render(<ProjectDiffDetails key={1} report={first} />));
    expect(toggle().getAttribute("aria-expanded")).toBe("true");
    // ProjectDiffPanel gives every comparison a new key.
    await act(async () => root!.render(<ProjectDiffDetails key={2} report={report()} />));
    expect(toggle().getAttribute("aria-expanded")).toBe("false");
  });
});
