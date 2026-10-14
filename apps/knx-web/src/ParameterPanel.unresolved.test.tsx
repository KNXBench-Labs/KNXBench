/** Tests the Parameters tab's unresolved-program notice and restore action (#21). */
// @vitest-environment happy-dom
import { act, useState } from "react";
import { createRoot } from "react-dom/client";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { ManufacturerFileRestore, ParameterPanel as ParameterPanelDto } from "./api";
import type { ProjectTree } from "./bindings/ProjectTree";
import { resetProductLanguageForTests } from "./productLanguage";
import { resetSettingsForTests, setSetting } from "./settingsStore";
import { UI_LANGUAGE_STORAGE_KEY, resetUiLanguageForTests } from "./uiLanguage";

const apiMock = vi.hoisted(() => ({
  deviceParameters: vi.fn(),
  setParameterValue: vi.fn(),
  restoreManufacturerFiles: vi.fn(),
  currentProject: vi.fn(),
}));
const pickerMock = vi.hoisted(() => ({ pickOpenPath: vi.fn() }));

vi.mock("./api", () => ({
  ...apiMock,
  errorMessage: (error: unknown) => (error instanceof Error ? error.message : String(error)),
}));
vi.mock("./filePicker", () => pickerMock);

import ParameterPanel from "./ParameterPanel";

const tree = (revision: number): ProjectTree => ({
  schema_version: 16, errors: 0, warnings: 0, can_undo: false, can_redo: false, is_modified: false,
  group_address_style: "ThreeLevel", installations: [],
  server_incarnation: "srv", project_incarnation: 1, snapshot_revision: revision,
});

const unresolved = (over: Partial<NonNullable<ParameterPanelDto["unresolvedProgram"]>> = {}): ParameterPanelDto => ({
  programId: null, sourceLanguage: null, sections: [], stale: [], diagnostics: [], tree: null,
  unresolvedProgram: { programRef: "M-0001_H-1_HP-1", productDatabase: true, missingFiles: 2, ...over },
});

const resolved: ParameterPanelDto = {
  programId: "M-0001_A-1", sourceLanguage: null, stale: [], diagnostics: [], tree: null, unresolvedProgram: null,
  sections: [{
    scope: null,
    fields: [{
      etsId: "P-1", name: "Delay", nameLanguage: null, text: null, textLanguage: null, kind: "Number",
      value: "5", valueSource: "Stored", editable: true, min: "0", max: "255", enumOptions: [], displayOrder: 1,
      access: null, writeEtsId: "P-1",
    }],
  }],
};

const restoreResult = (over: Partial<ManufacturerFileRestore>): ManufacturerFileRestore => ({
  installed: [], alreadyInstalled: 0, mismatched: [], notInArchive: [], failed: [], stillMissing: 0, ...over,
});

let host: HTMLDivElement | undefined;

afterEach(() => {
  host?.remove();
  host = undefined;
  vi.clearAllMocks();
  resetSettingsForTests();
  resetProductLanguageForTests();
  resetUiLanguageForTests();
});

/** The panel as the Inspector uses it: a published tree re-reads it. */
function Harness() {
  const [shown, setShown] = useState<ProjectTree>(tree(1));
  return <ParameterPanel deviceId={1} refreshKey={shown} onValueApplied={setShown} />;
}

async function render() {
  host = document.createElement("div");
  document.body.appendChild(host);
  const root = createRoot(host);
  await act(async () => {
    root.render(<Harness />);
  });
  return root;
}

async function click(selector: string) {
  await act(async () => {
    host!.querySelector<HTMLButtonElement>(selector)!.click();
  });
}

describe("unresolved application program", () => {
  it("restores the missing files, reloads, and keeps the summary visible", async () => {
    apiMock.deviceParameters.mockResolvedValueOnce(unresolved()).mockResolvedValue(resolved);
    apiMock.currentProject.mockResolvedValue(tree(1));
    pickerMock.pickOpenPath.mockResolvedValue("/data/original.knxproj");
    apiMock.restoreManufacturerFiles.mockResolvedValue(
      restoreResult({ installed: ["M-0001/Hardware.xml", "M-0001/M-0001_A-1.xml"] }));
    const root = await render();

    expect(host!.textContent).toContain("2 manufacturer files are missing from the product database");
    expect(host!.querySelector("code")!.textContent).toBe("M-0001_H-1_HP-1");
    await click(".unresolved-program-restore");

    expect(pickerMock.pickOpenPath).toHaveBeenCalledWith([{ name: expect.any(String), extensions: ["knxproj"] }]);
    expect(apiMock.restoreManufacturerFiles).toHaveBeenCalledWith("/data/original.knxproj");
    expect(apiMock.deviceParameters).toHaveBeenCalledTimes(2);
    expect(host!.querySelector(".unresolved-program")).toBeNull();
    expect(host!.querySelectorAll(".parameter-field").length).toBe(1);
    expect(host!.querySelector('[role="status"]')!.textContent).toBe(
      "Product data restored: 2 files installed. Save the project so its file contains them from now on.");
    root.unmount();
  });

  it("does nothing when the file picker is cancelled", async () => {
    apiMock.deviceParameters.mockResolvedValue(unresolved());
    pickerMock.pickOpenPath.mockResolvedValue(null);
    const root = await render();
    await click(".unresolved-program-restore");
    expect(apiMock.restoreManufacturerFiles).not.toHaveBeenCalled();
    expect(host!.querySelector('[role="status"]')).toBeNull();
    root.unmount();
  });

  it("names files with other bytes and does not reload when nothing was installed", async () => {
    apiMock.deviceParameters.mockResolvedValue(unresolved({ missingFiles: 1 }));
    pickerMock.pickOpenPath.mockResolvedValue("/data/other.knxproj");
    apiMock.restoreManufacturerFiles.mockResolvedValue(
      restoreResult({ mismatched: ["M-0001/M-0001_A-1.xml"], stillMissing: 1 }));
    const root = await render();
    expect(host!.textContent).toContain("1 manufacturer file is missing");
    await click(".unresolved-program-restore");

    expect(apiMock.currentProject).not.toHaveBeenCalled();
    expect(apiMock.deviceParameters).toHaveBeenCalledTimes(1);
    const summary = host!.querySelector('[role="status"]')!.textContent!;
    expect(summary).toContain("No file from this archive was installed.");
    expect(summary).toContain("1 file is still missing.");
    expect(summary).toContain("differ from the files this project was made with: M-0001/M-0001_A-1.xml");
    root.unmount();
  });

  it("shows a refused restore as an error", async () => {
    apiMock.deviceParameters.mockResolvedValue(unresolved());
    pickerMock.pickOpenPath.mockResolvedValue("/data/notes.txt");
    apiMock.restoreManufacturerFiles.mockRejectedValue(new Error("not a ZIP archive"));
    const root = await render();
    await click(".unresolved-program-restore");
    expect(host!.querySelector('[role="alert"]')!.textContent).toBe("not a ZIP archive");
    root.unmount();
  });

  it.each([
    [{ productDatabase: false, missingFiles: 0 }, "This server has no product database"],
    [{ productDatabase: true, missingFiles: 0 }, "Install the device's product data (.knxprod) from the catalogue."],
  ])("offers no restore when the project's files are not what is missing (%o)", async (over, text) => {
    apiMock.deviceParameters.mockResolvedValue(unresolved(over));
    const root = await render();
    expect(host!.textContent).toContain(text);
    expect(host!.querySelector(".unresolved-program-restore")).toBeNull();
    root.unmount();
  });

  it("speaks German with du", async () => {
    setSetting(UI_LANGUAGE_STORAGE_KEY, "de");
    apiMock.deviceParameters.mockResolvedValue(unresolved());
    const root = await render();
    expect(host!.textContent).toContain("2 Herstellerdateien fehlen in der Produktdatenbank");
    expect(host!.textContent).toContain("Stelle sie aus der ursprünglichen .knxproj");
    expect(host!.querySelector(".unresolved-program-restore")!.textContent)
      .toBe("Aus ursprünglicher .knxproj wiederherstellen…");
    root.unmount();
  });

  it("keeps the old sentence for a server that sends no explanation", async () => {
    const { unresolvedProgram: _omitted, ...older } = unresolved();
    apiMock.deviceParameters.mockResolvedValue(older);
    const root = await render();
    expect(host!.textContent).toContain("This device has no resolvable application program");
    expect(host!.querySelector(".unresolved-program")).toBeNull();
    root.unmount();
  });
});
