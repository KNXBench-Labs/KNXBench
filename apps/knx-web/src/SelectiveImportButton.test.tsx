/** Exercises explicit selective-import preview, consent and stale UI responses. */
// @vitest-environment happy-dom
import { act } from "react";
import { createRoot } from "react-dom/client";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
beforeEach(() => Object.assign(globalThis, { IS_REACT_ACT_ENVIRONMENT: true }));
import type { ProjectTree } from "./bindings/ProjectTree";
const api = vi.hoisted(() => ({ inspectImportSource: vi.fn(), previewImportSelection: vi.fn(), applyImportSelection: vi.fn(), currentProject: vi.fn() }));
const picker = vi.hoisted(() => ({ pickOpenPath: vi.fn() }));
vi.mock("./api", () => api);
vi.mock("./filePicker", () => picker);
import SelectiveImportButton from "./SelectiveImportButton";
import { resetSettingsForTests, settingsStorage } from "./settingsStore";
import { resetUiLanguageForTests } from "./uiLanguage";
let root: ReturnType<typeof createRoot> | undefined;
let host: HTMLDivElement | undefined;
const update = vi.fn();
const tree: ProjectTree = { schema_version: 11, errors: 0, warnings: 0, can_undo: false, can_redo: false, is_modified: false,
  server_incarnation: "test-server", snapshot_revision: 1, group_address_style: "ThreeLevel",
  installations: [{ id: 0, name: "Destination", topology: [], buildings: [], unassigned: [], group_addresses: [], group_ranges: [] }] };
const source = { sourceHash: "a".repeat(64), sourceReport: { unknown: [{ name: "Source-only extension" }] }, installations: [{ id: 0, name: "Source",
  lines: [{ id: 3, name: "Source line", address: "1.1", deviceCount: 1 }], devices: [{ id: 1, name: "Imported switch", address: "1.1.1", line: 3 }] }] };
const preview = { confirmationToken: "reviewed-token", preview: { sourceHash: source.sourceHash,
  selection: { sourceInstallation: 0, targetInstallation: 0, devices: [1], lines: [] },
  counts: { devices: 1, lines: 1, communicationObjects: 2, parameters: 3, modules: 0, groupAddresses: 1, buildingParts: 0 },
  mappings: [{ kind: "device", source: 1, target: 9, reused: false }], sourceReport: source.sourceReport,
  retainedSourceMayContainUnselectedData: true, retainedSourceEntries: 5, notes: [] } };
afterEach(async () => { if (root) await act(() => root!.unmount()); host?.remove(); root = undefined; host = undefined; vi.clearAllMocks(); update.mockReset(); resetSettingsForTests(); resetUiLanguageForTests(); });
function button(label: string) { return [...document.querySelectorAll("button")].find(b => b.textContent === label)!; }
async function click(label: string) { await act(async () => button(label).click()); }
async function render(value: ProjectTree | null = tree) {
  if (!host) { host = document.createElement("div"); document.body.appendChild(host); root = createRoot(host); }
  await act(() => root!.render(<SelectiveImportButton tree={value} scope="project-A" onTreeUpdate={update} />));
}
async function select() {
  picker.pickOpenPath.mockResolvedValue("source.knxproj"); api.inspectImportSource.mockResolvedValue(source); api.previewImportSelection.mockResolvedValue(preview);
  await render(); await click("Import selected lines/devices…"); await click("Choose source project…");
  await act(() => document.querySelector<HTMLInputElement>('[data-import-device="1"]')!.click());
}
it.each([
  ["en", "Import selected lines/devices…", "Choose source project…", "Unnamed installation (ID 7)", "Unnamed installation (ID 9)"],
  ["de", "Ausgewählte Linien/Geräte importieren…", "Quellprojekt auswählen…", "Unbenannte Installation (ID 7)", "Unbenannte Installation (ID 9)"],
])("labels unnamed installations without changing their identity or source data (%s)", async (language, entry, choose, sourceLabel, targetLabel) => {
  settingsStorage.setItem("uiLanguage", language);
  resetUiLanguageForTests();
  const unnamedSource = { ...source, installations: [{ ...source.installations[0], id: 7, name: "" }] };
  const unnamedTarget: ProjectTree = { ...tree, installations: [{ ...tree.installations[0], id: 9, name: "  " }] };
  picker.pickOpenPath.mockResolvedValue("source.knxproj");
  api.inspectImportSource.mockResolvedValue(unnamedSource);
  api.previewImportSelection.mockResolvedValue(preview);
  await render(unnamedTarget); await click(entry); await click(choose);
  const selects = [...document.querySelectorAll("select")];
  expect(selects.map(s => s.selectedOptions[0].textContent)).toEqual([sourceLabel, targetLabel]);
  expect(selects.map(s => s.value)).toEqual(["7", "9"]);
  await act(() => document.querySelector<HTMLInputElement>('[data-import-device="1"]')!.click());
  await click(language === "en" ? "Preview selection" : "Auswahl prüfen");
  expect(api.previewImportSelection.mock.calls[0][0].selection).toEqual({ sourceInstallation: 7, targetInstallation: 9, devices: [1], lines: [] });
  expect(unnamedSource.installations[0].name).toBe("");
  expect(unnamedTarget.installations[0].name).toBe("  ");
});
it("requires an open project, a selection, a preview and explicit source-retention consent", async () => {
  await render(null); expect(button("Import selected lines/devices…").disabled).toBe(true);
  await select(); expect(api.applyImportSelection).not.toHaveBeenCalled();
  await click("Preview selection"); expect(document.body.textContent).toContain("Source-only extension");
  expect(button("Import selection").disabled).toBe(true);
  await act(() => document.querySelector<HTMLInputElement>('[data-import-consent]')!.click());
  api.applyImportSelection.mockResolvedValue({ project: { ...tree, can_undo: true, snapshot_revision: 2 }, preview: preview.preview });
  await click("Import selection"); expect(api.applyImportSelection).toHaveBeenCalledTimes(1);
  expect(api.applyImportSelection.mock.calls[0][0].confirmationToken).toBe("reviewed-token");
  expect(update).toHaveBeenCalledTimes(1); expect(document.body.textContent).toContain("Selection imported");
});
it("changing the selection invalidates both preview and consent", async () => {
  await select(); await click("Preview selection");
  await act(() => document.querySelector<HTMLInputElement>('[data-import-consent]')!.click());
  await act(() => document.querySelector<HTMLInputElement>('[data-import-line="3"]')!.click());
  expect(document.querySelector('[data-import-consent]')).toBeNull();
  expect(api.applyImportSelection).not.toHaveBeenCalled();
});
it("ignores a delayed preview after the destination revision changes", async () => {
  await select(); let finish!: (v: unknown) => void;
  api.previewImportSelection.mockImplementationOnce(() => new Promise(resolve => { finish = resolve; }));
  await click("Preview selection"); await render({ ...tree, snapshot_revision: 2 });
  await act(async () => finish(preview));
  expect(document.querySelector('[data-import-consent]')).toBeNull(); expect(api.applyImportSelection).not.toHaveBeenCalled();
});
it("reports refusal without replacing the open project", async () => {
  await select(); api.previewImportSelection.mockRejectedValueOnce(new Error("Individual address already exists"));
  await click("Preview selection"); expect(document.querySelector('[role=alert]')?.textContent).toContain("already exists");
  expect(update).not.toHaveBeenCalled(); expect(api.applyImportSelection).not.toHaveBeenCalled();
});

it("keeps the acknowledged success visible when the real parent accepts the returned revision", async () => {
  await select(); await click("Preview selection");
  await act(() => document.querySelector<HTMLInputElement>('[data-import-consent]')!.click());
  const appliedTree = { ...tree, can_undo: true, snapshot_revision: 2 };
  api.applyImportSelection.mockResolvedValue({ project: appliedTree, preview: preview.preview });
  update.mockImplementation(value => root!.render(<SelectiveImportButton tree={value} scope="project-A" onTreeUpdate={update} />));
  await click("Import selection");
  expect(document.body.textContent).toContain("Selection imported");
  expect(api.applyImportSelection).toHaveBeenCalledTimes(1);
});
