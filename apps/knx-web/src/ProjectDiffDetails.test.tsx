/** KL-60: ProjectDiffDetails remembers opened tables for one mount, and a remount starts closed. */
// @vitest-environment happy-dom
import { tableColumnSmoke } from "./tableColumnSmoke.fixture";
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { ProjectDiffReport } from "./api";
import ProjectDiffDetails from "./ProjectDiffDetails";

Object.assign(globalThis, { IS_REACT_ACT_ENVIRONMENT: true });

let host: HTMLDivElement | undefined;
let root: Root | undefined;

afterEach(async () => {
  if (root) await act(async () => root!.unmount());
  root = undefined;
  host?.remove();
  host = undefined;
  vi.restoreAllMocks();
  window.localStorage.clear();
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
        functions: empty,
        groupAddresses: {
          ...empty,
          added: [[{ etsId: null, address: "1/1/1" }, { name: "Light", central: false, unfiltered: false, range: null }]],
        },
      },
    ],
  };
}

const toggle = () => host!.querySelector<HTMLButtonElement>("button.project-diff-toggle")!;

it("one shared field reset restores every changed entry's widths and measured session floors", async () => {
  vi.spyOn(HTMLElement.prototype, "clientWidth", "get").mockReturnValue(40);
  vi.spyOn(HTMLElement.prototype, "getBoundingClientRect").mockImplementation(function (this: HTMLElement) {
    return { width: this.hasAttribute("data-table-word-measure") ? this.textContent!.length * 10 : 0 } as DOMRect;
  });
  host = document.createElement("div"); document.body.append(host); root = createRoot(host);
  const comparison = (long: boolean) => {
    const result = report();
    result.installations[0].groupAddresses.added = [];
    result.installations[0].groupAddresses.changed = [1, 2, 3].map(id => ({
      key: { etsId: null, address: `1/1/${id}` }, matchedBy: "naturalKey",
      left: { name: "Old", central: false, unfiltered: false, range: null },
      right: { name: "New", central: false, unfiltered: false, range: null },
      changedFields: ["name"], fieldChanges: [{ field: long ? "M".repeat(20 + id) : "name", left: "Old", right: "New" }],
    }));
    return result;
  };
  await act(async () => root!.render(<ProjectDiffDetails report={comparison(true)} />));
  await act(async () => toggle().click());
  const tables = () => [...host!.querySelectorAll('[data-table-id="project-diff-fields"]')];
  expect(tables()).toHaveLength(3);
  const resets = () => host!.querySelectorAll<HTMLButtonElement>('.table-column-reset');
  expect(resets()).toHaveLength(1);
  expect(resets()[0].getAttribute("aria-label")).toBe("Reset column widths: Field changes");
  expect(tables().every(table => !table.querySelector(".table-column-reset"))).toBe(true);
  const widths = () => tables().map(table => table.querySelector("col")!.style.width);
  expect(widths()).toEqual(["210px", "220px", "230px"]);
  await act(async () => root!.render(<ProjectDiffDetails report={comparison(false)} />));
  expect(widths()).toEqual(["210px", "220px", "230px"]);
  const handle = tables()[1].querySelector<HTMLElement>('[role="separator"]')!;
  await act(async () => handle.dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowRight", bubbles: true })));
  expect(widths()).toEqual(["218px", "228px", "238px"]);
  await act(async () => handle.dispatchEvent(new MouseEvent("dblclick", { bubbles: true })));
  // A column reset still preserves each mount's measured maxima.
  expect(widths()).toEqual(["210px", "220px", "230px"]);
  await act(async () => handle.dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowRight", bubbles: true })));
  await act(async () => resets()[0].click());
  expect(widths()).toEqual(["50px", "50px", "50px"]);
  expect(JSON.parse(window.localStorage.getItem("knxbench:table-column-widths:v1:project-diff-fields")!))
    .toEqual({ field: 180, before: 240, after: 240 });
});

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

tableColumnSmoke([["project-diff-fields", 3]], async () => {
  host = document.createElement("div"); root = createRoot(host);
  await act(async () => root!.render(<ProjectDiffDetails report={{ ...report(), infoChanges: [{ field: "name", left: "Before", right: "After" }] }} />));
  await act(async () => toggle().click()); return { host };
});

it("omits the reset for added and removed entries without field changes", async () => {
  host = document.createElement("div"); document.body.append(host); root = createRoot(host);
  const value = report();
  value.installations[0].groupAddresses.removed = value.installations[0].groupAddresses.added;
  await act(async () => root!.render(<ProjectDiffDetails report={value} />));
  await act(async () => toggle().click());
  expect(host.querySelectorAll(".table-column-reset")).toHaveLength(0);
  expect(host.querySelectorAll('[data-table-id="project-diff-fields"]')).toHaveLength(0);
});

it.each(["project", "installation", "entry", "nested"])("one reset exists for %s field changes even while collapsed", async source => {
  host = document.createElement("div"); document.body.append(host); root = createRoot(host);
  const value = report();
  const changes = [{ field: "name", left: "Before", right: "After" }];
  if (source === "project") value.infoChanges = changes;
  if (source === "installation") value.installations[0].fieldChanges = changes;
  if (source === "entry") value.installations[0].groupAddresses.changed = [{
    key: { etsId: null, address: "1/1/2" }, matchedBy: "naturalKey", changedFields: ["name"], fieldChanges: changes,
    left: { name: "Before", central: false, unfiltered: false, range: null },
    right: { name: "After", central: false, unfiltered: false, range: null },
  }];
  if (source === "nested") {
    const key = { etsId: null, address: "1.1.1" };
    const device = { name: "Device", description: null, address: "1.1.1", productRef: "P", programRef: "A", line: null, building: null,
      commissioning: { completion: "Editing", individualAddressLoaded: false, applicationProgramLoaded: false, parametersLoaded: false,
        communicationPartLoaded: false, mediumConfigLoaded: false, lastModified: null, lastDownload: null, broken: false } };
    value.installations[0].devices = { ...empty, changed: [{ key, matchedBy: "naturalKey", left: device, right: device,
      changedFields: [], fieldChanges: [], comObjects: empty, parameters: { ...empty, changed: [{
        key: { device: key, etsId: "parameter-1" }, matchedBy: "naturalKey", changedFields: ["raw"],
        fieldChanges: changes, left: { raw: "Before" }, right: { raw: "After" },
      }] } }] };
  }
  await act(async () => root!.render(<ProjectDiffDetails report={value} />));
  expect(host.querySelectorAll(".table-column-reset")).toHaveLength(1);
});
