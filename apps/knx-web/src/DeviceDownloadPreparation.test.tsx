/** Download tab preparation: gateway choice, device hand-off and the step guide. */
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
import type { BusDiscoveredInterface, DeviceDownloadStatusResponse } from "./api";
import { resetBusDiscoveryForTests, searchBusInterfaces } from "./busDiscovery";
import { downloadHandoff, type DownloadHandoff } from "./downloadHandoff";
import { PREFERRED_GATEWAY_KEY } from "./gatewayPreference";
import { resetSettingsForTests, settingsStorage } from "./settingsStore";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

let host: HTMLDivElement;
let root: Root;

function project(incarnation = 1): ProjectTree {
  const device = (id: number, address: string | null, name: string) => ({ id, name, address, description: null, com_object_count: 0 });
  return {
    schema_version: 1, errors: 0, warnings: 0, can_undo: false, can_redo: false, is_modified: false,
    group_address_style: "ThreeLevel", server_incarnation: "srv", project_incarnation: incarnation,
    installations: [{
      id: 0, name: "Test", buildings: [], group_addresses: [], group_ranges: [],
      topology: [{ id: 1, name: "Area", address: 1, lines: [{ id: 2, name: "Line", address: 1,
        devices: [device(10, "1.1.67", "Push button 2-fold"), device(11, "1.1.3", "Actuator")] }] }],
      unassigned: [device(12, null, "Not yet addressed")],
    }],
  };
}

const PLAN = {
  planId: 7, address: "1.1.67", deviceId: 10, deviceName: "Push button 2-fold", programId: "M-0083_A-0027-15-0BAC",
  maskVersion: 0x0701, manufacturer: 0x0083, parameterValues: 3, groupLinks: 1,
  segments: [{ id: "S", address: 0x4000, size: 4, written: 4 }], dataOctets: 4, steps: ["connect"],
  confirmationPhrase: "I confirm download to 1.1.67", support: { level: "verified", evidence: "MDT" },
  untestedAcknowledgement: null, partial: false, notWritten: [],
};

const iface = (endpoint: string, name: string, address = "1.1.250"): BusDiscoveredInterface =>
  ({ controlEndpoint: endpoint, friendlyName: name, individualAddress: address, supportsTunnelling: true });

function status(code: number): Error {
  const error = new Error(`status ${code}`) as Error & { status: number };
  error.status = code;
  return error;
}

function running(): DeviceDownloadStatusResponse {
  return { downloadId: 3, address: "1.1.3", deviceName: "Actuator", steps: 4, dataOctets: 8,
    status: { state: "running" }, nextSince: 1, events: [], backupFile: null };
}

function deferred<T>() {
  let resolve!: (value: T) => void; let reject!: (error: unknown) => void;
  const promise = new Promise<T>((ok, fail) => { resolve = ok; reject = fail; });
  return { promise, resolve, reject };
}

async function flush() {
  await act(async () => { for (let i = 0; i < 8; i += 1) await Promise.resolve(); });
}

async function render(props: { tree?: ProjectTree | null; handoff?: DownloadHandoff | null; onHandoffHandled?: (nonce: number) => void } = {}) {
  await act(async () => root.render(<DeviceDownloadPanel project={props.tree === undefined ? project() : props.tree}
    handoff={props.handoff ?? null} onHandoffHandled={props.onHandoffHandled} />));
  await flush();
}

async function discover(interfaces: BusDiscoveredInterface[] | Error) {
  if (interfaces instanceof Error) apiMock.discoverBusInterfaces.mockRejectedValueOnce(interfaces);
  else apiMock.discoverBusInterfaces.mockResolvedValueOnce({ interfaces });
  await act(async () => { await searchBusInterfaces(); });
  await flush();
}

const gatewayInput = () => host.querySelector<HTMLInputElement>("input[data-download-gateway]")!;
const deviceSelect = () => host.querySelector<HTMLSelectElement>("select[data-download-device]")!;
const foundSelect = () => host.querySelector<HTMLSelectElement>("select[data-download-gateway-found]");
const text = (selector: string) => host.querySelector(selector)?.textContent ?? "";
const button = (label: string) => [...host.querySelectorAll("button")].find((b) => b.textContent === label);

async function setSelect(select: HTMLSelectElement, value: string) {
  await act(async () => { select.value = value; select.dispatchEvent(new Event("change", { bubbles: true })); });
}
async function type(value: string) {
  const setter = Object.getOwnPropertyDescriptor(window.HTMLInputElement.prototype, "value")!.set!;
  await act(async () => { setter.call(gatewayInput(), value); gatewayInput().dispatchEvent(new Event("input", { bubbles: true })); });
}
async function showPlan() {
  await act(async () => button(en["deviceDownload.preparePlan"])!.click());
  await flush();
}

beforeEach(() => {
  window.localStorage.clear(); resetSettingsForTests(); resetBusDiscoveryForTests();
  apiMock.pollDeviceDownload.mockReset().mockRejectedValue(status(404));
  apiMock.planDeviceDownload.mockReset().mockResolvedValue(PLAN);
  apiMock.startDeviceDownload.mockReset();
  apiMock.discoverBusInterfaces.mockReset();
  apiMock.serverVersion.mockReset().mockResolvedValue({ version: "0.1.0-alpha.1" });
  host = document.createElement("div"); document.body.appendChild(host); root = createRoot(host);
});

afterEach(async () => {
  await act(async () => root.unmount());
  host.remove();
  resetBusDiscoveryForTests();
});

it("reads the shared search without starting one, and says when none has run", async () => {
  await render();
  expect(apiMock.discoverBusInterfaces).not.toHaveBeenCalled();
  expect(text(".device-download-gateway-status")).toBe(en["deviceDownload.gatewaySearch.idle"]);
  expect(gatewayInput().value).toBe("");
  expect(foundSelect()).toBeNull();
});

it("says so when no gateway answered, and keeps manual entry", async () => {
  await render();
  await discover([]);
  expect(text(".device-download-gateway-status")).toBe(en["deviceDownload.gatewaySearch.none"]);
  expect(foundSelect()).toBeNull();
  await type("192.0.2.40:3671");
  expect(gatewayInput().value).toBe("192.0.2.40:3671");
});

it("fills in the one gateway found, and says why", async () => {
  await render();
  await discover([iface("192.0.2.10:3671", "Line coupler IP")]);
  expect(gatewayInput().value).toBe("192.0.2.10:3671");
  expect(text("[data-gateway-note]")).toBe(en["deviceDownload.gatewayNote.auto"]);
  expect(text(".device-download-gateway-status")).toBe("1 gateway answered the search.");
});

it("offers several gateways by name, endpoint and interface address without choosing one", async () => {
  await render();
  await discover([iface("192.0.2.10:3671", "KNX IP", "1.1.250"), iface("192.0.2.11:3671", "KNX IP", "1.1.251")]);
  expect(gatewayInput().value).toBe("");
  expect([...foundSelect()!.options].map((o) => o.textContent)).toEqual([
    en["deviceDownload.gatewayChooseFound"], "KNX IP — 192.0.2.10:3671 · 1.1.250", "KNX IP — 192.0.2.11:3671 · 1.1.251",
  ]);
  await setSelect(foundSelect()!, "192.0.2.11:3671");
  expect(gatewayInput().value).toBe("192.0.2.11:3671");
  expect(host.querySelector("[data-gateway-note]")).toBeNull();
});

it("never overwrites a typed gateway with a later search result", async () => {
  await render();
  await type("10.0.0.5:3671");
  await discover([iface("192.0.2.10:3671", "Late answer")]);
  expect(gatewayInput().value).toBe("10.0.0.5:3671");
  // Clearing the field is a choice too: a later single answer does not refill it.
  await type("");
  await discover([iface("192.0.2.10:3671", "Late answer")]);
  expect(gatewayInput().value).toBe("");
});

it("fills in a late single answer while a search was still running when the tab opened", async () => {
  const pending = deferred<{ interfaces: BusDiscoveredInterface[] }>();
  apiMock.discoverBusInterfaces.mockReturnValueOnce(pending.promise);
  let search: Promise<void> | undefined;
  await act(async () => { search = searchBusInterfaces(); });
  await render();
  expect(text(".device-download-gateway-status")).toBe(en["deviceDownload.gatewaySearch.searching"]);
  expect(button(en["deviceDownload.gatewaySearchAgain"])!.disabled).toBe(true);
  await act(async () => { pending.resolve({ interfaces: [iface("192.0.2.10:3671", "Late")] }); await search; });
  await flush();
  expect(gatewayInput().value).toBe("192.0.2.10:3671");
});

it("starts from the preferred gateway without calling it found", async () => {
  settingsStorage.setItem(PREFERRED_GATEWAY_KEY, "192.0.2.5:3671");
  await render();
  await discover([iface("192.0.2.10:3671", "Other")]);
  expect(gatewayInput().value).toBe("192.0.2.5:3671");
  expect(text("[data-gateway-note]")).toBe(en["deviceDownload.gatewayNote.preferred"]);
  expect(foundSelect()!.value).toBe("");
  await discover([iface("192.0.2.5:3671", "Preferred one")]);
  expect(text("[data-gateway-note]")).toBe(en["deviceDownload.gatewayNote.preferredFound"]);
});

it("keeps a chosen gateway that a later search no longer lists, and marks it", async () => {
  await render();
  await discover([iface("192.0.2.10:3671", "A"), iface("192.0.2.11:3671", "B")]);
  await setSelect(foundSelect()!, "192.0.2.10:3671");
  await discover([iface("192.0.2.11:3671", "B")]);
  expect(gatewayInput().value).toBe("192.0.2.10:3671");
  expect(text("[data-gateway-note]")).toBe(en["deviceDownload.gatewayNote.stale"]);
});

it("shows a failed search quietly with the server's words and keeps the field usable", async () => {
  await render();
  await discover(new Error("no multicast route"));
  expect(text(".device-download-gateway-status")).toBe(en["deviceDownload.gatewaySearch.failed"]);
  expect(host.textContent).toContain("no multicast route");
  expect(gatewayInput().disabled).toBe(false);
});

it("searches again only when asked", async () => {
  await render();
  apiMock.discoverBusInterfaces.mockResolvedValueOnce({ interfaces: [] });
  await act(async () => button(en["deviceDownload.gatewaySearchAgain"])!.click());
  await flush();
  expect(apiMock.discoverBusInterfaces).toHaveBeenCalledTimes(1);
});

it("discards a shown plan when the gateway changes", async () => {
  await render();
  await setSelect(deviceSelect(), "10");
  await type("192.0.2.10:3671");
  await showPlan();
  expect(host.querySelector(".device-download-plan")).not.toBeNull();
  await type("192.0.2.11:3671");
  expect(host.querySelector(".device-download-plan")).toBeNull();
});

it("preselects the requested device by id, plans and sends nothing, and answers once", async () => {
  const handled = vi.fn();
  await render({ handoff: downloadHandoff(project(), 10, 1), onHandoffHandled: handled });
  expect(deviceSelect().value).toBe("10");
  expect(text("[data-handoff]")).toContain("1.1.67 — Push button 2-fold");
  expect(handled).toHaveBeenCalledExactlyOnceWith(1);
  expect(apiMock.planDeviceDownload).not.toHaveBeenCalled();
  expect(apiMock.startDeviceDownload).not.toHaveBeenCalled();
  expect(apiMock.discoverBusInterfaces).not.toHaveBeenCalled();
});

it("acts again on a repeated click for the same device", async () => {
  await render({ handoff: downloadHandoff(project(), 10, 1) });
  await setSelect(deviceSelect(), "11");
  await render({ handoff: downloadHandoff(project(), 10, 2) });
  expect(deviceSelect().value).toBe("10");
});

it("selects nothing for another project, a missing device or one without an individual address", async () => {
  await render({ handoff: downloadHandoff(project(1), 10, 1), tree: project(2) });
  expect(deviceSelect().value).toBe("");
  expect(text("[data-handoff]")).toBe(en["deviceDownload.handoff.stale"]);
  await render({ handoff: downloadHandoff(project(2), 99, 2), tree: project(2) });
  expect(text("[data-handoff]")).toBe(en["deviceDownload.handoff.missing"]);
  await render({ handoff: downloadHandoff(project(2), 12, 3), tree: project(2) });
  expect(text("[data-handoff]")).toContain("Not yet addressed has no individual address");
  expect(deviceSelect().value).toBe("");
});

it("waits for the status read and never switches devices during a running download", async () => {
  const poll = deferred<DeviceDownloadStatusResponse>();
  apiMock.pollDeviceDownload.mockReset().mockReturnValueOnce(poll.promise).mockResolvedValue(running());
  const handled = vi.fn();
  await render({ handoff: downloadHandoff(project(), 10, 1), onHandoffHandled: handled });
  expect(handled).not.toHaveBeenCalled();
  await act(async () => poll.resolve(running()));
  await flush();
  expect(handled).toHaveBeenCalledWith(1);
  expect(deviceSelect().value).toBe("");
  expect(text("[data-handoff]")).toBe(en["deviceDownload.handoff.busy"]);
});

it("re-reads an uncertain status once, then refuses rather than guessing", async () => {
  apiMock.pollDeviceDownload.mockReset().mockRejectedValue(status(500));
  await render({ handoff: downloadHandoff(project(), 10, 1) });
  expect(apiMock.pollDeviceDownload).toHaveBeenCalledTimes(2);
  expect(deviceSelect().value).toBe("");
  expect(text("[data-handoff]")).toBe(en["deviceDownload.handoff.unknownStatus"]);
  apiMock.pollDeviceDownload.mockReset().mockRejectedValue(status(404));
  await render({ handoff: downloadHandoff(project(), 10, 2) });
  expect(deviceSelect().value).toBe("10");
});

it("walks the five steps from the actual state and names what is missing", async () => {
  await render();
  const state = (id: string) => host.querySelector(`[data-guide-step="${id}"]`)?.getAttribute("data-guide-state");
  expect([...host.querySelectorAll("[data-guide-step]")].map((li) => li.getAttribute("data-guide-step")))
    .toEqual(["device", "gateway", "scope", "plan", "confirm"]);
  expect(host.querySelector('[aria-current="step"]')?.getAttribute("data-guide-step")).toBe("device");
  expect(text('[data-guide-step="device"]')).toContain(en["deviceDownload.guide.missing.device"]);
  expect([state("plan"), state("confirm")]).toEqual(["blocked", "blocked"]);
  await setSelect(deviceSelect(), "10");
  await type("192.0.2.10:3671");
  expect(host.querySelector('[aria-current="step"]')?.getAttribute("data-guide-step")).toBe("plan");
  await showPlan();
  expect(state("plan")).toBe("done");
  expect(state("confirm")).toBe("open");
  expect(text('[data-guide-step="confirm"]')).toContain(en["deviceDownload.guide.onStart"]);
  expect(apiMock.startDeviceDownload).not.toHaveBeenCalled();
});
