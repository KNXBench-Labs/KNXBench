/** Offline readiness and explicitly requested read-only comparison: no live bus in these tests. */
// @vitest-environment happy-dom
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, expect, it, vi } from "vitest";

const apiMock = vi.hoisted(() => ({ getDeviceReadiness: vi.fn(), compareDevice: vi.fn() }));
vi.mock("./api", () => ({
  ...apiMock,
  errorMessage: (error: unknown) => error instanceof Error ? error.message : String(error),
}));

import DeviceInspectionPanel from "./DeviceInspectionPanel";
import { messages as en } from "./messages/en";
import type { ProjectTree } from "./bindings/ProjectTree";
import { subscribeAchievementEvents, type AchievementEvent } from "./achievementEvents";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;
const project = (): ProjectTree => ({ installations: [] }) as unknown as ProjectTree;
const devices = [
  { address: "1.1.67", name: "Push button", programRef: "M-0083_A-0027", readiness: "verified", category: null, detail: "One device; 2026-09-29; research", steps: 11, octets: 20 },
  { address: "1.1.68", name: "Actuator", programRef: "M-0083_A-0080", readiness: "untested", category: null, detail: null, steps: 3, octets: 0 },
  { address: "1.1.69", name: "No program", programRef: "", readiness: "unsupported", category: "configuration", detail: "cannot prepare plan", steps: null, octets: null },
  { address: "9.9.9", name: "Excluded", programRef: "M-X", readiness: "excluded", category: null, detail: null, steps: null, octets: null },
  { address: null, name: "Unaddressed", programRef: "M-Y", readiness: "no-address", category: null, detail: null, steps: null, octets: null },
];
const readiness = { devices, counts: { verified: 1, untested: 1, unsupported: 1, excluded: 1, "no-address": 1 } };
const comparison = {
  address: "1.1.67", deviceName: "Push button", programId: "M-0083_A-0027",
  written: false, partial: false, mask: 0x0701, manufacturer: 0x0083,
  loadStates: [{ machine: "GroupAddressTable", state: "Loaded" }],
  octets: 20, differingOctets: 2, same: false,
  changes: [{ address: 0x4000, segment: "seg-4", device: [0, 255], project: [1, 2] }],
};
let host: HTMLDivElement;
let root: Root;
async function flush() { await act(async () => { for (let i = 0; i < 6; i++) await Promise.resolve(); }); }
async function render(tree: ProjectTree | null = project()) {
  host = document.createElement("div"); document.body.appendChild(host); root = createRoot(host);
  await act(async () => root.render(<DeviceInspectionPanel project={tree} />));
  await flush();
}
function button(label: string): HTMLButtonElement {
  const found = [...host.querySelectorAll("button")].find((b) => b.textContent?.trim() === label);
  if (!found) throw new Error(`missing button: ${label}`);
  return found;
}
async function select(address: string) {
  const field = host.querySelector<HTMLSelectElement>(".device-checks-target")!;
  await act(async () => { field.value = address; field.dispatchEvent(new Event("change", { bubbles: true })); });
}
async function gateway(value: string) {
  const field = host.querySelector<HTMLInputElement>(".device-checks-gateway")!;
  const setter = Object.getOwnPropertyDescriptor(window.HTMLInputElement.prototype, "value")!.set!;
  await act(async () => { setter.call(field, value); field.dispatchEvent(new Event("input", { bubbles: true })); });
}
async function prepare() {
  await select("1.1.67"); await gateway("192.0.2.10:3671");
  await act(async () => button(en["deviceChecks.review"]).click());
}
beforeEach(() => {
  apiMock.getDeviceReadiness.mockReset().mockResolvedValue(readiness);
  apiMock.compareDevice.mockReset().mockResolvedValue(comparison);
});
afterEach(async () => { if (root) await act(async () => root.unmount()); host?.remove(); });

it("stays entirely offline until asked and shows every readiness grade with its evidence", async () => {
  await render();
  expect(apiMock.getDeviceReadiness).toHaveBeenCalledTimes(1);
  expect(apiMock.compareDevice).not.toHaveBeenCalled();
  for (const device of devices) expect(host.textContent).toContain(device.name);
  expect(host.textContent).toContain("One device; 2026-09-29; research");
  expect(host.textContent).toContain("cannot prepare plan");
  expect(host.textContent).toContain("0"); // zero-octet plan is not treated as missing
  expect(host.querySelectorAll(".device-checks-readiness tbody tr")).toHaveLength(5);
  expect(host.querySelector(".device-checks-readiness-section .device-checks-table-scroll")?.getAttribute("tabindex")).toBe("0");
  expect(host.querySelector(".device-checks-readiness-section .device-checks-table-scroll")?.getAttribute("aria-label")).toBe(en["deviceChecks.grade"]);
  const eligible = [...host.querySelectorAll<HTMLSelectElement>(".device-checks-target option")].map((option) => option.value);
  expect(eligible).toContain("1.1.67"); expect(eligible).toContain("1.1.68");
  expect(eligible).not.toContain("1.1.69"); expect(eligible).not.toContain("9.9.9");
  expect(apiMock.compareDevice).not.toHaveBeenCalled();
});

it("does not call either endpoint without a project", async () => {
  await render(null);
  expect(host.textContent).toContain(en["deviceChecks.projectRequired"]);
  expect(apiMock.getDeviceReadiness).not.toHaveBeenCalled();
  expect(apiMock.compareDevice).not.toHaveBeenCalled();
});

it("requires a second explicit action before a tunnel read; reports every compared byte and state", async () => {
  await render(); await prepare();
  expect(host.textContent).toContain("Read the current device at 1.1.67 through 192.0.2.10:3671");
  expect(apiMock.compareDevice).not.toHaveBeenCalled();
  await act(async () => button(en["deviceChecks.confirm"]).click()); await flush();
  expect(apiMock.compareDevice).toHaveBeenCalledExactlyOnceWith("1.1.67", "192.0.2.10:3671");
  const result = host.querySelector(".device-checks-result")!;
  expect(result.textContent).toContain(en["deviceChecks.noWrite"]);
  expect(result.textContent).toContain("GroupAddressTable");
  expect(result.textContent).toContain("0701h"); expect(result.textContent).toContain("0083h");
  expect(result.textContent).toContain("4000h"); expect(result.textContent).toContain("seg-4");
  expect(result.textContent).toContain("00 FF"); expect(result.textContent).toContain("01 02");
  expect(result.querySelector(".device-checks-table-scroll")?.getAttribute("tabindex")).toBe("0");
  expect(result.textContent).toContain("2");
});

it("refuses to describe a server-reported write as read-only", async () => {
  apiMock.compareDevice.mockResolvedValue({ ...comparison, written: true });
  await render(); await prepare();
  await act(async () => button(en["deviceChecks.confirm"]).click()); await flush();
  expect(host.querySelector("[role=alert]")?.textContent ?? "").toContain(en["deviceChecks.unexpectedWrite"]);
  expect(host.querySelector(".device-checks-result")).toBeNull();
});

it("never compares when the gateway is malformed, readiness failed or an address is ambiguous", async () => {
  await render();
  await select("1.1.67"); await gateway("example.com:3671");
  await act(async () => button(en["deviceChecks.review"]).click());
  expect(host.querySelector("[role=alert]")?.textContent).toContain(en["busMonitor.invalidHost"]);
  expect(apiMock.compareDevice).not.toHaveBeenCalled();
  await act(async () => root.unmount()); host.remove();
  apiMock.getDeviceReadiness.mockRejectedValueOnce(new Error("no product database is configured"));
  await render();
  expect(host.querySelector("[role=alert]")?.textContent).toContain("no product database");
  expect(button(en["deviceChecks.review"]).disabled).toBe(true);
  await act(async () => button(en["deviceChecks.refresh"]).click()); await flush();
  expect(host.querySelectorAll(".device-checks-readiness tbody tr")).toHaveLength(5);
  await act(async () => root.unmount()); host.remove();
  apiMock.getDeviceReadiness.mockResolvedValue({
    devices: [...devices, { ...devices[0], name: "Duplicate owner" }],
    counts: { ...readiness.counts, verified: 2 },
  });
  await render();
  expect(host.textContent).toContain(en["deviceChecks.ambiguous"]);
  expect([...host.querySelectorAll<HTMLSelectElement>(".device-checks-target option")].map((option) => option.value)).not.toContain("1.1.67");
  expect(apiMock.compareDevice).not.toHaveBeenCalled();
});

it("distinguishes a same-device result with no load states from missing output", async () => {
  apiMock.compareDevice.mockResolvedValue({ ...comparison, same: true, differingOctets: 0, changes: [], loadStates: [] });
  await render(); await prepare();
  await act(async () => button(en["deviceChecks.confirm"]).click()); await flush();
  const result = host.querySelector(".device-checks-result")!;
  expect(result.textContent).toContain("No load states were reported.");
  expect(result.textContent).toContain(en["deviceChecks.noRanges"]);
  expect(result.textContent).toContain(en["deviceChecks.same"]);
});

it("refuses an unexpected device, a partial scope or contradictory byte counts", async () => {
  for (const response of [
    { ...comparison, address: "1.1.68" },
    { ...comparison, partial: true },
    { ...comparison, differingOctets: 0 },
    { ...comparison, changes: [{ ...comparison.changes[0], project: [1] }] },
  ]) {
    apiMock.compareDevice.mockResolvedValue(response);
    await render(); await prepare();
    await act(async () => button(en["deviceChecks.confirm"]).click()); await flush();
    expect(host.querySelector(".device-checks-result")).toBeNull();
    expect(host.querySelector("[role=alert]")).not.toBeNull();
    await act(async () => root.unmount()); host.remove();
  }
});

it("keeps unknown readiness grades visible and ineligible rather than guessing", async () => {
  apiMock.getDeviceReadiness.mockResolvedValue({
    devices: [{ ...devices[0], readiness: "future-grade", detail: "opaque server evidence" }],
    counts: { "future-grade": 1 },
  });
  await render();
  expect(host.textContent).toContain("Unknown grade (future-grade)");
  expect(host.textContent).toContain("opaque server evidence");
  expect(host.querySelectorAll(".device-checks-readiness tbody tr")).toHaveLength(1);
  expect([...host.querySelectorAll<HTMLSelectElement>(".device-checks-target option")].map((option) => option.value)).not.toContain("1.1.67");
  expect(apiMock.compareDevice).not.toHaveBeenCalled();
});

it("cancel and input edits revoke the confirmation before any tunnel call", async () => {
  await render(); await prepare();
  await act(async () => button(en["deviceChecks.cancel"]).click());
  expect(apiMock.compareDevice).not.toHaveBeenCalled();
  await act(async () => button(en["deviceChecks.review"]).click());
  expect(host.querySelector(".device-checks-confirm")).not.toBeNull();
  await gateway("192.0.2.11:3671");
  expect(host.querySelector(".device-checks-confirm")).toBeNull();
  expect(apiMock.compareDevice).not.toHaveBeenCalled();
});

it("ignores an old readiness response after the project changes", async () => {
  let resolveOld!: (value: typeof readiness) => void;
  apiMock.getDeviceReadiness.mockReturnValueOnce(new Promise((done) => { resolveOld = done; }));
  apiMock.getDeviceReadiness.mockResolvedValueOnce({ devices: [devices[1]], counts: { untested: 1 } });
  await render();
  await act(async () => root.render(<DeviceInspectionPanel project={project()} />)); await flush();
  await act(async () => resolveOld(readiness)); await flush();
  expect(host.querySelectorAll(".device-checks-readiness tbody tr")).toHaveLength(1);
  expect(host.textContent).toContain("Actuator");
  expect(host.textContent).not.toContain("Push button");
});

it("invalidates an in-flight comparison when the project changes", async () => {
  let resolve!: (value: typeof comparison) => void;
  apiMock.compareDevice.mockReturnValue(new Promise((done) => { resolve = done; }));
  await render(); await prepare();
  await act(async () => button(en["deviceChecks.confirm"]).click());
  expect(apiMock.compareDevice).toHaveBeenCalledTimes(1);
  await act(async () => root.render(<DeviceInspectionPanel project={project()} />));
  await act(async () => resolve(comparison)); await flush();
  expect(host.querySelector(".device-checks-result")).toBeNull();
});

// ADR-0089: what the checks report to the achievement tracker.
function recordAchievementEvents(): { seen: AchievementEvent[]; stop: () => void } {
  const seen: AchievementEvent[] = [];
  return { seen, stop: subscribeAchievementEvents((event) => seen.push(event)) };
}

it("reports how many graded devices cannot be planned", async () => {
  const events = recordAchievementEvents();
  try {
    await render();
  } finally {
    events.stop();
  }
  // unsupported, excluded and no-address are not plannable; verified and untested are.
  expect(events.seen).toEqual([{ type: "readinessChecked", deviceCount: 5, unplannableCount: 3 }]);
});

it("reports a comparison only once it passed the consistency checks", async () => {
  const same = { ...comparison, differingOctets: 0, same: true, changes: [] };
  apiMock.compareDevice.mockReset().mockResolvedValue(same);
  const events = recordAchievementEvents();
  try {
    await render(); await prepare();
    await act(async () => button(en["deviceChecks.confirm"]).click()); await flush();
  } finally {
    events.stop();
  }
  expect(events.seen.filter((event) => event.type === "deviceCompared")).toEqual([{ type: "deviceCompared", differingOctets: 0 }]);
});

it("reports nothing for a comparison it refuses", async () => {
  apiMock.compareDevice.mockReset().mockResolvedValue({ ...comparison, written: true });
  const events = recordAchievementEvents();
  try {
    await render(); await prepare();
    await act(async () => button(en["deviceChecks.confirm"]).click()); await flush();
  } finally {
    events.stop();
  }
  expect(events.seen.filter((event) => event.type === "deviceCompared")).toEqual([]);
});
