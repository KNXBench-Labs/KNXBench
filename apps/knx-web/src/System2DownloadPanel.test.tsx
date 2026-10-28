/** The actual Download entry and experimental controller, with intercepted API only. */
// @vitest-environment happy-dom
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
const mock = vi.hoisted(() => ({ prepareSystem2: vi.fn(), getSystem2Status: vi.fn(), startSystem2: vi.fn(), cancelSystem2: vi.fn(), recoverSystem2: vi.fn(), getSystem2Report: vi.fn() }));
const saved = vi.hoisted(() => vi.fn());
vi.mock("./localJsonDownload", () => ({ downloadLocalJson: saved }));
const consent = vi.hoisted(() => ({ request: vi.fn() }));
const ordinary = vi.hoisted(() => ({ discoverBusInterfaces: vi.fn(), planDeviceDownload: vi.fn(), startDeviceDownload: vi.fn() }));
vi.mock("./system2DownloadApi", async (importOriginal) => ({
  ...await importOriginal<typeof import("./system2DownloadApi")>(), ...mock,
}));
vi.mock("./useProgrammingConsent", () => ({ useProgrammingConsent: () => ({ request: consent.request, dialog: null }) }));
vi.mock("./api", () => ({
  ...ordinary,
  pollDeviceDownload: vi.fn().mockRejectedValue({ status: 404 }), getDeviceReadiness: vi.fn().mockResolvedValue({ devices: [] }),
  busActivity: vi.fn().mockResolvedValue({ sessions: [], busyLocks: [], untracked: [], oneShot: [] }),
  errorMessage: (e: unknown) => e instanceof Error ? e.message : String(e), errorStatus: (e: {status?: number}) => e.status,
}));
import DeviceDownloadPanel from "./DeviceDownloadPanel";
import System2DownloadPanel from "./System2DownloadPanel";
import { preview, status, recoveryStatus } from "./system2DownloadTestFixtures";
import type { ProjectTree } from "./bindings/ProjectTree";
import type { BusDiscoveredInterface } from "./api";
import { resetBusDiscoveryForTests } from "./busDiscovery";
import { resetSettingsForTests } from "./settingsStore";
import { resetSystem2StatusForTests } from "./system2Status";
(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;
const project: ProjectTree = { schema_version: 1, errors: 0, warnings: 0, can_undo: false, can_redo: false, is_modified: false, group_address_style: "ThreeLevel", server_incarnation: "server", project_incarnation: 1,
  installations: [{ id: 0, name: "Test", buildings: [], group_addresses: [], group_ranges: [], unassigned: [], topology: [{ id: 1, name: "Area", address: 1, lines: [{ id: 2, name: "Line", address: 1, devices: [{id: 10, name: "Configured button", address: "1.1.67", description: null, com_object_count: 0 }] }] }] }] };
let host: HTMLDivElement; let root: Root;
const flush = async () => { await act(async () => { for (let i=0;i<10;i++) await Promise.resolve(); }); };
const click = async (selector: string) => { const node = host.querySelector<HTMLButtonElement>(selector); expect(node).not.toBeNull(); await act(async () => node!.click()); await flush(); };
async function input(selector: string, value: string) {
  const node = host.querySelector<HTMLInputElement>(selector)!; expect(node).not.toBeNull();
  await act(async () => { Object.getOwnPropertyDescriptor(window.HTMLInputElement.prototype, "value")!.set!.call(node, value); node.dispatchEvent(new Event("input", {bubbles:true})); });
}
async function render(deviceId = 10, tree = project) {
  await act(async () => root.render(<System2DownloadPanel project={tree} deviceId={deviceId} gateway="192.0.2.1:3671" enabled ordinaryBusy={false} />)); await flush();
}
async function prepare() { await click("[data-system2-prepare]"); }
async function confirmations() {
  for (const [key, value] of Object.entries({ confirmation: preview.confirmationPhrase, hypothesisConfirmation: preview.hypothesisPhrase, downloadConfirmation: preview.downloadPhrase, releaseConfirmation: preview.releasePhrase })) await input(`[data-system2-phrase='${key}']`, value);
  const select = host.querySelector<HTMLSelectElement>("[data-system2-track]")!;
  await act(async () => { select.value = "knxbenchBestEffort"; select.dispatchEvent(new Event("change", {bubbles:true})); });
}
beforeEach(() => {
  vi.clearAllMocks(); window.localStorage.clear(); resetSettingsForTests(); resetBusDiscoveryForTests(); resetSystem2StatusForTests();
  mock.getSystem2Status.mockResolvedValue(null); mock.prepareSystem2.mockResolvedValue(preview); mock.startSystem2.mockResolvedValue(status);
  mock.cancelSystem2.mockResolvedValue({ ...status, state: "cancelled" }); consent.request.mockResolvedValue(true);
  ordinary.discoverBusInterfaces.mockResolvedValue({ interfaces: [] });
  host = document.createElement("div"); document.body.append(host); root = createRoot(host);
});
afterEach(async () => { await act(async () => root.unmount()); host.remove(); vi.useRealTimers(); });
it("reattaches a server-owned System-2 operation when returning to ordinary Download without reselecting the mode", async () => {
  mock.getSystem2Status.mockResolvedValue(status);
  await act(async () => root.render(<DeviceDownloadPanel project={project} />)); await flush();
  expect(mock.getSystem2Status).toHaveBeenCalledOnce();
  expect(host.querySelector("[data-system2-status]")!.textContent).toContain(status.operationId);
  expect(mock.startSystem2).not.toHaveBeenCalled(); expect(mock.recoverSystem2).not.toHaveBeenCalled();
});
it("offers System-2 in the existing device download while preserving the ordinary scope", async () => {
  await act(async () => root.render(<DeviceDownloadPanel project={project} />)); await flush();
  expect(host.querySelector(".device-download-scope")).not.toBeNull();
  const mode = host.querySelector<HTMLSelectElement>("[data-download-mode]"); expect(mode).not.toBeNull();
  await act(async () => { mode!.value = "system2"; mode!.dispatchEvent(new Event("change", {bubbles:true})); }); await flush();
  expect(host.querySelector("[data-system2-prepare]")).not.toBeNull(); expect(mock.prepareSystem2).not.toHaveBeenCalled();
  expect(host.querySelector(".device-download-scope")).toBeNull();
});
it.each([0, 2])("recovers the original without a project or saved gateway after %i discovery results, only with explicit gateway and fresh consent", async (count: number) => {
  const interfaces: BusDiscoveredInterface[] = Array.from({ length: count }, (_, index) => ({
    controlEndpoint: `192.0.2.${index + 1}:3671`, individualAddress: `1.1.${index + 1}`,
    friendlyName: `Fixture gateway ${index + 1}`, supportsTunnelling: true,
  }));
  ordinary.discoverBusInterfaces.mockResolvedValue({ interfaces });
  mock.getSystem2Status.mockResolvedValue(recoveryStatus);
  await act(async () => root.render(<DeviceDownloadPanel project={null} />)); await flush();
  const gateway = host.querySelector<HTMLInputElement>("[data-download-gateway]");
  expect(gateway).not.toBeNull(); expect(gateway!.disabled).toBe(false); expect(gateway!.value).toBe("");
  expect(host.querySelector("[data-download-device]")).toBeNull();
  expect(host.querySelector("[data-download-mode]")).toBeNull();
  expect(host.querySelector(".device-download-scope")).toBeNull();
  expect(host.querySelector(".device-download-actions")).toBeNull();
  expect(host.querySelector("[data-system2-prepare]")).toBeNull();
  expect(host.querySelector("[data-system2-status]")!.textContent).toContain(recoveryStatus.operationId);
  expect(host.querySelector("[data-system2-status]")!.textContent).toContain(`${recoveryStatus.address} — ${recoveryStatus.deviceName}`);
  expect(ordinary.discoverBusInterfaces).not.toHaveBeenCalled();
  expect(consent.request).not.toHaveBeenCalled(); expect(mock.recoverSystem2).not.toHaveBeenCalled();
  const search = host.querySelector<HTMLButtonElement>(".device-download-search"); expect(search).not.toBeNull();
  await act(async () => search!.click()); await flush();
  expect(ordinary.discoverBusInterfaces).toHaveBeenCalledOnce(); expect(gateway!.value).toBe("");
  expect(host.querySelectorAll("[data-system2-recovery]")).toHaveLength(3);
  for (const node of host.querySelectorAll<HTMLInputElement>("[data-system2-recovery]")) expect(node.value).toBe("");
  for (const [key, value] of Object.entries({ recoveryConfirmation: recoveryStatus.recoveryPhrase!, downloadConfirmation: recoveryStatus.downloadPhrase!, releaseConfirmation: recoveryStatus.releasePhrase! })) await input(`[data-system2-recovery='${key}']`, value);
  expect(host.querySelector<HTMLButtonElement>("[data-system2-recover]")!.disabled).toBe(true);
  if (count > 0) {
    const found = host.querySelector<HTMLSelectElement>("[data-download-gateway-found]"); expect(found).not.toBeNull();
    await act(async () => { found!.value = interfaces[1].controlEndpoint; found!.dispatchEvent(new Event("change", { bubbles: true })); });
    expect(gateway!.value).toBe(interfaces[1].controlEndpoint);
  }
  await input("[data-download-gateway]", "203.0.113.250:3671");
  expect(host.querySelector<HTMLButtonElement>("[data-system2-recover]")!.disabled).toBe(false);
  expect(mock.recoverSystem2).not.toHaveBeenCalled(); expect(consent.request).not.toHaveBeenCalled();
  consent.request.mockResolvedValueOnce(false); await click("[data-system2-recover]");
  expect(mock.recoverSystem2).not.toHaveBeenCalled();
  let allow!: (value: boolean) => void;
  consent.request.mockReturnValueOnce(new Promise<boolean>((resolve) => { allow = resolve; }));
  mock.recoverSystem2.mockResolvedValue({ ...status, state: "restoring" });
  await click("[data-system2-recover]");
  expect(consent.request).toHaveBeenCalledTimes(2);
  expect(consent.request).toHaveBeenLastCalledWith(`${recoveryStatus.address} — ${recoveryStatus.deviceName}`);
  expect(mock.recoverSystem2).not.toHaveBeenCalled();
  await act(async () => allow(true)); await flush();
  expect(mock.recoverSystem2).toHaveBeenCalledExactlyOnceWith({ operationId: recoveryStatus.operationId, gateway: "203.0.113.250:3671", recoveryConfirmation: recoveryStatus.recoveryPhrase, downloadConfirmation: recoveryStatus.downloadPhrase, releaseConfirmation: recoveryStatus.releasePhrase });
  expect(mock.prepareSystem2).not.toHaveBeenCalled(); expect(mock.startSystem2).not.toHaveBeenCalled(); expect(mock.cancelSystem2).not.toHaveBeenCalled();
  expect(ordinary.planDeviceDownload).not.toHaveBeenCalled(); expect(ordinary.startDeviceDownload).not.toHaveBeenCalled();
});
it("mounts read-only, prepares actual retained sources and requires every exact phrase and track before ordinary consent", async () => {
  await render(); expect(mock.getSystem2Status).toHaveBeenCalledOnce(); expect(mock.prepareSystem2).not.toHaveBeenCalled(); expect(mock.startSystem2).not.toHaveBeenCalled();
  await prepare(); expect(mock.prepareSystem2).toHaveBeenCalledWith(10); expect(host.textContent).toContain(preview.configurationSha256);
  expect(host.textContent).toContain(preview.warnings[0]); expect(host.querySelector<HTMLButtonElement>("[data-system2-start]")!.disabled).toBe(true);
  await confirmations(); await input("[data-system2-phrase='confirmation']", preview.confirmationPhrase + " ");
  expect(host.querySelector<HTMLButtonElement>("[data-system2-start]")!.disabled).toBe(true); expect(consent.request).not.toHaveBeenCalled();
  await input("[data-system2-phrase='confirmation']", preview.confirmationPhrase); await click("[data-system2-start]");
  expect(consent.request).toHaveBeenCalledWith("1.1.67 — Configured button");
  expect(mock.startSystem2).toHaveBeenCalledWith({ preparationId: preview.preparationId, gateway: "192.0.2.1:3671", confirmation: preview.confirmationPhrase, hypothesisConfirmation: preview.hypothesisPhrase, downloadConfirmation: preview.downloadPhrase, releaseConfirmation: preview.releasePhrase, recoveryTrack: "knxbenchBestEffort" });
});
it("drops a late preparation after target or project change", async () => {
  let resolve!: (value: typeof preview) => void; mock.prepareSystem2.mockReturnValueOnce(new Promise((r) => {resolve=r;}));
  await render(); await prepare(); await render(11); await act(async () => resolve(preview)); await flush();
  expect(host.querySelector("[data-system2-preview]")).toBeNull();
});
it("retains original operation identity across selection and never cancels on unmount", async () => {
  mock.getSystem2Status.mockResolvedValue(status); await render(); await render(11);
  expect(host.querySelector("[data-system2-status]")!.textContent).toContain("1.1.67");
  expect(host.querySelector("[data-system2-status]")!.textContent).toContain(status.operationId);
  expect(host.querySelector("progress")).toBeNull(); await click("[data-system2-cancel]"); expect(mock.cancelSystem2).toHaveBeenCalledWith(status.operationId);
  await act(async () => root.unmount()); root = createRoot(host); expect(mock.cancelSystem2).toHaveBeenCalledTimes(1);
});
it("a lost start response polls only, consumes the local preview and never resends", async () => {
  await render(); await prepare(); await confirmations(); mock.startSystem2.mockRejectedValueOnce(new Error("response lost"));
  mock.getSystem2Status.mockResolvedValue(status); await click("[data-system2-start]");
  expect(mock.startSystem2).toHaveBeenCalledOnce(); expect(mock.getSystem2Status.mock.calls.length).toBeGreaterThan(1);
  expect(host.querySelector("[data-system2-preview]")).toBeNull(); expect(host.querySelector("[data-system2-unknown]")).not.toBeNull();
  mock.getSystem2Status.mockResolvedValue(recoveryStatus); await click("[data-system2-refresh]");
  expect(mock.recoverSystem2).not.toHaveBeenCalled();
  expect(host.querySelector<HTMLButtonElement>("[data-system2-recover]")!.disabled).toBe(true);
  for (const [key, value] of Object.entries({ recoveryConfirmation: recoveryStatus.recoveryPhrase!, downloadConfirmation: recoveryStatus.downloadPhrase!, releaseConfirmation: recoveryStatus.releasePhrase! })) await input(`[data-system2-recovery='${key}']`, value);
  mock.recoverSystem2.mockResolvedValue({ ...status, state: "restoring" }); await click("[data-system2-recover]");
  expect(consent.request).toHaveBeenCalledTimes(2); expect(mock.startSystem2).toHaveBeenCalledOnce();
  expect(mock.recoverSystem2).toHaveBeenCalledWith({ operationId: status.operationId, gateway: "192.0.2.1:3671", recoveryConfirmation: recoveryStatus.recoveryPhrase, downloadConfirmation: recoveryStatus.downloadPhrase, releaseConfirmation: recoveryStatus.releasePhrase });
});
it("shows a refused start's reason while keeping the outcome unknown and never resending (issue #30)", async () => {
  await render(); await prepare(); await confirmations();
  mock.startSystem2.mockRejectedValueOnce(Object.assign(new Error("system2Busy"), { status: 409, body: { error: "system2Busy", kind: "system2Busy" } }));
  await click("[data-system2-start]");
  expect(host.querySelector("[data-system2-unknown]")).not.toBeNull();
  expect(host.querySelector("[data-system2-start-refusal]")!.textContent).toBe("The server refused this start request: system2Busy.");
  expect(mock.startSystem2).toHaveBeenCalledOnce();
});
it("shows no reason for a lost start response", async () => {
  await render(); await prepare(); await confirmations();
  mock.startSystem2.mockRejectedValueOnce(new Error("response lost")); await click("[data-system2-start]");
  expect(host.querySelector("[data-system2-unknown]")).not.toBeNull();
  expect(host.querySelector("[data-system2-start-refusal]")).toBeNull();
});
it("shows the server's reason for a run that ended before writing (issue #30)", async () => {
  mock.getSystem2Status.mockResolvedValue({ ...status, state: "failed", refusal: "gatewayBusy" }); await render();
  expect(host.querySelector("[data-system2-refusal]")!.textContent).toBe("Reason reported by the server: gatewayBusy.");
  expect(host.querySelector("[data-system2-nothing-written]")).not.toBeNull();
  mock.getSystem2Status.mockResolvedValue({ ...status, state: "failed" }); await click("[data-system2-refresh]");
  expect(host.querySelector("[data-system2-refusal]")).toBeNull();
});
it("never binds recovery phrases to a start response for another target", async () => {
  await render(); await prepare(); await confirmations();
  mock.startSystem2.mockResolvedValueOnce({ ...status, binding: { ...status.binding, deviceId: 11 }, address: "1.1.68" });
  await click("[data-system2-start]");
  expect(host.querySelector("[data-system2-unknown]")).not.toBeNull();
  expect(host.querySelector("[data-system2-status]")).toBeNull();
});
it("blocks stale project consent before sending any start", async () => {
  let allow!: (v: boolean) => void; consent.request.mockReturnValueOnce(new Promise((r) => { allow = r; }));
  await render(); await prepare(); await confirmations(); await click("[data-system2-start]");
  await render(10, { ...project, snapshot_revision: 3 }); await act(async () => allow(true)); await flush();
  expect(mock.startSystem2).not.toHaveBeenCalled();
});
it("attaches a retained operation read-only and refuses to guess missing recovery acknowledgements", async () => {
  mock.getSystem2Status.mockResolvedValue({ ...status, state: "interrupted", backupAvailable: true });
  await render(); expect(host.textContent).toContain("unavailable in the current transport status");
  expect(host.querySelector("[data-system2-recover]")).toBeNull(); expect(mock.recoverSystem2).not.toHaveBeenCalled();
});
it("reattaches original recovery acknowledgements after navigation, never substituting the latest preview", async () => {
  mock.getSystem2Status.mockResolvedValue(recoveryStatus); await render();
  await act(async () => root.unmount()); root = createRoot(host); await render(11);
  expect(mock.prepareSystem2).not.toHaveBeenCalled(); expect(mock.startSystem2).not.toHaveBeenCalled(); expect(mock.recoverSystem2).not.toHaveBeenCalled();
  expect(host.querySelectorAll("[data-system2-recovery]")).toHaveLength(3);
  for (const node of host.querySelectorAll<HTMLInputElement>("[data-system2-recovery]")) expect(node.value).toBe("");
  const latest = { ...preview, binding: { ...preview.binding, deviceId: 11 }, address: "1.1.68", preparationId: "e".repeat(32), downloadPhrase: "Download a different plan", releasePhrase: "Release a different plan" };
  mock.prepareSystem2.mockResolvedValue(latest); await prepare();
  await input("[data-system2-recovery='recoveryConfirmation']", recoveryStatus.recoveryPhrase!);
  await input("[data-system2-recovery='downloadConfirmation']", latest.downloadPhrase);
  await input("[data-system2-recovery='releaseConfirmation']", latest.releasePhrase);
  expect(host.querySelector<HTMLButtonElement>("[data-system2-recover]")!.disabled).toBe(true);
  await input("[data-system2-recovery='downloadConfirmation']", recoveryStatus.downloadPhrase!);
  await input("[data-system2-recovery='releaseConfirmation']", recoveryStatus.releasePhrase!);
  consent.request.mockResolvedValueOnce(false); await click("[data-system2-recover]"); expect(mock.recoverSystem2).not.toHaveBeenCalled();
  mock.recoverSystem2.mockResolvedValue({ ...status, state: "restoring" }); await click("[data-system2-recover]");
  expect(consent.request).toHaveBeenLastCalledWith("1.1.67 — Configured button");
  expect(mock.recoverSystem2).toHaveBeenCalledWith({ operationId: status.operationId, gateway: "192.0.2.1:3671", recoveryConfirmation: recoveryStatus.recoveryPhrase, downloadConfirmation: recoveryStatus.downloadPhrase, releaseConfirmation: recoveryStatus.releasePhrase });
  expect(mock.startSystem2).not.toHaveBeenCalled();
});
it("a refused preparation leaves no start capability or defaults fallback", async () => {
  mock.prepareSystem2.mockRejectedValueOnce(new Error("private debug output must not be displayed"));
  await render(); await prepare(); expect(host.textContent).toContain("Configured preparation was refused");
  expect(host.textContent).not.toContain("private debug"); expect(host.querySelector("[data-system2-start]")).toBeNull(); expect(mock.startSystem2).not.toHaveBeenCalled();
});
it("a lost recovery response is never automatically resent and retains the original operation", async () => {
  await render(); await prepare(); await confirmations(); await click("[data-system2-start]");
  mock.getSystem2Status.mockResolvedValue({ ...recoveryStatus, state: "failed" });
  await click("[data-system2-refresh]");
  for (const [key, value] of Object.entries({ recoveryConfirmation: "Recover original actual state", downloadConfirmation: preview.downloadPhrase, releaseConfirmation: preview.releasePhrase })) await input(`[data-system2-recovery='${key}']`, value);
  mock.recoverSystem2.mockRejectedValueOnce(new Error("lost response")); await click("[data-system2-recover]");
  await click("[data-system2-refresh]"); expect(mock.recoverSystem2).toHaveBeenCalledOnce();
  expect(host.querySelector("[data-system2-unknown]")).not.toBeNull(); expect(host.querySelector("[data-system2-status]")!.textContent).toContain(status.operationId);
  expect(host.querySelector<HTMLButtonElement>("[data-system2-recover]")!.disabled).toBe(true);
});
it("requires explicit recovery of the original backup with fresh ordinary consent", async () => {
  await render(); await prepare(); await confirmations(); await click("[data-system2-start]");
  mock.getSystem2Status.mockResolvedValue({ ...recoveryStatus, state: "failed" });
  await click("[data-system2-refresh]");
  expect(mock.recoverSystem2).not.toHaveBeenCalled();
  for (const [key, value] of Object.entries({ recoveryConfirmation: "Recover original actual state", downloadConfirmation: preview.downloadPhrase, releaseConfirmation: preview.releasePhrase })) await input(`[data-system2-recovery='${key}']`, value);
  mock.recoverSystem2.mockResolvedValue({ ...status, state: "restoring" }); await click("[data-system2-recover]");
  expect(consent.request).toHaveBeenCalledTimes(2); expect(mock.recoverSystem2).toHaveBeenCalledWith({ operationId: status.operationId, gateway: "192.0.2.1:3671", recoveryConfirmation: "Recover original actual state", downloadConfirmation: preview.downloadPhrase, releaseConfirmation: preview.releasePhrase });
});
it("says nothing was written only when the server reports no mutation could have started", async () => {
  mock.getSystem2Status.mockResolvedValue({ ...status, state: "failed", mutationMayHaveStarted: false });
  await render();
  expect(host.querySelector("[data-system2-nothing-written]")).not.toBeNull();
  expect(host.textContent).toContain("nothing was written");
  expect(host.textContent).not.toContain("device state may have changed");
  mock.getSystem2Status.mockResolvedValue({ ...status, state: "failed", mutationMayHaveStarted: true });
  await click("[data-system2-refresh]");
  expect(host.querySelector("[data-system2-nothing-written]")).toBeNull();
  expect(host.textContent).toContain("device state may have changed");
});
it("lists pointer observations and offers no original-state recovery after a differing prestate", async () => {
  const { prestateStatus } = await import("./system2DownloadTestFixtures");
  mock.getSystem2Status.mockResolvedValue(prestateStatus);
  await render();
  const observations = host.querySelector("[data-system2-observations]");
  expect(observations).not.toBeNull();
  expect(observations!.textContent).toContain("00003c00"); expect(observations!.textContent).toContain("00000000");
  expect(host.querySelector("[data-system2-recovery-unavailable]")).not.toBeNull();
  expect(host.querySelector("[data-system2-recovery]")).toBeNull(); expect(host.querySelector("[data-system2-recover]")).toBeNull();
});
it("labels in-procedure and refused pointer observations", async () => {
  const { inProcedureStatus } = await import("./system2DownloadTestFixtures");
  mock.getSystem2Status.mockResolvedValue(inProcedureStatus);
  await render();
  const text = host.querySelector("[data-system2-observations]")!.textContent!;
  expect(text).toContain("During the download"); expect(text).toContain("step 4"); expect(text).toContain("00003d00");
  expect(text).toContain("not reported (read refused)");
  expect(text).toContain("Recovery run:");
});
it("never calls zero placeholders a backup when prior memory was unreadable", async () => {
  const { unreadableStatus } = await import("./system2DownloadTestFixtures");
  mock.getSystem2Status.mockResolvedValue(unreadableStatus);
  await render();
  expect(host.textContent).not.toContain("Actual-prestate backup evidence is retained");
  const line = host.querySelector("[data-system2-prestate-unreadable]");
  expect(line).not.toBeNull();
  expect(line!.textContent).toContain("Prior memory could not be read — no backup");
  expect(line!.textContent).toContain("3c00");
  expect(host.querySelector("[data-system2-recovery-unavailable]")!.textContent).toContain("memory");
  expect(host.querySelector("[data-system2-recover]")).toBeNull();
});
const text = (selector: string) => host.querySelector(selector)?.textContent ?? "";
it("renders a typed download diagnosis as a translated observation, not a support claim", async () => {
  const { downloadFailedStatus } = await import("./system2DownloadTestFixtures");
  mock.getSystem2Status.mockResolvedValue(downloadFailedStatus); await render();
  const section = text("[data-system2-diagnostics]");
  expect(section).toContain("Recorded pointer-read observations"); expect(section).toContain("observations, not a statement about hardware support");
  expect(section).toContain("Download run"); expect(section).toContain("neither two nor four octets");
  expect(section).toContain("object 3"); expect(section).toContain("property 7"); expect(section).toContain("elements 1"); expect(section).toContain("octets 3");
  expect(host.querySelector("[data-system2-diagnostic='download']")).not.toBeNull(); expect(host.querySelector("[data-system2-diagnostic='restore']")).toBeNull();
  expect(host.querySelector("[data-system2-diagnostic-unavailable]")).toBeNull();
  expect(host.textContent).not.toContain("unsupportedWidth");
});
it("keeps the historical download diagnosis distinct from the current restore reason", async () => {
  const { restoreFailedStatus } = await import("./system2DownloadTestFixtures");
  mock.getSystem2Status.mockResolvedValue(restoreFailedStatus); await render();
  expect(text("[data-system2-diagnostic='download']")).toContain("neither two nor four octets");
  expect(text("[data-system2-diagnostic='restore']")).toContain("Restore run"); expect(text("[data-system2-diagnostic='restore']")).toContain("element count other than one");
  expect(text("[data-system2-diagnostic='restore']")).toContain("elements 2"); expect(text("[data-system2-diagnostic='restore']")).toContain("octets 8");
  expect(text("[data-system2-diagnostic='download']")).not.toContain("element count other than one");
});
it("shows 'not available' for a restore failure whose only diagnosis is the prior download", async () => {
  const { restoreFailedStatus, downloadDiagnostic } = await import("./system2DownloadTestFixtures");
  mock.getSystem2Status.mockResolvedValue({ ...restoreFailedStatus, failureDiagnostics: [downloadDiagnostic] }); await render();
  expect(text("[data-system2-diagnostic='download']")).toContain("neither two nor four octets");
  expect(host.querySelector("[data-system2-diagnostic='restore']")).toBeNull();
  expect(text("[data-system2-diagnostic-unavailable='restore']")).toContain("no typed pointer-read observation is available");
});
it.each([["failed", "download"], ["restoreFailed", "restore"]] as const)("shows 'not available' for %s without typed metadata and never infers it from message or refusal", async (state, run) => {
  mock.getSystem2Status.mockResolvedValue({ ...status, state, mutationMayHaveStarted: true, refusal: "recoveryRefused", message: "unsupported width: 3 octets, elements 1, object 3" });
  await render();
  expect(text(`[data-system2-diagnostic-unavailable='${run}']`)).toContain("no typed pointer-read observation is available");
  expect(host.querySelector("[data-system2-diagnostic]")).toBeNull();
});
it("shows no diagnostics block for an old-wire status or a healthy run", async () => {
  const { wireV1 } = await import("./system2DownloadTestFixtures");
  mock.getSystem2Status.mockResolvedValue({ ...wireV1(status), failureDiagnostics: [], state: "finished" }); await render();
  expect(host.querySelector("[data-system2-diagnostics]")).toBeNull();
});
it("renders the diagnosis fully in German", async () => {
  const { setSetting } = await import("./settingsStore"); const { restoreFailedStatus } = await import("./system2DownloadTestFixtures");
  await act(async () => setSetting("uiLanguage", "de"));
  mock.getSystem2Status.mockResolvedValue(restoreFailedStatus); await render();
  const section = text("[data-system2-diagnostics]");
  expect(section).toContain("Aufgezeichnete Zeiger-Lesebeobachtungen"); expect(section).toContain("Download-Lauf"); expect(section).toContain("Wiederherstellungslauf");
  expect(section).toContain("weder zwei noch vier Oktette"); expect(section).toContain("andere Elementanzahl als eins"); expect(section).toContain("Objekt 3"); expect(section).toContain("Oktette 8");
  expect(section).not.toContain("Download run"); expect(section).not.toContain("octets");
  await act(async () => setSetting("uiLanguage", "en"));
});
it("drops a prior diagnosis when a new operation starts", async () => {
  const { downloadFailedStatus } = await import("./system2DownloadTestFixtures");
  mock.getSystem2Status.mockResolvedValue(downloadFailedStatus); await render();
  expect(host.querySelector("[data-system2-diagnostics]")).not.toBeNull();
  const fresh = { ...status, operationId: "e".repeat(32) };
  mock.getSystem2Status.mockResolvedValue(fresh); mock.startSystem2.mockResolvedValue(fresh);
  await prepare(); await confirmations(); await click("[data-system2-start]");
  expect(text("[data-system2-status]")).toContain(fresh.operationId);
  expect(host.querySelector("[data-system2-diagnostics]")).toBeNull(); expect(host.querySelector("[data-system2-diagnostic-unavailable]")).toBeNull();
});
it("renders diagnostics without enabling retry or changing recovery", async () => {
  const { downloadFailedStatus, restoreFailedStatus } = await import("./system2DownloadTestFixtures");
  mock.getSystem2Status.mockResolvedValue(downloadFailedStatus); await render();
  expect(mock.startSystem2).not.toHaveBeenCalled(); expect(mock.recoverSystem2).not.toHaveBeenCalled(); expect(consent.request).not.toHaveBeenCalled();
  expect(host.querySelector("[data-system2-recover]")).toBeNull();
  mock.getSystem2Status.mockResolvedValue(restoreFailedStatus); await click("[data-system2-refresh]");
  expect(host.querySelectorAll("[data-system2-recovery]")).toHaveLength(3); expect(host.querySelector<HTMLButtonElement>("[data-system2-recover]")!.disabled).toBe(true);
});

// Issue #31: an ended run is explained from the journal facts and its report can be saved.
const unfinished = { ...recoveryStatus, state: "failed" as const, completedSteps: 26, totalSteps: 27, downloadResult: "transferCompleteVerificationUnfinished" as const };
const reportBody = { format: 1, kind: "knxbench-system2-download-summary", support: "EXPERIMENTAL BEST-EFFORT; HARDWARE COMPATIBILITY AND RECOVERY UNVERIFIED",
  evidence: "local-journal-only", outcome: "failed", downloadResult: "transferCompleteVerificationUnfinished", mutationPossible: true,
  completedSteps: 26, totalSteps: 27, prestateDiffers: false, failureDiagnostics: [] };
it("explains transfer-complete-but-unverified runs without claiming a loaded device", async () => {
  mock.getSystem2Status.mockResolvedValue(unfinished); await render();
  const text = host.querySelector("[data-system2-result-text]")!.textContent!;
  expect(text).toContain("final verification step did not finish"); expect(text).toContain("not confirmed loaded");
  expect(host.querySelector("[data-system2-status]")!.textContent).not.toMatch(/nothing was written/i);
});
it("shows no result and no report while the operation is running", async () => {
  mock.getSystem2Status.mockResolvedValue(status); await render();
  expect(host.querySelector("[data-system2-result-text]")).toBeNull(); expect(host.querySelector("[data-system2-report-save]")).toBeNull();
});
it("saves the privacy-bounded report locally and points to the support address", async () => {
  mock.getSystem2Status.mockResolvedValue(unfinished); mock.getSystem2Report.mockResolvedValue(reportBody); await render();
  expect(host.querySelector("[data-system2-report-guidance]")!.textContent).toContain("issue@knxbench.com");
  await click("[data-system2-report-save]");
  expect(mock.getSystem2Report).toHaveBeenCalledExactlyOnceWith(unfinished.operationId);
  expect(saved).toHaveBeenCalledExactlyOnceWith(JSON.stringify(reportBody, null, 2), "knxbench-system2-report.json");
  const written = String(saved.mock.calls[0][0]);
  for (const forbidden of [unfinished.address, unfinished.deviceName, unfinished.operationId, "Recover original", "192.0.2.1"]) expect(written).not.toContain(forbidden);
  expect(host.querySelector("[data-system2-report-saved]")).not.toBeNull();
});
it("keeps a failed report request visible and saves nothing", async () => {
  mock.getSystem2Status.mockResolvedValue(unfinished); mock.getSystem2Report.mockRejectedValue(new Error("refused"));
  await render(); await click("[data-system2-report-save]");
  expect(saved).not.toHaveBeenCalled(); expect(host.querySelector("[data-system2-report-failed]")).not.toBeNull();
});
it("drops a report that arrives after another operation was attached", async () => {
  mock.getSystem2Status.mockResolvedValue(unfinished);
  let release!: (value: unknown) => void; mock.getSystem2Report.mockReturnValue(new Promise((resolve) => { release = resolve; }));
  await render(); await click("[data-system2-report-save]");
  mock.getSystem2Status.mockResolvedValue({ ...unfinished, operationId: "e".repeat(32) });
  await click("[data-system2-refresh]");
  await act(async () => { release(reportBody); await Promise.resolve(); });
  expect(saved).not.toHaveBeenCalled();
});

it("hands a settled terminal status to the shared lifecycle as a pending belief refresh, once", async () => {
  const { getSystem2Snapshot, resetSystem2StatusForTests } = await import("./system2Status");
  resetSystem2StatusForTests();
  mock.getSystem2Status.mockResolvedValue({ ...status, downloadResult: "inProgress" });
  await render();
  expect(getSystem2Snapshot().belief).toBe("current");
  mock.getSystem2Status.mockResolvedValue({ ...unfinished, projectSettlement: "persisted" });
  await click("[data-system2-refresh]");
  expect(getSystem2Snapshot()).toMatchObject({ belief: "refreshing", pendingKey: `${unfinished.operationId}:false` });
  // The application-level lifecycle owns the one visible notice, not this disposable tab.
  expect(host.querySelector("[data-system2-belief='refreshing']")).toBeNull();
  expect(host.querySelector("[data-system2-settlement='persisted']")).not.toBeNull();
});
it("withdraws the shared authority when a known operation disappears or its read fails", async () => {
  const { getSystem2Snapshot, resetSystem2StatusForTests } = await import("./system2Status");
  resetSystem2StatusForTests();
  mock.getSystem2Status.mockResolvedValue({ ...unfinished, downloadResult: "verified", state: "finished" });
  await render();
  expect(getSystem2Snapshot().authority).toBe("ok");
  mock.getSystem2Status.mockResolvedValue(null); await click("[data-system2-refresh]");
  expect(getSystem2Snapshot().authority).toBe("unavailable");
});
it("coalesces the panel poll with an already pending lifecycle read", async () => {
  const { refreshSystem2Status, resetSystem2StatusForTests } = await import("./system2Status");
  resetSystem2StatusForTests();
  mock.getSystem2Status.mockResolvedValue(unfinished);
  await render();
  const before = mock.getSystem2Status.mock.calls.length;
  let release!: (value: unknown) => void;
  const pending = new Promise((resolve) => { release = resolve; });
  mock.getSystem2Status.mockReturnValue(pending);
  let read!: Promise<void>;
  await act(async () => { read = refreshSystem2Status(); });
  await click("[data-system2-refresh]");
  const calls = mock.getSystem2Status.mock.calls.length;
  await act(async () => { release(unfinished); await read; });
  await flush();
  expect(calls).toBe(before + 1);
});
it("keeps the download's counters apart from the recovery's and says the settlement failed visibly", async () => {
  mock.getSystem2Status.mockResolvedValue({ ...unfinished, state: "restoreFailed", restoreAttempted: true, completedSteps: 2, totalSteps: 9,
    downloadCompletedSteps: 26, downloadTotalSteps: 27, projectSettlement: "persistenceFailed" });
  await render();
  expect(host.querySelector("[data-system2-download-counts]")!.textContent).toContain("26 of 27");
  expect(host.querySelector("[data-system2-settlement='persistenceFailed']")!.textContent).toContain("could NOT be saved");
});
