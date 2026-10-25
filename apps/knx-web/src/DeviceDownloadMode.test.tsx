/** Issue #30: Download mode selector placement and ordinary-only memory guidance. */
// @vitest-environment happy-dom

import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, expect, it, vi } from "vitest";

const apiMock = vi.hoisted(() => ({
  planDeviceDownload: vi.fn(), startDeviceDownload: vi.fn(), pollDeviceDownload: vi.fn(), discoverBusInterfaces: vi.fn(),
  serverVersion: vi.fn(), busActivity: vi.fn(), getDeviceReadiness: vi.fn(),
}));
const system2 = vi.hoisted(() => ({ prepareSystem2: vi.fn(), getSystem2Status: vi.fn(), startSystem2: vi.fn(), cancelSystem2: vi.fn(), recoverSystem2: vi.fn() }));
vi.mock("./system2DownloadApi", () => system2);
vi.mock("./api", () => ({
  ...apiMock,
  errorMessage: (error: unknown) => (error instanceof Error ? error.message : String(error)),
  errorStatus: (error: unknown) => (error && typeof error === "object" && "status" in error ? (error as { status: number }).status : undefined),
}));

import DeviceDownloadPanel from "./DeviceDownloadPanel";
import type { ProjectTree } from "./bindings/ProjectTree";
import { resetBusDiscoveryForTests } from "./busDiscovery";
import { resetSettingsForTests } from "./settingsStore";
import { messages as en } from "./messages/en";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

let host: HTMLDivElement;
let root: Root;

const project: ProjectTree = {
  schema_version: 1, errors: 0, warnings: 0, can_undo: false, can_redo: false, is_modified: false,
  group_address_style: "ThreeLevel", server_incarnation: "srv", project_incarnation: 1,
  installations: [{ id: 0, name: "Test", buildings: [], group_addresses: [], group_ranges: [], unassigned: [],
    topology: [{ id: 1, name: "Area", address: 1, lines: [{ id: 2, name: "Line", address: 1,
      devices: [{ id: 10, name: "Configured button", address: "1.1.67", description: null, com_object_count: 0 }] }] }] }],
};

const row = (readiness: string, category: string | null) => ({
  address: "1.1.67", name: "Configured button", programRef: "M-0001_A-0001-01-0000", readiness, category, detail: null, steps: null, octets: null,
});
const flush = async () => { await act(async () => { for (let i = 0; i < 10; i += 1) await Promise.resolve(); }); };
const q = <T extends Element>(selector: string) => host.querySelector<T>(selector);

async function render(tree: ProjectTree | null = project) {
  await act(async () => root.render(<DeviceDownloadPanel project={tree} />));
  await flush();
}
async function selectValue(selector: string, value: string) {
  const select = q<HTMLSelectElement>(selector)!;
  await act(async () => { select.value = value; select.dispatchEvent(new Event("change", { bubbles: true })); });
  await flush();
}
const prepareButton = () => [...host.querySelectorAll<HTMLButtonElement>(".device-download-actions button")]
  .find((b) => b.textContent === en["deviceDownload.preparePlan"])!;
const chooseDevice = () => selectValue("select[data-download-device]", "10");
const chooseMode = (mode: "memory" | "system2") => selectValue("[data-download-mode]", mode);
const deviceOption = () => q<HTMLOptionElement>("select[data-download-device] option[value='10']")!.textContent ?? "";
const MARK = ` (${en["deviceDownload.readiness.optionMark"]})`;
const follows = (a: Element, b: Element) => Boolean(a.compareDocumentPosition(b) & Node.DOCUMENT_POSITION_FOLLOWING);

function readiness(readinessState: string, category: string | null) {
  apiMock.getDeviceReadiness.mockResolvedValue({ devices: [row(readinessState, category)], counts: {} });
}

beforeEach(() => {
  window.localStorage.clear(); resetSettingsForTests(); resetBusDiscoveryForTests();
  apiMock.pollDeviceDownload.mockReset().mockRejectedValue({ status: 404 });
  apiMock.planDeviceDownload.mockReset();
  apiMock.startDeviceDownload.mockReset();
  apiMock.discoverBusInterfaces.mockReset().mockResolvedValue({ interfaces: [] });
  apiMock.serverVersion.mockReset().mockResolvedValue({ version: "0.1.0-alpha.1" });
  apiMock.busActivity.mockReset().mockResolvedValue({ sessions: [], busyLocks: [], untracked: [], oneShot: [] });
  apiMock.getDeviceReadiness.mockReset().mockResolvedValue({ devices: [], counts: {} });
  for (const fn of Object.values(system2)) fn.mockReset();
  system2.getSystem2Status.mockResolvedValue(null);
  host = document.createElement("div"); document.body.appendChild(host); root = createRoot(host);
});
afterEach(async () => { await act(async () => root.unmount()); host.remove(); resetBusDiscoveryForTests(); });

it("puts the Download mode selector directly after the device selector, above refusal notice and gateway", async () => {
  readiness("unsupported", "not-memory-mapped");
  await render();
  await chooseDevice();
  const device = q("select[data-download-device]")!, mode = q("[data-download-mode]")!;
  expect(q("[data-support-guidance]")).not.toBeNull();
  expect(follows(device, mode)).toBe(true);
  expect(follows(mode, q("[data-support-guidance]")!)).toBe(true);
  expect(follows(mode, q(".device-download-gateway")!)).toBe(true);
  expect(follows(mode, q("[data-download-gateway]")!)).toBe(true);
});

it("shows no mode selector without a project", async () => {
  await render(null);
  expect(q("[data-download-mode]")).toBeNull();
});

it("scopes the unsupported mark and notice to ordinary memory mode and restores them on return", async () => {
  readiness("unsupported", "not-memory-mapped");
  await render();
  await chooseDevice();
  expect(deviceOption()).toBe(`1.1.67 — Configured button${MARK}`);
  expect(q("[data-support-guidance]")).not.toBeNull();
  await chooseMode("system2");
  expect(deviceOption()).toBe("1.1.67 — Configured button");
  expect(q("[data-support-guidance]")).toBeNull();
  expect(q("[data-system2-prepare]")).not.toBeNull();
  await chooseMode("memory");
  expect(deviceOption()).toBe(`1.1.67 — Configured button${MARK}`);
  expect(q("[data-support-guidance]")).not.toBeNull();
});

it("explains the experimental System-2 route beside the selector only for not-memory-mapped, in memory mode", async () => {
  readiness("unsupported", "not-memory-mapped");
  await render();
  await chooseDevice();
  const hint = q("[data-system2-hint]");
  expect(hint).not.toBeNull();
  expect(follows(q("[data-download-mode]")!, hint!)).toBe(true);
  expect(hint!.textContent).toMatch(/offline/i);
  expect(hint!.textContent).toMatch(/fallback/i);
  await chooseMode("system2");
  expect(q("[data-system2-hint]")).toBeNull();
});

it("keeps the ordinary mark but no System-2 hint for other unsupported applications", async () => {
  readiness("unsupported", "modules");
  await render();
  await chooseDevice();
  expect(deviceOption()).toBe(`1.1.67 — Configured button${MARK}`);
  expect(q("[data-support-guidance]")).not.toBeNull();
  expect(q("[data-system2-hint]")).toBeNull();
});

it("drops an ordinary plan refusal when the mode changes, so it never returns stale", async () => {
  apiMock.planDeviceDownload.mockRejectedValue(Object.assign(new Error("refused"),
    { status: 422, body: { kind: "downloadPreparationRefused", category: "not-memory-mapped" } }));
  await render();
  await chooseDevice();
  await act(async () => prepareButton().click());
  await flush();
  expect(q("[data-support-guidance=unsupported]")).not.toBeNull();
  await chooseMode("system2");
  expect(q("[data-support-guidance]")).toBeNull();
  await chooseMode("memory");
  expect(q("[data-support-guidance]")).toBeNull();
  expect(apiMock.planDeviceDownload).toHaveBeenCalledTimes(1);
});

it("keeps an excluded device marked and noticed in both modes", async () => {
  readiness("excluded", null);
  await render();
  await chooseDevice();
  expect(deviceOption()).toContain("excluded");
  expect(q("[data-support-guidance=excluded]")).not.toBeNull();
  await chooseMode("system2");
  expect(deviceOption()).toContain("excluded");
  expect(q("[data-support-guidance=excluded]")).not.toBeNull();
  expect(q("[data-system2-hint]")).toBeNull();
});

it("keeps an ordinary readiness failure out of System-2 mode and back in memory mode", async () => {
  apiMock.getDeviceReadiness.mockRejectedValue(new Error("PRIVATE service detail"));
  await render();
  await chooseDevice();
  expect(q("[data-support-guidance=transient]")).not.toBeNull();
  expect(q("[data-readiness-retry]")).not.toBeNull();
  await chooseMode("system2");
  expect(q("[data-support-guidance]")).toBeNull();
  expect(q("[data-readiness-retry]")).toBeNull();
  expect(host.textContent).not.toContain("PRIVATE service detail");
  await chooseMode("memory");
  expect(q("[data-support-guidance=transient]")).not.toBeNull();
});

it("hides the ordinary readiness loading note in System-2 mode", async () => {
  apiMock.getDeviceReadiness.mockImplementation(() => new Promise(() => {}));
  await render();
  expect(q("[data-readiness-loading]")).not.toBeNull();
  await chooseMode("system2");
  expect(q("[data-readiness-loading]")).toBeNull();
});

it("never prepares, plans, starts or connects on mode selection or when switching back", async () => {
  readiness("unsupported", "not-memory-mapped");
  await render();
  await chooseDevice();
  await chooseMode("system2");
  await chooseMode("memory");
  await chooseMode("system2");
  expect(q<HTMLSelectElement>("[data-download-mode]")!.value).toBe("system2");
  for (const fn of [system2.prepareSystem2, system2.startSystem2, system2.recoverSystem2, system2.cancelSystem2,
    apiMock.planDeviceDownload, apiMock.startDeviceDownload]) expect(fn).not.toHaveBeenCalled();
});

it("does not switch to System-2 by itself for a not-memory-mapped device", async () => {
  readiness("unsupported", "not-memory-mapped");
  await render();
  await chooseDevice();
  expect(q<HTMLSelectElement>("[data-download-mode]")!.value).toBe("memory");
  expect(q("[data-system2-prepare]")).toBeNull();
});

it("locks the mode selector while an ordinary plan is being prepared", async () => {
  apiMock.planDeviceDownload.mockImplementation(() => new Promise(() => {}));
  await render();
  await chooseDevice();
  await act(async () => prepareButton().click());
  expect(q<HTMLSelectElement>("[data-download-mode]")!.disabled).toBe(true);
});
