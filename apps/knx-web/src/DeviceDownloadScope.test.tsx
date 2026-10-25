/** Download tab "only what is pending": the server's flag advice, its no-op and unmet needs. */
// @vitest-environment happy-dom

import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, expect, it, vi } from "vitest";

const apiMock = vi.hoisted(() => ({
  planDeviceDownload: vi.fn(),
  startDeviceDownload: vi.fn(),
  pollDeviceDownload: vi.fn(),
  discoverBusInterfaces: vi.fn(),
  serverVersion: vi.fn(),
  busActivity: vi.fn(),
  getDeviceReadiness: vi.fn(),
  stopBusMonitor: vi.fn(),
  cancelLineScan: vi.fn(),
  stopAddressProgramming: vi.fn(),
}));

vi.mock("./api", () => ({
  ...apiMock,
  errorMessage: (error: unknown) => (error instanceof Error ? error.message : String(error)),
  errorStatus: (error: unknown) =>
    error && typeof error === "object" && "status" in error ? (error as { status: number }).status : undefined,
}));

import DeviceDownloadPanel from "./DeviceDownloadPanel";
import { messages as en } from "./messages/en";
import type { ProjectTree } from "./bindings/ProjectTree";
import type { DeviceDownloadScopeAdvice } from "./api";
import { resetBusDiscoveryForTests } from "./busDiscovery";
import { resetSettingsForTests } from "./settingsStore";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

let host: HTMLDivElement;
let root: Root;

function project(): ProjectTree {
  const device = (id: number, address: string | null, name: string) => ({ id, name, address, description: null, com_object_count: 0 });
  return {
    schema_version: 1, errors: 0, warnings: 0, can_undo: false, can_redo: false, is_modified: false,
    group_address_style: "ThreeLevel", server_incarnation: "srv", project_incarnation: 1,
    installations: [{
      id: 0, name: "Test", buildings: [], group_addresses: [], group_ranges: [],
      topology: [{ id: 1, name: "Area", address: 1, lines: [{ id: 2, name: "Line", address: 1,
        devices: [device(10, "1.1.67", "Push button 2-fold")] }] }],
      unassigned: [],
    }],
  };
}

function advice(overrides: Partial<DeviceDownloadScopeAdvice> = {}): DeviceDownloadScopeAdvice {
  return { basis: "projectFlags", write: "parameters", address: "current", mediumPending: false, fullyProgrammed: false, ...overrides };
}

const PLAN = {
  planId: 7, address: "1.1.67", deviceId: 10, deviceName: "Push button 2-fold", programId: "M-0083_A-0027-15-0BAC",
  maskVersion: 0x0701, manufacturer: 0x0083, parameterValues: 3, groupLinks: 1,
  segments: [{ id: "S", address: 0x4000, size: 4, written: 4 }], dataOctets: 4, steps: ["connect"],
  confirmationPhrase: "I confirm download to 1.1.67", support: { level: "verified", evidence: "MDT", inferences: [] },
  untestedAcknowledgement: null, partial: true, notWritten: [], warnings: [],
  scope: "parameters", scopeFromChanges: true, scopeAdvice: advice(),
};

function refusal(status: number, body: unknown): Error {
  const error = new Error("refused") as Error & { status: number; body: unknown };
  error.status = status;
  error.body = body;
  return error;
}

async function flush() {
  await act(async () => { for (let i = 0; i < 8; i += 1) await Promise.resolve(); });
}

const text = (selector: string) => host.querySelector(selector)?.textContent ?? "";
const button = (label: string) => [...host.querySelectorAll("button")].find((b) => b.textContent === label);
const radio = (value: string) => host.querySelector<HTMLInputElement>(`input[name="device-download-scope"][value="${value}"]`);

async function prepare(scope: string, gatewayValue?: string) {
  await act(async () => root.render(<DeviceDownloadPanel project={project()} />));
  await flush();
  const select = host.querySelector<HTMLSelectElement>("select[data-download-device]")!;
  await act(async () => { select.value = "10"; select.dispatchEvent(new Event("change", { bubbles: true })); });
  if (gatewayValue !== undefined) {
    // A gateway change drops a shown plan, so it is chosen first.
    const gateway = host.querySelector<HTMLInputElement>("input[data-download-gateway]")!;
    const setter = Object.getOwnPropertyDescriptor(window.HTMLInputElement.prototype, "value")!.set!;
    await act(async () => { setter.call(gateway, gatewayValue); gateway.dispatchEvent(new Event("input", { bubbles: true })); });
  }
  await act(async () => { radio(scope)!.click(); });
  await act(async () => button(en["deviceDownload.preparePlan"])!.click());
  await flush();
}

beforeEach(() => {
  window.localStorage.clear(); resetSettingsForTests(); resetBusDiscoveryForTests();
  apiMock.pollDeviceDownload.mockReset().mockRejectedValue(refusal(404, null));
  apiMock.planDeviceDownload.mockReset().mockResolvedValue(PLAN);
  apiMock.startDeviceDownload.mockReset();
  apiMock.discoverBusInterfaces.mockReset();
  apiMock.serverVersion.mockReset().mockResolvedValue({ version: "0.1.0-alpha.1" });
  apiMock.busActivity.mockReset().mockResolvedValue({ serverIncarnation: "srv", coverage: "partial", historyState: "disabled",
    sessions: [], oneShot: [], oneShotDropped: 0, busyLocks: [], untracked: [] });
  apiMock.getDeviceReadiness.mockReset().mockResolvedValue({ devices: [], counts: {} });
  apiMock.stopBusMonitor.mockReset();
  apiMock.cancelLineScan.mockReset();
  apiMock.stopAddressProgramming.mockReset();
  host = document.createElement("div"); document.body.appendChild(host); root = createRoot(host);
});

afterEach(async () => {
  await act(async () => root.unmount());
  host.remove();
  resetBusDiscoveryForTests();
});

it("asks the server for the pending scope and shows the scope it resolved to", async () => {
  await prepare("changes");
  expect(apiMock.planDeviceDownload).toHaveBeenCalledWith("1.1.67", "changes");
  expect(text("[data-scope-resolved]")).toBe(en["deviceDownload.scopeAdvice.resolved.parameters"]);
  expect(text(".device-download-not-written")).toContain(en["deviceDownload.partialScope.parameters"]);
  expect(text("[data-scope-advice]")).toContain(en["deviceDownload.scopeAdvice.basis"]);
  expect(host.querySelector("[data-scope-unmet]")).toBeNull();
});

it("marks a pending address and medium configuration as unmet, never as fully programmed", async () => {
  apiMock.planDeviceDownload.mockResolvedValue({ ...PLAN, scopeAdvice: advice({ address: "pending", mediumPending: true }) });
  await prepare("changes");
  expect(text('[data-scope-unmet="address"]')).toBe(en["deviceDownload.scopeAdvice.address.pending"]);
  expect(text('[data-scope-unmet="medium"]')).toBe(en["deviceDownload.scopeAdvice.medium"]);
  expect(host.querySelector("[data-scope-fully-programmed]")).toBeNull();
});

it("builds nothing when nothing is pending and says what is still unmet", async () => {
  apiMock.planDeviceDownload.mockRejectedValue(refusal(422, { kind: "downloadNothingPending", error: "nothing pending",
    scopeAdvice: advice({ write: "nothing", mediumPending: true }) }));
  await prepare("changes");
  expect(text("[data-scope-nothing]")).toContain(en["deviceDownload.scopeAdvice.nothing"]);
  expect(text('[data-scope-unmet="medium"]')).toBe(en["deviceDownload.scopeAdvice.medium"]);
  expect(host.querySelector("[data-scope-fully-programmed]")).toBeNull();
  expect(host.querySelector(".device-download-plan")).toBeNull();
  expect(button(en["deviceDownload.start"].replace("{address}", "1.1.67"))).toBeUndefined();
  // Not a support refusal: no "report this" guidance for a no-op.
  expect(host.querySelector(".download-support-notice")).toBeNull();
});

it("says fully programmed only when the project marks every part programmed", async () => {
  apiMock.planDeviceDownload.mockRejectedValue(refusal(422, { kind: "downloadNothingPending", error: "nothing pending",
    scopeAdvice: advice({ write: "nothing", fullyProgrammed: true }) }));
  await prepare("changes");
  expect(text("[data-scope-fully-programmed]")).toBe(en["deviceDownload.scopeAdvice.fullyProgrammed"]);
});

it("keeps explicit scopes as they were and shows no advice from an older server", async () => {
  const { scope: _scope, scopeFromChanges: _from, scopeAdvice: _advice, ...older } = PLAN;
  apiMock.planDeviceDownload.mockResolvedValue({ ...older, partial: false });
  await prepare("complete");
  expect(apiMock.planDeviceDownload).toHaveBeenCalledWith("1.1.67");
  expect(host.querySelector(".device-download-plan")).not.toBeNull();
  expect(host.querySelector("[data-scope-advice]")).toBeNull();
  await prepare("both");
  expect(apiMock.planDeviceDownload).toHaveBeenLastCalledWith("1.1.67", { parameters: true, groupAddresses: true });
});

it("starts exactly the plan the pending scope resolved to", async () => {
  apiMock.startDeviceDownload.mockResolvedValue({ downloadId: 3 });
  await prepare("changes", "192.0.2.10:3671");
  await act(async () => button(en["deviceDownload.start"].replace("{address}", "1.1.67"))!.click());
  await flush();
  const dialog = document.querySelector<HTMLElement>(".programming-consent-dialog")!;
  const confirm = [...dialog.querySelectorAll("button")].find((b) => b.textContent === en["programmingConsent.confirm"])!;
  await act(async () => confirm.click());
  await flush();
  expect(apiMock.startDeviceDownload).toHaveBeenCalledWith(7, "192.0.2.10:3671", "I confirm download to 1.1.67");
});

it("treats a malformed nothing-pending answer as a refusal, never as fully programmed", async () => {
  apiMock.planDeviceDownload.mockRejectedValue(refusal(422, { kind: "downloadNothingPending", error: "nothing pending",
    scopeAdvice: { basis: "deviceEvidence", write: "nothing", address: "current", mediumPending: false, fullyProgrammed: true } }));
  await prepare("changes");
  expect(host.querySelector("[data-scope-nothing]")).toBeNull();
  expect(host.querySelector("[data-scope-fully-programmed]")).toBeNull();
  expect(host.querySelector(".download-support-notice")).not.toBeNull();
});

it("refuses to show a complete plan from a server that ignored the pending scope", async () => {
  // A server without the feature answers `scope: "changes"` with the complete plan.
  const { scope: _scope, scopeFromChanges: _from, scopeAdvice: _advice, ...older } = PLAN;
  apiMock.planDeviceDownload.mockResolvedValue({ ...older, partial: false });
  await prepare("changes", "192.0.2.10:3671");
  expect(host.querySelector(".device-download-plan")).toBeNull();
  expect(button(en["deviceDownload.start"].replace("{address}", "1.1.67"))).toBeUndefined();
  expect(text("[data-scope-unsupported]")).toBe(en["deviceDownload.scopeAdvice.unsupported"]);
});

it("refuses a pending-scope plan whose resolved scope is missing, unknown or inconsistent", async () => {
  const { scope: _scope, ...noScope } = PLAN;
  const answers: unknown[] = [
    noScope,
    { ...PLAN, scope: "everything" },
    { ...PLAN, scope: "complete" },
    { ...PLAN, scope: "both", partial: false },
    { ...PLAN, scopeAdvice: advice({ write: "both" }) },
    { ...PLAN, scopeAdvice: undefined },
  ];
  for (const answer of answers) {
    apiMock.planDeviceDownload.mockResolvedValue(answer);
    await prepare("changes", "192.0.2.10:3671");
    const shown = JSON.stringify(answer);
    expect(host.querySelector(".device-download-plan"), shown).toBeNull();
    expect(button(en["deviceDownload.start"].replace("{address}", "1.1.67")), shown).toBeUndefined();
    expect(text("[data-scope-unsupported]"), shown).toBe(en["deviceDownload.scopeAdvice.malformed"]);
    expect(host.textContent, shown).not.toContain("deviceDownload.scopeAdvice.resolved.");
    expect(host.querySelector("[data-scope-fully-programmed]"), shown).toBeNull();
  }
});

it("refuses a pending-scope plan whose advice has a foreign basis or contradicts itself", async () => {
  for (const scopeAdvice of [
    { ...advice(), basis: "deviceEvidence" },
    advice({ fullyProgrammed: true }),
    advice({ write: "nothing", address: "pending", fullyProgrammed: true }),
    advice({ write: "nothing", mediumPending: true, fullyProgrammed: true }),
    { ...advice(), mediumPending: "no" },
  ]) {
    apiMock.planDeviceDownload.mockResolvedValue({ ...PLAN, scopeAdvice });
    await prepare("changes");
    const shown = JSON.stringify(scopeAdvice);
    expect(host.querySelector(".device-download-plan"), shown).toBeNull();
    expect(host.querySelector("[data-scope-fully-programmed]"), shown).toBeNull();
    expect(text("[data-scope-unsupported]"), shown).toBe(en["deviceDownload.scopeAdvice.malformed"]);
  }
});

it("shows an explicit plan without advice it cannot trust", async () => {
  apiMock.planDeviceDownload.mockResolvedValue({ ...PLAN, partial: false, scope: "complete", scopeFromChanges: false,
    scopeAdvice: { basis: "deviceEvidence", write: "nothing", address: "current", mediumPending: false, fullyProgrammed: true } });
  await prepare("complete");
  expect(host.querySelector(".device-download-plan")).not.toBeNull();
  expect(host.querySelector("[data-scope-advice]")).toBeNull();
  expect(host.querySelector("[data-scope-fully-programmed]")).toBeNull();
});

it("says the flags advise nothing beside an explicit plan, with the unmet needs apart", async () => {
  apiMock.planDeviceDownload.mockResolvedValue({ ...PLAN, partial: false, scope: "complete", scopeFromChanges: false,
    scopeAdvice: advice({ write: "nothing", address: "pending", mediumPending: true }) });
  await prepare("complete");
  expect(host.querySelector(".device-download-plan")).not.toBeNull();
  expect(text("[data-scope-advised]")).toBe(en["deviceDownload.scopeAdvice.advisedNothing"]);
  expect(text('[data-scope-unmet="address"]')).toBe(en["deviceDownload.scopeAdvice.address.pending"]);
  expect(text('[data-scope-unmet="medium"]')).toBe(en["deviceDownload.scopeAdvice.medium"]);
  expect(host.querySelector("[data-scope-fully-programmed]")).toBeNull();
});

const NOTHING = () => refusal(422, { kind: "downloadNothingPending", error: "nothing pending",
  scopeAdvice: advice({ write: "nothing" }) });

it("clears a no-op answer when the scope, the device or the project changes", async () => {
  const changes: [string, () => Promise<void>][] = [
    ["scope", async () => { await act(async () => { radio("complete")!.click(); }); }],
    ["device", async () => {
      const select = host.querySelector<HTMLSelectElement>("select[data-download-device]")!;
      await act(async () => { select.value = ""; select.dispatchEvent(new Event("change", { bubbles: true })); });
    }],
    ["project", async () => { await act(async () => root.render(<DeviceDownloadPanel project={project()} />)); await flush(); }],
  ];
  for (const [what, change] of changes) {
    apiMock.planDeviceDownload.mockRejectedValue(NOTHING());
    await prepare("changes");
    expect(host.querySelector("[data-scope-nothing]"), what).not.toBeNull();
    await change();
    expect(host.querySelector("[data-scope-nothing]"), what).toBeNull();
  }
});

it("clears an unsupported answer when the scope, the device or the project changes", async () => {
  const { scope: _scope, scopeFromChanges: _from, scopeAdvice: _advice, ...older } = PLAN;
  const changes: [string, () => Promise<void>][] = [
    ["scope", async () => { await act(async () => { radio("both")!.click(); }); }],
    ["device", async () => {
      const select = host.querySelector<HTMLSelectElement>("select[data-download-device]")!;
      await act(async () => { select.value = ""; select.dispatchEvent(new Event("change", { bubbles: true })); });
    }],
    ["project", async () => { await act(async () => root.render(<DeviceDownloadPanel project={project()} />)); await flush(); }],
  ];
  for (const [what, change] of changes) {
    apiMock.planDeviceDownload.mockResolvedValue({ ...older, partial: false });
    await prepare("changes");
    expect(host.querySelector("[data-scope-unsupported]"), what).not.toBeNull();
    await change();
    expect(host.querySelector("[data-scope-unsupported]"), what).toBeNull();
  }
});
