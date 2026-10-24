/** Diagnostics-tab entry: both dialogs open preselected, close on change, never as menu items. */
// @vitest-environment happy-dom
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import type { DeviceDetail } from "./bindings/DeviceDetail";
import type { ProjectTree } from "./bindings/ProjectTree";
import { messages as en } from "./messages/en";
const api = vi.hoisted(() => ({ deviceParameters: vi.fn().mockResolvedValue({ programId: null, sections: [], stale: [], diagnostics: [] }) }));
vi.mock("./api", () => ({ ...api, errorMessage: String }));
const diagnostics = vi.hoisted(() => ({ previewDiagnostics: vi.fn(), startDiagnostics: vi.fn(), pollDiagnostics: vi.fn(), cancelDiagnostics: vi.fn(), saveDiagnostics: vi.fn() }));
vi.mock("./diagnosticApi", () => diagnostics);
const support = vi.hoisted(() => ({ previewSupport: vi.fn(), diagnoseSupport: vi.fn(), saveSupport: vi.fn() }));
vi.mock("./supportApi", () => support);
vi.mock("./features/rtf/client", async (importOriginal) => ({
  ...(await importOriginal<typeof import("./features/rtf/client")>()),
  createInstructionClient: () => ({ read: vi.fn(() => new Promise(() => {})), preview: vi.fn(), apply: vi.fn(), cancel: vi.fn().mockResolvedValue(undefined) }),
}));
import { DeviceWorkspace } from "./Inspector";
import DeviceDiagnosticsEntry from "./DeviceDiagnosticsEntry";
(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const device = (id: number, address: string) => ({ id, name: `Synthetic device ${id}`, address, description: null, com_object_count: 0 });
const project: ProjectTree = {
  schema_version: 11, errors: 0, warnings: 0, can_undo: false, can_redo: false, is_modified: false, group_address_style: "ThreeLevel",
  installations: [{ id: 1, name: "Installation", topology: [{ id: 2, name: "Area", address: 1, lines: [{ id: 3, name: "Line", address: 1,
    devices: [device(9, "1.1.9"), device(10, "1.1.10")] }] }], buildings: [], unassigned: [], group_addresses: [], group_ranges: [] }],
  server_incarnation: "synthetic", project_incarnation: 1, snapshot_revision: 1,
};
const detail: DeviceDetail = { id: 9, name: "Synthetic device 9", address: "1.1.9", description: null, com_objects: [],
  product: { product_ref: null, program_ref: null, catalog: null, resolution: "NoReference" } };

let host: HTMLDivElement; let root: Root;
beforeEach(() => { host = document.createElement("div"); document.body.append(host); root = createRoot(host); });
afterEach(async () => { await act(async () => root.unmount()); host.remove(); document.body.innerHTML = ""; vi.clearAllMocks(); });
const analyse = () => host.querySelector<HTMLButtonElement>("[data-device-analyse]");
const report = () => host.querySelector<HTMLButtonElement>("[data-device-report]");
const diagnosticDialog = () => document.querySelector<HTMLElement>(".diagnostic-export");
const supportDialog = () => document.querySelector<HTMLElement>("[data-universal-support]");
function noApiCalls() {
  for (const mock of [...Object.values(diagnostics), ...Object.values(support)]) expect(mock).not.toHaveBeenCalled();
}

it("the Diagnostics tab offers both entries as plain buttons, not menu items", async () => {
  await act(async () => root.render(<DeviceWorkspace detail={detail} tree={project} onApplied={() => {}} />));
  const tabs = [...host.querySelectorAll<HTMLButtonElement>('[role="tab"]')];
  expect(tabs[3].textContent).toBe(en["parameters.diagnosticsTab"]);
  await act(async () => tabs[3].click());
  const panel = host.querySelector<HTMLElement>("#device-panel-9-3")!;
  expect(panel.hidden).toBe(false);
  const buttons = [panel.querySelector<HTMLButtonElement>("[data-device-analyse]"), panel.querySelector<HTMLButtonElement>("[data-device-report]")];
  expect(buttons.map((b) => b?.textContent)).toEqual([en["deviceDiagnostics.analyse"], en["deviceDiagnostics.report"]]);
  for (const button of buttons) {
    expect(button!.tagName).toBe("BUTTON");
    expect(button!.type).toBe("button");
    expect(button!.disabled).toBe(false);
    expect(button!.hasAttribute("role")).toBe(false);
    expect(button!.hasAttribute("data-file-menu-item")).toBe(false);
  }
  // The File-menu triggers of the reused components are not rendered here.
  expect(panel.querySelector("[data-diagnostics-open],[data-support-open],[role=menuitem]")).toBeNull();
  expect(panel.textContent).toContain(en["deviceDiagnostics.hint"]
    .replace("{analysis}", en["diagnostics.title"]).replace("{report}", en["support.title"]));
  noApiCalls();
});

it("Analyse device opens the existing diagnostic export dialog with this device preselected", async () => {
  await act(async () => root.render(<DeviceDiagnosticsEntry project={project} deviceId={9} />));
  await act(async () => analyse()!.click());
  const dialog = diagnosticDialog()!;
  expect(dialog.querySelector("h2")!.textContent).toBe(en["diagnostics.title"]);
  expect(dialog.querySelector<HTMLSelectElement>("[data-diagnostics-device]")!.value).toBe("9");
  expect(supportDialog()).toBeNull();
  noApiCalls();
});

it("Send report opens the existing support dialog with this device preselected", async () => {
  await act(async () => root.render(<DeviceDiagnosticsEntry project={project} deviceId={10} />));
  await act(async () => report()!.click());
  const dialog = supportDialog()!;
  expect(dialog.querySelector("h2")!.textContent).toBe(en["support.title"]);
  expect(dialog.querySelector<HTMLSelectElement>("[data-support-device]")!.value).toBe("10");
  expect(diagnosticDialog()).toBeNull();
  noApiCalls();
});

it("an equal refresh keeps an open dialog; a device change or new revision closes the report", async () => {
  await act(async () => root.render(<DeviceDiagnosticsEntry project={project} deviceId={9} />));
  await act(async () => report()!.click());
  await act(async () => root.render(<DeviceDiagnosticsEntry project={{ ...project }} deviceId={9} />));
  expect(supportDialog()).not.toBeNull();
  await act(async () => root.render(<DeviceDiagnosticsEntry project={project} deviceId={10} />));
  expect(supportDialog()).toBeNull();
  await act(async () => report()!.click());
  await act(async () => root.render(<DeviceDiagnosticsEntry project={{ ...project, snapshot_revision: 2 }} deviceId={10} />));
  expect(supportDialog()).toBeNull();
});

it("a device change or another project closes the analysis; a revision keeps its stale review", async () => {
  await act(async () => root.render(<DeviceDiagnosticsEntry project={project} deviceId={9} />));
  await act(async () => analyse()!.click());
  await act(async () => root.render(<DeviceDiagnosticsEntry project={{ ...project, snapshot_revision: 2 }} deviceId={9} />));
  expect(diagnosticDialog()).not.toBeNull();
  await act(async () => root.render(<DeviceDiagnosticsEntry project={{ ...project, snapshot_revision: 2 }} deviceId={10} />));
  expect(diagnosticDialog()).toBeNull();
  await act(async () => analyse()!.click());
  await act(async () => root.render(<DeviceDiagnosticsEntry project={{ ...project, project_incarnation: 2 }} deviceId={10} />));
  expect(diagnosticDialog()).toBeNull();
});

it("both entries are disabled without a project and open nothing", async () => {
  await act(async () => root.render(<DeviceDiagnosticsEntry project={null} deviceId={9} />));
  expect(analyse()!.disabled).toBe(true);
  expect(report()!.disabled).toBe(true);
  await act(async () => { analyse()!.click(); report()!.click(); });
  expect(diagnosticDialog()).toBeNull();
  expect(supportDialog()).toBeNull();
  noApiCalls();
});
