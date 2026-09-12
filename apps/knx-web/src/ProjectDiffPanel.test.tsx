// @vitest-environment happy-dom
import { act } from "react";
import { createRoot } from "react-dom/client";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { ProjectTree } from "./bindings/ProjectTree";
import type { ProjectDiffReport, EntityTable, DeviceTable } from "./api";
import { resetUiLanguageForTests, saveUiLanguage } from "./uiLanguage";

const apiMock = vi.hoisted(() => ({
  diffProject: vi.fn(),
}));

const filePickerMock = vi.hoisted(() => ({
  pickOpenPath: vi.fn(),
}));

vi.mock("./api", () => ({
  ...apiMock,
  errorMessage: (error: unknown) => (error instanceof Error ? error.message : String(error)),
}));

vi.mock("./filePicker", () => ({ ...filePickerMock }));

import ProjectDiffPanel from "./ProjectDiffPanel";

let host: HTMLDivElement | undefined;

afterEach(() => {
  host?.remove();
  host = undefined;
  vi.clearAllMocks();
  window.localStorage.removeItem("knx-desktop:ui-language");
  resetUiLanguageForTests();
});

const fakeTree = { installations: [] } as unknown as ProjectTree;

function emptyTable<K, F>(): EntityTable<K, F> {
  return { added: [], removed: [], changed: [], ambiguous: [] };
}

function emptyDeviceTable(): DeviceTable {
  return { added: [], removed: [], changed: [], ambiguous: [] };
}

const emptyReport: ProjectDiffReport = {
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
          },
          {
            key: { etsId: "ga2", address: "1/1/2" },
            matchedBy: "etsId",
            left: { name: "A", central: false, unfiltered: false, range: null },
            right: { name: "B", central: false, unfiltered: false, range: null },
            changedFields: ["name"],
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

  it("passes the picked path with the 'KNXBench project' filter", async () => {
    filePickerMock.pickOpenPath.mockResolvedValueOnce("/data/compare.knxdb");
    apiMock.diffProject.mockResolvedValueOnce(emptyReport);
    const { root } = await renderPanel();
    await click(compareButton());
    expect(filePickerMock.pickOpenPath).toHaveBeenCalledWith([
      { name: "KNXBench project", extensions: ["knxdb"] },
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
    saveUiLanguage(window.localStorage, "de");
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
