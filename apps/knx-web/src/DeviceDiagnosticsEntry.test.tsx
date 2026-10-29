/** Diagnostics-tab entry: analysis and report open preselected, never as menu items. */
// @vitest-environment happy-dom
import { act, type ReactNode } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import type { DeviceDetail } from "./bindings/DeviceDetail";
import type { ProjectTree } from "./bindings/ProjectTree";
import type { DiagnosticView } from "./diagnosticApi";
import { messages as en } from "./messages/en";
const api = vi.hoisted(() => ({ deviceParameters: vi.fn().mockResolvedValue({ programId: null, sections: [], stale: [], diagnostics: [] }) }));
vi.mock("./api", () => ({ ...api, errorMessage: String }));
const diagnostics = vi.hoisted(() => ({ previewDiagnostics: vi.fn(), startDiagnostics: vi.fn(), pollDiagnostics: vi.fn(), cancelDiagnostics: vi.fn(), saveDiagnostics: vi.fn() }));
const bundle = vi.hoisted(() => ({ previewBundle: vi.fn(), sectionText: vi.fn(), exportBundle: vi.fn(), deleteRecordings: vi.fn() }));
vi.mock("./diagnosticBundleApi", async (importOriginal) => ({ ...(await importOriginal<typeof import("./diagnosticBundleApi")>()), ...bundle }));
vi.mock("./diagnosticApi", () => ({ IDENTITY_PROFILE: "generic-identity-v1", MEMORY_PROFILE: "system2-declared-segments-v1", DiagnosticRefusal: class extends Error {}, ...diagnostics }));
vi.mock("./features/rtf/client", async (importOriginal) => ({
  ...(await importOriginal<typeof import("./features/rtf/client")>()),
  createInstructionClient: () => ({ read: vi.fn(() => new Promise(() => {})), preview: vi.fn(), apply: vi.fn(), cancel: vi.fn().mockResolvedValue(undefined) }),
}));
import { DeviceWorkspace } from "./Inspector";
import DeviceDiagnosticsEntry from "./DeviceDiagnosticsEntry";
import { ReportProblemProvider } from "./ReportProblemEntry";
import type { ReportProblemRequest } from "./reportProblem";
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

const opened = vi.fn<(request: ReportProblemRequest) => void>();
/** The workbench's report host; opening is observed, the wizard itself is tested elsewhere. */
function Workbench({ children }: { children: ReactNode }) {
  return <ReportProblemProvider request={null} onOpen={opened} onClose={() => {}} tree={null} notifications={[]}>{children}</ReportProblemProvider>;
}
function Entry(props: { project: ProjectTree | null; deviceId: number }) {
  return <Workbench><DeviceDiagnosticsEntry {...props} /></Workbench>;
}

let host: HTMLDivElement; let root: Root;
beforeEach(() => {
  host = document.createElement("div"); document.body.append(host); root = createRoot(host);
  for (const mock of Object.values(diagnostics)) mock.mockReset();
});
afterEach(async () => { await act(async () => root.unmount()); host.remove(); document.body.innerHTML = ""; vi.clearAllMocks(); opened.mockReset(); vi.useRealTimers(); });
const analyse = () => host.querySelector<HTMLButtonElement>("[data-device-analyse]");
const report = () => host.querySelector<HTMLButtonElement>("[data-device-report]");
const diagnosticDialog = () => document.querySelector<HTMLElement>(".diagnostic-export");
const reportWizard = () => document.querySelector<HTMLElement>(".report-problem");
function noApiCalls() {
  for (const mock of [...Object.values(diagnostics), ...Object.values(bundle)]) expect(mock).not.toHaveBeenCalled();
}

it("the Diagnostics tab offers both entries as plain buttons, not menu items", async () => {
  await act(async () => root.render(<Workbench><DeviceWorkspace detail={detail} tree={project} onApplied={() => {}} /></Workbench>));
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
  expect(panel.querySelector("[data-diagnostics-open],[data-report-problem-open],[role=menuitem]")).toBeNull();
  expect(panel.textContent).toContain(en["deviceDiagnostics.hint"]
    .replace("{analysis}", en["diagnostics.title"]).replace("{report}", en["reportProblem.title"]));
  noApiCalls();
});

it("Analyse device opens the existing diagnostic export dialog with this device preselected", async () => {
  await act(async () => root.render(<Entry project={project} deviceId={9} />));
  await act(async () => analyse()!.click());
  const dialog = diagnosticDialog()!;
  expect(dialog.querySelector("h2")!.textContent).toBe(en["diagnostics.title"]);
  expect(dialog.querySelector<HTMLSelectElement>("[data-diagnostics-device]")!.value).toBe("9");
  expect(opened).not.toHaveBeenCalled();
  noApiCalls();
});

it("Send report opens the report wizard for reading this device", async () => {
  await act(async () => root.render(<Entry project={project} deviceId={10} />));
  await act(async () => report()!.click());
  expect(opened).toHaveBeenCalledExactlyOnceWith({ problemType: "deviceReading", deviceId: 10 });
  expect(diagnosticDialog()).toBeNull();
  noApiCalls();
});

it("outside the workbench there is no report host, so no report button", async () => {
  await act(async () => root.render(<DeviceDiagnosticsEntry project={project} deviceId={10} />));
  expect(analyse()).not.toBeNull();
  expect(report()).toBeNull();
});

it("a device change or another project closes the analysis; a revision keeps its stale review", async () => {
  await act(async () => root.render(<Entry project={project} deviceId={9} />));
  await act(async () => analyse()!.click());
  await act(async () => root.render(<Entry project={{ ...project, snapshot_revision: 2 }} deviceId={9} />));
  expect(diagnosticDialog()).not.toBeNull();
  await act(async () => root.render(<Entry project={{ ...project, snapshot_revision: 2 }} deviceId={10} />));
  expect(diagnosticDialog()).toBeNull();
  await act(async () => analyse()!.click());
  await act(async () => root.render(<Entry project={{ ...project, project_incarnation: 2 }} deviceId={10} />));
  expect(diagnosticDialog()).toBeNull();
});

const preview: DiagnosticView = {
  token: "a".repeat(64), target: "1.1.9", gateway: "192.0.2.10:3671", phase: "preview", cancelRequested: false,
  report: {
    kind: "knxbench-device-diagnostics", format: 1, build: "synthetic", collectedAt: "2026-10-24T00:00:00Z",
    device: "device-1", projectReference: "a".repeat(64), profile: "generic-identity-v1", assertedMask: null,
    assertedManufacturer: null, simulated: true, written: false, complete: false, stopReason: null,
    memoryStatus: "not-requested", memoryReason: "No safe memory profile", disconnectAttemptCompleted: false,
    items: ["descriptor-type-0", "manufacturer-id", "hardware-type"].map((query, index) => ({
      order: index + 1, query, objectIndex: index === 0 ? null : 0, propertyId: index === 0 ? null : index === 1 ? 12 : 78,
      startIndex: index === 0 ? null : 1, requestedElements: index === 0 ? null : 1, expectedBytes: index === 2 ? 6 : 2,
      status: "not-read", receivedBytes: 0, returnedElements: null, bytes: [], attemptedAt: null, completedAt: null, error: null,
    })),
  },
  memory: null, exportable: false,
};
const running: DiagnosticView = { ...preview, phase: "running" };
function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (reason: Error) => void;
  const promise = new Promise<T>((ok, fail) => { resolve = ok; reject = fail; });
  return { promise, resolve, reject };
}
async function click(selector: string) {
  await act(async () => document.querySelector<HTMLButtonElement>(selector)!.click());
}
async function openPreview(value: DiagnosticView = preview) {
  diagnostics.previewDiagnostics.mockResolvedValue(value);
  await click("[data-device-analyse]");
  const input = document.querySelector<HTMLInputElement>("[data-diagnostics-gateway]")!;
  await act(async () => {
    Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")!.set!.call(input, value.gateway);
    input.dispatchEvent(new Event("input", { bubbles: true }));
  });
  await click("[data-diagnostics-preview]");
}
async function start() {
  await click("[data-diagnostics-read-consent]");
  await click("[data-diagnostics-start]");
}

const departures = ["device change", "project change", "project close", "unmount"] as const;
function renderDeparture(reason: typeof departures[number]) {
  if (reason === "device change") root.render(<Entry project={project} deviceId={10} />);
  else if (reason === "project change") root.render(<Entry project={{ ...project, project_incarnation: 2 }} deviceId={9} />);
  else if (reason === "project close") root.render(<Entry project={null} deviceId={9} />);
  else root.render(null);
}
async function leaveAnalysis(reason: typeof departures[number]) {
  await act(async () => renderDeparture(reason));
  expect(diagnosticDialog()).toBeNull();
}

it.each(departures)("cancels a late running start after %s closes the analysis", async (reason) => {
  vi.useFakeTimers();
  const pending = deferred<DiagnosticView>();
  diagnostics.startDiagnostics.mockReturnValueOnce(pending.promise);
  diagnostics.cancelDiagnostics.mockResolvedValue({ ...running, cancelRequested: true });
  await act(async () => root.render(<Entry project={project} deviceId={9} />));
  await openPreview();
  await start();
  expect(diagnostics.startDiagnostics).toHaveBeenCalledExactlyOnceWith(preview.token);
  await leaveAnalysis(reason);
  expect(diagnostics.cancelDiagnostics).not.toHaveBeenCalled();
  await act(async () => pending.resolve(running));
  expect(diagnostics.cancelDiagnostics).toHaveBeenCalledExactlyOnceWith(preview.token);
  await act(async () => vi.advanceTimersByTimeAsync(1000));
  expect(diagnostics.pollDiagnostics).not.toHaveBeenCalled();
  expect(diagnosticDialog()).toBeNull();
});

it.each(departures.flatMap((reason) => ["before commit", "after commit"].map((timing) => ({ reason, timing }))))(
  "cancels the successful running start exactly once on $reason $timing", async ({ reason, timing }) => {
  vi.useFakeTimers();
  const pending = deferred<DiagnosticView>();
  const ownedRunning = { ...running, token: "c".repeat(64) };
  diagnostics.startDiagnostics.mockReturnValueOnce(pending.promise);
  diagnostics.cancelDiagnostics.mockResolvedValue({ ...ownedRunning, cancelRequested: true });
  await act(async () => root.render(<Entry project={project} deviceId={9} />));
  await openPreview();
  await start();
  if (timing === "before commit") {
    await act(async () => {
      pending.resolve(ownedRunning);
      // Run Start's continuation without letting React commit its queued running view.
      await Promise.resolve();
      expect(document.querySelector("[data-diagnostics-start]")).not.toBeNull();
      expect(document.querySelector("[data-diagnostics-cancel]")).toBeNull();
      renderDeparture(reason);
    });
  } else {
    await act(async () => pending.resolve(ownedRunning));
    expect(document.querySelector("[data-diagnostics-cancel]")).not.toBeNull();
    await leaveAnalysis(reason);
  }
  expect(diagnosticDialog()).toBeNull();
  await act(async () => vi.advanceTimersByTimeAsync(1000));
  expect(diagnostics.cancelDiagnostics).toHaveBeenCalledExactlyOnceWith(ownedRunning.token);
  expect(diagnostics.pollDiagnostics).not.toHaveBeenCalled();
});

it.each(["project close", "unmount"] as const)(
  "a rejected commit-boundary cleanup after %s cannot affect the newer job", async (reason) => {
  vi.useFakeTimers();
  const pending = deferred<DiagnosticView>();
  const cleanup = deferred<DiagnosticView>();
  const ownedRunning: DiagnosticView = { ...running, token: "c".repeat(64) };
  const newerPreview: DiagnosticView = { ...preview, token: "b".repeat(64), target: "1.1.10" };
  const newerRunning: DiagnosticView = { ...newerPreview, phase: "running" };
  diagnostics.startDiagnostics.mockReturnValueOnce(pending.promise).mockResolvedValueOnce(newerRunning);
  diagnostics.pollDiagnostics.mockResolvedValue(newerRunning);
  diagnostics.cancelDiagnostics.mockReturnValueOnce(cleanup.promise).mockResolvedValueOnce({ ...newerRunning, cancelRequested: true });
  await act(async () => root.render(<Entry project={project} deviceId={9} />));
  await openPreview();
  await start();
  await act(async () => {
    pending.resolve(ownedRunning);
    await Promise.resolve();
    expect(document.querySelector("[data-diagnostics-cancel]")).toBeNull();
    renderDeparture(reason);
  });
  expect(diagnosticDialog()).toBeNull();
  expect(diagnostics.cancelDiagnostics).toHaveBeenCalledExactlyOnceWith(ownedRunning.token);
  await act(async () => root.render(<Entry project={project} deviceId={10} />));
  await openPreview(newerPreview);
  await start();
  const newerDialog = diagnosticDialog();
  expect(newerDialog).not.toBeNull();
  await act(async () => cleanup.reject(new Error("cancel refused after replacement")));
  await act(async () => vi.advanceTimersByTimeAsync(250));
  expect(diagnostics.cancelDiagnostics).toHaveBeenCalledExactlyOnceWith(ownedRunning.token);
  expect(diagnostics.pollDiagnostics).toHaveBeenCalledExactlyOnceWith(newerPreview.token);
  expect(diagnosticDialog()).toBe(newerDialog);
  expect(newerDialog!.querySelector("[role=alert]")).toBeNull();
  expect(newerDialog!.querySelector<HTMLButtonElement>("[data-diagnostics-cancel]")!.disabled).toBe(false);
  await act(async () => root.render(null));
  expect(diagnostics.cancelDiagnostics.mock.calls).toEqual([[ownedRunning.token], [newerPreview.token]]);
});

it.each(["start", "poll", "stop"] as const)(
  "a completed %s response releases ownership before the terminal view commits", async (source) => {
  vi.useFakeTimers();
  const pending = deferred<DiagnosticView>();
  diagnostics.startDiagnostics.mockResolvedValue(running);
  diagnostics.cancelDiagnostics.mockResolvedValue({ ...running, cancelRequested: true });
  if (source === "start") diagnostics.startDiagnostics.mockReturnValueOnce(pending.promise);
  else if (source === "poll") diagnostics.pollDiagnostics.mockReturnValueOnce(pending.promise);
  else diagnostics.cancelDiagnostics.mockReturnValueOnce(pending.promise);
  await act(async () => root.render(<Entry project={project} deviceId={9} />));
  await openPreview();
  await start();
  if (source === "poll") {
    await act(async () => vi.advanceTimersByTimeAsync(250));
    expect(diagnostics.pollDiagnostics).toHaveBeenCalledExactlyOnceWith(running.token);
  } else if (source === "stop") await click("[data-diagnostics-cancel]");
  await act(async () => {
    pending.resolve({ ...running, phase: "complete", report: { ...running.report, complete: true } });
    await Promise.resolve();
    expect(document.querySelector("[data-diagnostics-result]")).toBeNull();
    expect(document.querySelector(source === "start" ? "[data-diagnostics-start]" : "[data-diagnostics-cancel]")).not.toBeNull();
    renderDeparture("unmount");
  });
  expect(diagnosticDialog()).toBeNull();
  expect(diagnostics.cancelDiagnostics.mock.calls).toEqual(source === "stop" ? [[running.token]] : []);
  await act(async () => vi.advanceTimersByTimeAsync(1000));
  expect(diagnostics.pollDiagnostics).toHaveBeenCalledTimes(source === "poll" ? 1 : 0);
});

it.each(["running", "complete", "rejected"] as const)("a late %s start never changes or cancels the newer job", async (outcome) => {
  vi.useFakeTimers();
  const pending = deferred<DiagnosticView>();
  const newerPreview: DiagnosticView = { ...preview, token: "b".repeat(64), target: "1.1.10" };
  const newerRunning: DiagnosticView = { ...newerPreview, phase: "running" };
  diagnostics.startDiagnostics.mockReturnValueOnce(pending.promise).mockResolvedValueOnce(newerRunning);
  diagnostics.pollDiagnostics.mockResolvedValue(newerRunning);
  // Cleanup failure is best effort; it must not leak into the newer dialog or reject unhandled.
  diagnostics.cancelDiagnostics.mockRejectedValue(new Error("cancel refused"));
  await act(async () => root.render(<Entry project={project} deviceId={9} />));
  await openPreview();
  await start();
  await leaveAnalysis("device change");
  await openPreview(newerPreview);
  await start();
  const newerDialog = diagnosticDialog();
  expect(newerDialog).not.toBeNull();
  expect(diagnostics.startDiagnostics.mock.calls).toEqual([[preview.token], [newerPreview.token]]);
  await act(async () => {
    if (outcome === "rejected") pending.reject(new Error("start refused"));
    else pending.resolve({ ...running, phase: outcome });
  });
  expect(diagnostics.cancelDiagnostics.mock.calls).toEqual(outcome === "running" ? [[preview.token]] : []);
  await act(async () => vi.advanceTimersByTimeAsync(250));
  expect(diagnostics.pollDiagnostics).toHaveBeenCalledExactlyOnceWith(newerPreview.token);
  expect(diagnosticDialog()).toBe(newerDialog);
  expect(newerDialog!.querySelector("[role=alert]")).toBeNull();
  expect(newerDialog!.querySelector<HTMLButtonElement>("[data-diagnostics-cancel]")!.disabled).toBe(false);
  await act(async () => root.render(null));
  expect(diagnostics.cancelDiagnostics.mock.calls).toEqual(outcome === "running" ? [[preview.token], [newerPreview.token]] : [[newerPreview.token]]);
});

it.each(["pending", "running"] as const)("a revision keeps the %s start and hands the stopped partial result to the report wizard", async (phase) => {
  vi.useFakeTimers();
  const pending = deferred<DiagnosticView>();
  const partial: DiagnosticView = {
    ...running, phase: "complete", cancelRequested: true, exportable: true,
    report: { ...running.report, complete: true, stopReason: "cancelled", disconnectAttemptCompleted: true,
      items: running.report.items.map((item, index) => index === 0 ? {
        ...item, status: "read", bytes: [7, 1], receivedBytes: 2,
        attemptedAt: "2026-10-24T00:00:00Z", completedAt: "2026-10-24T00:00:01Z",
      } : item),
    },
  };
  diagnostics.startDiagnostics.mockReturnValueOnce(pending.promise);
  diagnostics.cancelDiagnostics.mockResolvedValue(partial);
  await act(async () => root.render(<Entry project={project} deviceId={9} />));
  await openPreview();
  await start();
  if (phase === "running") await act(async () => pending.resolve(running));
  const dialog = diagnosticDialog();
  expect(dialog).not.toBeNull();
  await act(async () => root.render(<Entry project={{ ...project, snapshot_revision: 2 }} deviceId={9} />));
  if (phase === "pending") await act(async () => pending.resolve(running));
  expect(diagnosticDialog()).toBe(dialog);
  expect(diagnostics.cancelDiagnostics).not.toHaveBeenCalled();
  expect(dialog!.textContent).toContain(en["diagnostics.stale"]);
  await click("[data-diagnostics-cancel]");
  expect(diagnostics.cancelDiagnostics).toHaveBeenCalledExactlyOnceWith(preview.token);
  expect(document.querySelector("[data-diagnostics-result]")!.textContent).toContain("cancelled");
  expect(JSON.parse(document.querySelector('[aria-label="report.json"]')!.textContent!).items[0].bytes).toEqual([7, 1]);
  // DIAG-03: the ZIP moved into the report wizard, which finds this read by its token.
  expect(document.querySelector("[data-diagnostics-save],[data-diagnostics-share-consent]")).toBeNull();
  expect(opened).not.toHaveBeenCalled();
  await click("[data-report-problem-open=diagnostics]");
  expect(opened).toHaveBeenCalledExactlyOnceWith({ problemType: "deviceReading", deviceId: 9,
    operation: { relatedKey: "diagnosticsToken", relatedValue: preview.token } });
  expect(diagnosticDialog()).toBeNull();
  expect(diagnostics.saveDiagnostics).not.toHaveBeenCalled();
  await act(async () => root.render(null));
  expect(diagnostics.cancelDiagnostics).toHaveBeenCalledTimes(1);
});

it("both entries are disabled without a project and open nothing", async () => {
  await act(async () => root.render(<Entry project={null} deviceId={9} />));
  expect(analyse()!.disabled).toBe(true);
  expect(report()!.disabled).toBe(true);
  await act(async () => { analyse()!.click(); report()!.click(); });
  expect(diagnosticDialog()).toBeNull();
  expect(opened).not.toHaveBeenCalled();
  expect(reportWizard()).toBeNull();
  noApiCalls();
});
