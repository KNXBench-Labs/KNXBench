/** Tests for ProjectDiffPanel's diff request flow, grouped counts and expandable entity details. */
// @vitest-environment happy-dom
import { act } from "react";
import { createRoot } from "react-dom/client";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { ProjectTree } from "./bindings/ProjectTree";
import type {
  ComparisonImport,
  DeviceTable,
  EntityTable,
  GroupAddressFields,
  GroupAddressKey,
  LogEntry,
  ProjectDiffReport,
} from "./api";
import { resetUiLanguageForTests, saveUiLanguage } from "./uiLanguage";

const apiMock = vi.hoisted(() => ({
  diffProject: vi.fn(),
}));

const filePickerMock = vi.hoisted(() => ({
  pickOpenPath: vi.fn(),
}));

vi.mock("./api", async () => {
  const actual = await vi.importActual<typeof import("./api")>("./api");
  return {
    ...apiMock,
    importRefusal: actual.importRefusal,
    errorMessage: (error: unknown) => (error instanceof Error ? error.message : String(error)),
  };
});

vi.mock("./filePicker", () => ({ ...filePickerMock }));

import ProjectDiffPanel from "./ProjectDiffPanel";
import { resetSettingsForTests, settingsStorage } from "./settingsStore";

let host: HTMLDivElement | undefined;

afterEach(() => {
  host?.remove();
  host = undefined;
  vi.clearAllMocks();
  resetSettingsForTests();
  resetUiLanguageForTests();
});

const fakeTree = { installations: [] } as unknown as ProjectTree;

function emptyTable<K, F>(): EntityTable<K, F> {
  return { added: [], removed: [], changed: [], ambiguous: [] };
}

function emptyDeviceTable(): DeviceTable {
  return { added: [], removed: [], changed: [], ambiguous: [] };
}

const knxdbInput: ComparisonImport = { inputKind: "knxdb", importReport: null, importDiagnostics: [] };

const emptyReport: ProjectDiffReport = {
  ...knxdbInput,
  infoChanges: [],
  installations: [
    {
      id: 0,
      status: "matched",
      fieldChanges: [],
      areas: emptyTable(),
      lines: emptyTable(),
      devices: emptyDeviceTable(),
      groupRanges: emptyTable(),
      groupAddresses: emptyTable(),
      buildings: emptyTable(),
    },
  ],
};

// One added device, two changed group addresses — everything else empty.
const changesReport: ProjectDiffReport = {
  ...knxdbInput,
  infoChanges: [],
  installations: [
    {
      id: 0,
      status: "matched",
      fieldChanges: [],
      areas: emptyTable(),
      lines: emptyTable(),
      devices: {
        added: [
          [
            { etsId: "d1", address: null },
            {
              name: "New actuator",
              description: null,
              address: null,
              productRef: "P",
              programRef: "H",
              commissioning: {
                completion: "Editing",
                individualAddressLoaded: false,
                applicationProgramLoaded: false,
                parametersLoaded: false,
                communicationPartLoaded: false,
                mediumConfigLoaded: false,
                lastModified: null,
                lastDownload: null,
                broken: false,
              },
              line: null,
              building: null,
            },
          ],
        ],
        removed: [],
        changed: [],
        ambiguous: [],
      },
      groupRanges: emptyTable(),
      groupAddresses: {
        added: [],
        removed: [],
        changed: [
          {
            key: { etsId: "ga1", address: "1/1/1" },
            matchedBy: "etsId",
            left: { name: "Old name", central: false, unfiltered: false, range: null },
            right: { name: "New name", central: false, unfiltered: false, range: null },
            changedFields: ["name"],
            fieldChanges: [{ field: "name", left: "Old name", right: "New name" }],
          },
          {
            key: { etsId: "ga2", address: "1/1/2" },
            matchedBy: "etsId",
            left: { name: "A", central: false, unfiltered: false, range: null },
            right: { name: "B", central: false, unfiltered: false, range: null },
            changedFields: ["name"],
            fieldChanges: [{ field: "name", left: "A", right: "B" }],
          },
        ],
        ambiguous: [],
      },
      buildings: emptyTable(),
    },
  ],
};

// One group address that could not be matched unambiguously — nothing
// else differs. Exercises the "ambiguous-only table still renders" rule
// (CLAUDE.md: never silently discard information).
const ambiguousOnlyReport: ProjectDiffReport = {
  ...knxdbInput,
  infoChanges: [],
  installations: [
    {
      id: 0,
      status: "matched",
      fieldChanges: [],
      areas: emptyTable(),
      lines: emptyTable(),
      devices: emptyDeviceTable(),
      groupRanges: emptyTable(),
      groupAddresses: {
        added: [],
        removed: [],
        changed: [],
        ambiguous: [{ key: { etsId: "ga1", address: "1/1/1" }, leftCandidates: 2, rightCandidates: 1 }],
      },
      buildings: emptyTable(),
    },
  ],
};

// Task 5: the "field(s) changed" sentences are genuinely pluralized
// (`.one`/`.other`), unlike the grouped entity-table counts above — this
// pair of reports exercises the singular and plural branch of both the
// project-info and installation-info sentences at once.
const singularFieldChangeReport: ProjectDiffReport = {
  ...knxdbInput,
  infoChanges: [{ field: "name", left: "Old", right: "New" }],
  installations: [
    {
      id: 0,
      status: "matched",
      fieldChanges: [{ field: "address", left: "1", right: "2" }],
      areas: emptyTable(),
      lines: emptyTable(),
      devices: emptyDeviceTable(),
      groupRanges: emptyTable(),
      groupAddresses: emptyTable(),
      buildings: emptyTable(),
    },
  ],
};

const pluralFieldChangeReport: ProjectDiffReport = {
  ...knxdbInput,
  infoChanges: [
    { field: "name", left: "Old", right: "New" },
    { field: "comment", left: "A", right: "B" },
  ],
  installations: [
    {
      id: 0,
      status: "matched",
      fieldChanges: [
        { field: "address", left: "1", right: "2" },
        { field: "medium", left: "TP", right: "IP" },
      ],
      areas: emptyTable(),
      lines: emptyTable(),
      devices: emptyDeviceTable(),
      groupRanges: emptyTable(),
      groupAddresses: emptyTable(),
      buildings: emptyTable(),
    },
  ],
};

async function renderPanel(tree: ProjectTree | null = fakeTree) {
  host = document.createElement("div");
  document.body.appendChild(host);
  const root = createRoot(host);
  const onError = vi.fn();
  const onClearErrors = vi.fn();
  await act(async () => {
    root.render(<ProjectDiffPanel tree={tree} onError={onError} onClearErrors={onClearErrors} />);
  });
  return { root, onError, onClearErrors };
}

function compareButton() {
  return Array.from(host!.querySelectorAll("button")).find((b) =>
    b.textContent?.startsWith("Compare with"),
  )! as HTMLButtonElement;
}

async function click(button: HTMLButtonElement) {
  await act(async () => {
    button.dispatchEvent(new MouseEvent("click", { bubbles: true }));
  });
}

describe("ProjectDiffPanel", () => {
  it("is disabled when no project is open", async () => {
    const { root } = await renderPanel(null);
    expect(compareButton().disabled).toBe(true);
    root.unmount();
  });

  it("does not call the API or any callback when the picker is cancelled, and the panel stays closed", async () => {
    filePickerMock.pickOpenPath.mockResolvedValueOnce(null);
    const { root, onError, onClearErrors } = await renderPanel();
    await click(compareButton());
    expect(apiMock.diffProject).not.toHaveBeenCalled();
    expect(onError).not.toHaveBeenCalled();
    expect(onClearErrors).not.toHaveBeenCalled();
    expect(host!.textContent).not.toContain("Comparison result");
    root.unmount();
  });

  it("offers .knxdb and .knxproj in the picker, both kinds together first", async () => {
    filePickerMock.pickOpenPath.mockResolvedValueOnce("/data/compare.knxdb");
    apiMock.diffProject.mockResolvedValueOnce(emptyReport);
    const { root } = await renderPanel();
    await click(compareButton());
    expect(filePickerMock.pickOpenPath).toHaveBeenCalledWith([
      { name: "KNXBench or ETS project", extensions: ["knxdb", "knxproj"] },
      { name: "KNXBench project", extensions: ["knxdb"] },
      { name: "ETS project export", extensions: ["knxproj"] },
    ]);
    expect(apiMock.diffProject).toHaveBeenCalledWith("/data/compare.knxdb");
    root.unmount();
  });

  it("renders a 'no differences found' message when every table is empty", async () => {
    filePickerMock.pickOpenPath.mockResolvedValueOnce("/data/compare.knxdb");
    apiMock.diffProject.mockResolvedValueOnce(emptyReport);
    const { root } = await renderPanel();
    await click(compareButton());
    expect(host!.textContent).toContain("No differences found.");
    root.unmount();
  });

  it("renders grouped counts for a diff with changes, naming each non-empty table", async () => {
    filePickerMock.pickOpenPath.mockResolvedValueOnce("/data/compare.knxdb");
    apiMock.diffProject.mockResolvedValueOnce(changesReport);
    const { root } = await renderPanel();
    await click(compareButton());
    expect(host!.textContent).toContain("Devices: 1 added");
    expect(host!.textContent).toContain("Group addresses: 2 changed");
    root.unmount();
  });

  it("renders a line for a table that contains only ambiguous entries", async () => {
    filePickerMock.pickOpenPath.mockResolvedValueOnce("/data/compare.knxdb");
    apiMock.diffProject.mockResolvedValueOnce(ambiguousOnlyReport);
    const { root } = await renderPanel();
    await click(compareButton());
    expect(host!.textContent).toContain("Group addresses: 1 ambiguous");
    root.unmount();
  });

  it("surfaces a rejected comparison through onError, and the panel stays closed", async () => {
    filePickerMock.pickOpenPath.mockResolvedValueOnce("/data/compare.knxdb");
    apiMock.diffProject.mockRejectedValueOnce(new Error("comparison file does not exist"));
    const { root, onError } = await renderPanel();
    await click(compareButton());
    expect(onError).toHaveBeenCalledTimes(1);
    expect(host!.textContent).not.toContain("Comparison result");
    root.unmount();
  });

  it("clears a prior error toast before running the comparison", async () => {
    // First run: a real prior error — `onError` fires exactly like
    // "surfaces a rejected comparison through onError" above, so there is
    // an actual error toast showing by the time the second run starts.
    filePickerMock.pickOpenPath.mockResolvedValueOnce("/data/broken.knxdb");
    apiMock.diffProject.mockRejectedValueOnce(new Error("comparison file does not exist"));
    const { root, onError, onClearErrors } = await renderPanel();
    await click(compareButton());
    expect(onError).toHaveBeenCalledTimes(1);

    // Second run: record the order `onClearErrors` and `api.diffProject`
    // fire in, not just that both fire — pins down "before", which the
    // test's name promises.
    const order: string[] = [];
    onClearErrors.mockImplementationOnce(() => {
      order.push("clear");
    });
    filePickerMock.pickOpenPath.mockResolvedValueOnce("/data/compare.knxdb");
    apiMock.diffProject.mockImplementationOnce(async () => {
      order.push("diff");
      return emptyReport;
    });

    await click(compareButton());

    expect(order).toEqual(["clear", "diff"]);
    root.unmount();
  });

  // Task 5: German plural agreement is not always shaped like English —
  // this pins both the singular and plural branch of the "field(s)
  // changed" sentences under the German catalogue, for both the
  // project-info line and the per-installation info line.
  it("renders both plural branches of the field-changed sentences correctly in German", async () => {
    saveUiLanguage(settingsStorage, "de");
    resetUiLanguageForTests();

    filePickerMock.pickOpenPath.mockResolvedValueOnce("/data/compare.knxdb");
    apiMock.diffProject.mockResolvedValueOnce(singularFieldChangeReport);
    const singular = await renderPanel();
    // The "Compare with…" label is itself translated in German
    // ("Vergleichen mit…"), so this locates the (only) button by role
    // rather than by an English-literal text match.
    await click(host!.querySelector("button")!);
    expect(host!.textContent).toContain("Projektinfo: 1 Feld geändert");
    expect(host!.textContent).toContain("Installationsinfo: 1 Feld geändert");
    singular.root.unmount();
    host!.remove();

    filePickerMock.pickOpenPath.mockResolvedValueOnce("/data/compare.knxdb");
    apiMock.diffProject.mockResolvedValueOnce(pluralFieldChangeReport);
    const plural = await renderPanel();
    await click(host!.querySelector("button")!);
    expect(host!.textContent).toContain("Projektinfo: 2 Felder geändert");
    expect(host!.textContent).toContain("Installationsinfo: 2 Felder geändert");
    plural.root.unmount();
  });
});

// CT-1: the expandable entity list below the grouped counts.

function toggle(label: string): HTMLButtonElement {
  const button = Array.from(host!.querySelectorAll<HTMLButtonElement>("button.project-diff-toggle")).find(
    (b) => b.textContent?.includes(label),
  );
  if (!button) throw new Error(`no disclosure labelled ${label}`);
  return button;
}

// happy-dom does not synthesize a native button's keyboard activation, so
// this does what a browser does: Enter or Space on a focused `<button>`
// whose keydown was not cancelled activates it with a click.
async function pressKey(element: HTMLElement, key: string) {
  await act(async () => {
    const event = new KeyboardEvent("keydown", { key, bubbles: true, cancelable: true });
    const proceed = element.dispatchEvent(event);
    if (proceed && element instanceof HTMLButtonElement && (key === "Enter" || key === " ")) {
      element.click();
    }
  });
}

async function openReport(report: ProjectDiffReport) {
  filePickerMock.pickOpenPath.mockResolvedValueOnce("/data/compare.knxdb");
  apiMock.diffProject.mockResolvedValueOnce(report);
  const rendered = await renderPanel();
  await click(host!.querySelector("button")!);
  return rendered;
}

function manyAddedGroupAddresses(count: number): ProjectDiffReport {
  const added: [GroupAddressKey, GroupAddressFields][] = Array.from({ length: count }, (_, i) => [
    { etsId: null, address: `1/1/${i}` },
    { name: `GA ${i}`, central: false, unfiltered: false, range: null },
  ]);
  return {
    ...knxdbInput,
    infoChanges: [],
    installations: [
      {
        ...emptyReport.installations[0],
        groupAddresses: { added, removed: [], changed: [], ambiguous: [] },
      },
    ],
  };
}

function entryItems(): HTMLLIElement[] {
  return Array.from(host!.querySelectorAll<HTMLLIElement>(".project-diff-entries > li"));
}

describe("ProjectDiffPanel entity details", () => {
  it("keeps every table collapsed until asked, then lists entities by natural key", async () => {
    const { root } = await openReport(changesReport);
    expect(toggle("Devices (1)").getAttribute("aria-expanded")).toBe("false");
    expect(host!.textContent).not.toContain("New actuator");

    await click(toggle("Devices (1)"));
    expect(toggle("Devices (1)").getAttribute("aria-expanded")).toBe("true");
    const [device] = entryItems();
    expect(device.textContent).toContain("added");
    expect(device.textContent).toContain("unaddressed (d1)");
    expect(device.textContent).toContain("New actuator");
    root.unmount();
  });

  it("lists each changed field as field, before and after", async () => {
    const { root } = await openReport(changesReport);
    await click(toggle("Group addresses (2)"));
    const [first, second] = entryItems();
    expect(first.textContent).toContain("changed");
    expect(first.textContent).toContain("1/1/1 (ga1)");
    expect(first.textContent).toContain("matched by ETS ID");
    const headers = Array.from(first.querySelectorAll("thead th")).map((th) => th.textContent);
    expect(headers).toEqual(["Field", "Before", "After"]);
    const cells = Array.from(first.querySelectorAll("tbody tr > *")).map((cell) => cell.textContent);
    expect(cells).toEqual(["name", "Old name", "New name"]);
    expect(second.textContent).toContain("1/1/2 (ga2)");
    root.unmount();
  });

  it("names ambiguous entries in words with their candidate counts", async () => {
    const { root } = await openReport(ambiguousOnlyReport);
    await click(toggle("Group addresses (1)"));
    const [entry] = entryItems();
    expect(entry.textContent).toContain("ambiguous");
    expect(entry.textContent).toContain("2 candidates before, 1 after");
    root.unmount();
  });

  it("shows project and installation info changes as before/after tables", async () => {
    const { root } = await openReport(singularFieldChangeReport);
    await click(toggle("Project info (1)"));
    await click(toggle("Installation info (1)"));
    const rows = Array.from(host!.querySelectorAll(".project-diff-fields tbody tr")).map((row) =>
      Array.from(row.children).map((cell) => cell.textContent),
    );
    expect(rows).toEqual([
      ["name", "Old", "New"],
      ["address", "1", "2"],
    ]);
    root.unmount();
  });

  it("nests a changed device's communication object changes under the device", async () => {
    const left = changesReport.installations[0].devices.added[0][1];
    const report: ProjectDiffReport = {
      ...knxdbInput,
      infoChanges: [],
      installations: [
        {
          ...emptyReport.installations[0],
          devices: {
            added: [],
            removed: [],
            ambiguous: [],
            changed: [
              {
                key: { etsId: "d1", address: "1.1.5" },
                matchedBy: "naturalKey",
                left,
                right: left,
                changedFields: [],
                fieldChanges: [],
                comObjects: {
                  added: [],
                  changed: [],
                  ambiguous: [],
                  removed: [
                    [
                      { device: { etsId: "d1", address: "1.1.5" }, number: 3 },
                      {
                        text: "Switch",
                        description: null,
                        dpt: null,
                        read: null,
                        write: null,
                        transmit: null,
                        update: null,
                        communication: null,
                        readOnInit: null,
                        links: [],
                        moduleInstance: null,
                      },
                    ],
                  ],
                },
                parameters: emptyTable(),
              },
            ],
          },
        },
      ],
    };
    const { root } = await openReport(report);
    await click(toggle("Devices (1)"));
    expect(host!.textContent).toContain("matched by natural key");
    expect(host!.textContent).not.toContain("Parameters (");
    await click(toggle("Communication objects (1)"));
    const nested = host!.querySelector(".project-diff-entries .project-diff-entries > li")!;
    expect(nested.textContent).toContain("removed");
    expect(nested.textContent).toContain("#3");
    expect(nested.textContent).toContain("Switch");
    root.unmount();
  });

  it("expands and collapses with the keyboard, and Escape still closes the report", async () => {
    const { root } = await openReport(changesReport);
    const button = toggle("Devices (1)");
    expect(button.tagName).toBe("BUTTON");
    expect(button.type).toBe("button");
    expect(button.tabIndex).toBe(0);
    button.focus();
    expect(document.activeElement).toBe(button);

    await pressKey(button, "Enter");
    expect(button.getAttribute("aria-expanded")).toBe("true");
    const controlled = document.getElementById(button.getAttribute("aria-controls")!);
    expect(controlled?.textContent).toContain("New actuator");

    await pressKey(button, " ");
    expect(button.getAttribute("aria-expanded")).toBe("false");
    expect(host!.textContent).not.toContain("New actuator");

    await pressKey(button, "Escape");
    expect(host!.textContent).not.toContain("Comparison result");
    root.unmount();
  });

  it("starts a new comparison collapsed even if the previous one was expanded", async () => {
    filePickerMock.pickOpenPath.mockResolvedValue("/data/compare.knxdb");
    apiMock.diffProject.mockResolvedValue(changesReport);
    const { root } = await renderPanel();
    await click(compareButton());
    await click(toggle("Devices (1)"));
    expect(toggle("Devices (1)").getAttribute("aria-expanded")).toBe("true");
    await click(compareButton());
    expect(toggle("Devices (1)").getAttribute("aria-expanded")).toBe("false");
    root.unmount();
  });

  it("renders the details in German", async () => {
    saveUiLanguage(settingsStorage, "de");
    resetUiLanguageForTests();
    const { root } = await openReport(changesReport);
    await click(toggle("Geräte (1)"));
    await click(toggle("Gruppenadressen (2)"));
    expect(host!.textContent).toContain("hinzugefügt");
    expect(host!.textContent).toContain("ohne Adresse (d1)");
    expect(host!.textContent).toContain("zugeordnet über ETS-ID");
    const headers = Array.from(host!.querySelectorAll(".project-diff-fields thead th")).map(
      (th) => th.textContent,
    );
    expect(headers.slice(0, 3)).toEqual(["Feld", "Vorher", "Nachher"]);
    root.unmount();
  });

  it("limits a large table to one page and reveals more on request", async () => {
    const { root } = await openReport(manyAddedGroupAddresses(3000));
    expect(host!.textContent).toContain("Group addresses: 3000 added");
    expect(entryItems()).toHaveLength(0);

    await click(toggle("Group addresses (3000)"));
    expect(entryItems()).toHaveLength(50);
    expect(host!.textContent).toContain("Showing 50 of 3000.");

    const more = Array.from(host!.querySelectorAll("button")).find((b) =>
      b.textContent?.startsWith("Show more"),
    )!;
    expect(more.textContent).toBe("Show more (50)");
    await click(more);
    expect(entryItems()).toHaveLength(100);
    expect(document.activeElement).toBe(entryItems()[50]);
    expect(entryItems()[50].textContent).toContain("1/1/50");
    root.unmount();
  });

  it("drops the show-more control after the last page and keeps focus on the list", async () => {
    const { root } = await openReport(manyAddedGroupAddresses(60));
    await click(toggle("Group addresses (60)"));
    const more = Array.from(host!.querySelectorAll("button")).find((b) =>
      b.textContent?.startsWith("Show more"),
    )!;
    expect(more.textContent).toBe("Show more (10)");
    await click(more);
    expect(entryItems()).toHaveLength(60);
    expect(host!.textContent).not.toContain("Show more");
    expect(document.activeElement).toBe(entryItems()[50]);
    root.unmount();
  });
});

// CT-6: a raw `.knxproj` comparison input and its import report.

function diagnostic(severity: LogEntry["severity"], message: string): LogEntry {
  return {
    timestamp: "2026-09-28T00:00:00Z",
    severity,
    source: `import:${severity}`,
    message,
    location: "/KNX/Project",
    detail: null,
  };
}

const knxprojDiagnostics: LogEntry[] = [
  diagnostic("warning", "unknown Attribute 'FancyNewAttr' seen 1 time(s) in P-0001/0.xml"),
  diagnostic("info", "ContainerEntry preserved opaque: signature"),
  diagnostic("info", "RetainedAttribute preserved opaque: unknown attribute"),
];

const knxprojReport: ProjectDiffReport = {
  ...changesReport,
  inputKind: "knxproj",
  importReport: { errors: [] },
  importDiagnostics: knxprojDiagnostics,
};

function refusedError(): Error {
  const error = new Error("comparison refused: the ETS import reported 1 error diagnostic(s)") as Error & {
    status: number;
    body: unknown;
  };
  error.status = 422;
  error.body = {
    error: error.message,
    inputKind: "knxproj",
    importReport: { errors: [{ severity: "Error" }] },
    importDiagnostics: [
      diagnostic("error", "DuplicateId { kind: \"GroupAddress\", id: \"P-0001-0_GA-1\" }"),
      diagnostic("info", "ContainerEntry preserved opaque: signature"),
    ],
  };
  return error;
}

function importBlock(): HTMLDetailsElement | null {
  return host!.querySelector<HTMLDetailsElement>("details.project-diff-import");
}

async function compareKnxproj(result: ProjectDiffReport | Error) {
  filePickerMock.pickOpenPath.mockResolvedValueOnce("uploads/export.knxproj");
  if (result instanceof Error) apiMock.diffProject.mockRejectedValueOnce(result);
  else apiMock.diffProject.mockResolvedValueOnce(result);
  const rendered = await renderPanel();
  await click(host!.querySelector("button")!);
  return rendered;
}

describe("ProjectDiffPanel import diagnostics", () => {
  it("shows a .knxproj's diagnostics collapsed above the diff, with the count in the summary", async () => {
    const { root } = await compareKnxproj(knxprojReport);
    const block = importBlock()!;
    expect(block.open).toBe(false);
    expect(block.querySelector("summary")!.textContent).toBe("ETS import report: 3 diagnostics (1 warning)");
    const items = Array.from(block.querySelectorAll("li"));
    expect(items).toHaveLength(3);
    expect(items[0].textContent).toContain("Warning");
    expect(items[0].textContent).toContain("FancyNewAttr");
    expect(items[0].textContent).toContain("/KNX/Project");
    // Above the diff: the block precedes the grouped counts in the panel.
    const list = host!.querySelector(".project-diff-panel-list")!;
    expect(block.compareDocumentPosition(list) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
    expect(host!.textContent).toContain("Devices: 1 added");
    root.unmount();
  });

  it("shows no import block for a .knxdb comparison", async () => {
    const { root } = await openReport(changesReport);
    expect(importBlock()).toBeNull();
    root.unmount();
  });

  it("uses the singular summary for one diagnostic", async () => {
    const { root } = await compareKnxproj({
      ...emptyReport,
      inputKind: "knxproj",
      importReport: {},
      importDiagnostics: [knxprojDiagnostics[1]],
    });
    expect(importBlock()!.querySelector("summary")!.textContent).toBe("ETS import report: 1 diagnostic");
    expect(host!.textContent).toContain("No differences found.");
    root.unmount();
  });

  it("shows a refused import's diagnostics in the panel instead of a diff, without an error toast", async () => {
    const { root, onError } = await compareKnxproj(refusedError());
    expect(onError).not.toHaveBeenCalled();
    expect(host!.querySelector(".project-diff-panel-refused")!.textContent).toContain("Comparison refused");
    expect(importBlock()!.querySelector("summary")!.textContent).toBe(
      "ETS import report: 2 diagnostics (1 error)",
    );
    expect(host!.textContent).toContain("DuplicateId");
    expect(host!.querySelector(".project-diff-panel-list")).toBeNull();
    expect(host!.textContent).not.toContain("No differences found.");
    root.unmount();
  });

  it("still routes a 422 without an import report through onError", async () => {
    const error = new Error("unprocessable") as Error & { status: number; body: unknown };
    error.status = 422;
    error.body = { error: "unprocessable" };
    const { root, onError } = await compareKnxproj(error);
    expect(onError).toHaveBeenCalledWith(error);
    expect(host!.textContent).not.toContain("Comparison result");
    root.unmount();
  });

  it("renders the picker filters, the summary and the refusal in German", async () => {
    saveUiLanguage(settingsStorage, "de");
    resetUiLanguageForTests();

    const accepted = await compareKnxproj(knxprojReport);
    expect(filePickerMock.pickOpenPath).toHaveBeenCalledWith([
      { name: "KNXBench- oder ETS-Projekt", extensions: ["knxdb", "knxproj"] },
      { name: "KNXBench-Projekt", extensions: ["knxdb"] },
      { name: "ETS-Projektexport", extensions: ["knxproj"] },
    ]);
    expect(importBlock()!.querySelector("summary")!.textContent).toBe(
      "ETS-Importbericht: 3 Meldungen (1 Warnung)",
    );
    expect(importBlock()!.querySelector("li")!.textContent).toContain("Warnung");
    accepted.root.unmount();
    host!.remove();

    const refused = await compareKnxproj(refusedError());
    expect(host!.textContent).toContain("Vergleich abgelehnt");
    expect(importBlock()!.querySelector("summary")!.textContent).toBe(
      "ETS-Importbericht: 2 Meldungen (1 Fehler)",
    );
    refused.root.unmount();
  });
});
