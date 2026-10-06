/** Real-parent tests for history meaning, read-only requests and failed refreshes. */
// @vitest-environment happy-dom
import { act, StrictMode } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, expect, it, vi } from "vitest";
vi.mock("./BusMonitorPanel", () => ({ default: () => <p>Monitor stub</p> }));
vi.mock("./LineScanPanel", () => ({ default: () => <p>Scan stub</p> }));
vi.mock("./DeviceInspectionPanel", () => ({ default: () => <p>Checks stub</p> }));
vi.mock("./DeviceDownloadPanel", () => ({ default: () => <p>Download stub</p> }));
vi.mock("./AddressProgrammingPanel", () => ({ default: () => <p>Address stub</p> }));
vi.mock("./ServiceControlPanel", () => ({ default: () => <p>Service stub</p> }));
import BusDiagnosticsPanel from "./BusDiagnosticsPanel";
import { RUNNING_REFRESH_MS } from "./BusActivityHistory";
import { messages as en } from "./messages/en";
(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;
let host: HTMLDivElement; let root: Root;
function page(sequence = 1, incarnation = "synthetic-server") {
  return { format: 2, coverage: "partial", durability: "persistent", hasMore: false, nextCursor: sequence,
    untracked: ["deviceIdentify", "groupWrite", "serialAddress", "deviceDownload", "addressProgramming", "busMonitor", "lineScan"],
    entries: [{ sequence, serverIncarnation: incarnation, interrupted: true, id: 1,
      kind: "deviceDownload", address: "1.1.67", state: "finished",
      startedAt: "2026-10-01T12:00:00.000Z", finishedAt: "2026-10-01T12:01:00.000Z",
      writeEvidence: { backupRecorded: true, sendPossible: true },
      downloadEvidence: { sessionId: 7, written: "yes", restart: "unconfirmed", cleanup: "unknown" } }] };
}
function response(body: unknown, status = 200) {
  return { ok: status === 200, status, statusText: "", headers: new Headers(), json: async () => body };
}
async function flush() { await act(async () => { for (let i = 0; i < 8; i++) await Promise.resolve(); }); }
async function click(label: string) {
  const button = [...host.querySelectorAll("button")].find((node) => node.textContent === label);
  expect(button, `reachable button: ${label}`).toBeDefined();
  await act(async () => button!.click()); await flush();
}
async function open(strict = false) {
  host = document.createElement("div"); document.body.appendChild(host); root = createRoot(host);
  const view = <BusDiagnosticsPanel project={null} onTreeUpdate={vi.fn()} />;
  await act(async () => root.render(strict ? <StrictMode>{view}</StrictMode> : view));
  await click("Activity history");
}
afterEach(async () => { if (root) await act(async () => root.unmount()); host?.remove(); vi.unstubAllGlobals(); });

it("reaches durable history without a project and keeps witnessed write, unconfirmed restart and unknown cleanup separate", async () => {
  const fetcher = vi.fn().mockResolvedValue(response(page())); vi.stubGlobal("fetch", fetcher);
  await open();
  expect(host.textContent).toContain(en["activityHistory.partial"]);
  expect(host.textContent).toContain(en["activityHistory.written.yes"]);
  expect(host.textContent).toContain(en["activityHistory.restart.unconfirmed"]);
  expect(host.textContent).toContain(en["activityHistory.cleanup.unknown"]);
  expect(host.textContent).toContain(en["activityHistory.interrupted"]);
  expect(host.textContent).toContain(en["activityHistory.validationBoundary"]);
  expect(fetcher.mock.calls.map(([url]) => url)).toEqual(["/api/bus/history?after=0&limit=50"]);
});

it("does not turn an interrupted unknown into not-sent or fabricate a finish timestamp", async () => {
  const unknown = page(); Object.assign(unknown.entries[0], { state: "unknown", finishedAt: null,
    downloadEvidence: { sessionId: 7, written: null, restart: null, cleanup: "unknown" } });
  vi.stubGlobal("fetch", vi.fn().mockResolvedValue(response(unknown))); await open();
  expect(host.textContent).toContain(en["activityHistory.written.unknown"]);
  expect(host.querySelectorAll("time")).toHaveLength(1);
  expect(host.textContent).not.toContain(en["activityHistory.state.notSent"]);
});

it("pages by storage sequence while allowing the same operation id in distinct server incarnations", async () => {
  const first = page(); first.hasMore = true;
  const fetcher = vi.fn().mockResolvedValueOnce(response(first)).mockResolvedValueOnce(response(page(2, "synthetic-new-server")));
  vi.stubGlobal("fetch", fetcher); await open(); await click(en["activityHistory.more"]);
  expect(host.querySelectorAll(".activity-history-entry")).toHaveLength(2);
  expect(fetcher.mock.calls[1][0]).toBe("/api/bus/history?after=1&limit=50");
});

it("rejects a duplicate operation identity across admitted cursor pages instead of hiding it", async () => {
  const first = page(); first.hasMore = true;
  const fetcher = vi.fn().mockResolvedValueOnce(response(first)).mockResolvedValueOnce(response(page(2)));
  vi.stubGlobal("fetch", fetcher); await open(); await click(en["activityHistory.more"]);
  expect(host.querySelectorAll(".activity-history-entry")).toHaveLength(0);
  expect(host.querySelector(".activity-history-panel [role=alert]")?.textContent).toBe(en["activityHistory.malformed"]);
});

it("refreshes the beginning instead of pretending cursors deliver updates to old running rows", async () => {
  const pending = page(); Object.assign(pending.entries[0], { state: "running", interrupted: false, finishedAt: null,
    downloadEvidence: { sessionId: 7, written: null, restart: null, cleanup: "pending" } });
  const fetcher = vi.fn().mockResolvedValueOnce(response(pending)).mockResolvedValueOnce(response(page()));
  vi.stubGlobal("fetch", fetcher); await open(); expect(host.textContent).toContain(en["activityHistory.state.running"]);
  await click(en["activityHistory.refresh"]);
  expect(host.querySelectorAll(".activity-history-entry")).toHaveLength(1);
  expect(host.textContent).toContain(en["activityHistory.written.yes"]);
  expect(fetcher.mock.calls.map(([url]) => url)).toEqual(["/api/bus/history?after=0&limit=50", "/api/bus/history?after=0&limit=50"]);
});

it("clears stale success on unavailable refresh without displaying private error text or claiming empty history", async () => {
  const fetcher = vi.fn().mockResolvedValueOnce(response(page())).mockResolvedValueOnce(response({ error: "synthetic-private-value" }, 503));
  vi.stubGlobal("fetch", fetcher); await open(); await click(en["activityHistory.refresh"]);
  expect(host.textContent).toContain(en["activityHistory.unavailable"]);
  expect(host.querySelectorAll(".activity-history-entry")).toHaveLength(0);
  expect(host.textContent).not.toContain("synthetic-private-value");
  expect(host.textContent).not.toContain(en["activityHistory.empty"]);
});

it("ignores a stale success after strict-effect remount on the same history instance", async () => {
  let resolve!: (value: ReturnType<typeof response>) => void;
  const fetcher = vi.fn().mockImplementationOnce(() => new Promise((done) => { resolve = done; }))
    .mockResolvedValueOnce(response({ ...page(), entries: [], nextCursor: 0 }));
  vi.stubGlobal("fetch", fetcher); await open(true);
  expect(fetcher).toHaveBeenCalledTimes(2);
  await act(async () => resolve(response(page()))); await flush();
  expect(host.querySelectorAll(".activity-history-entry")).toHaveLength(0);
  expect(host.textContent).toContain(en["activityHistory.empty"]);
});

it("ignores a stale error after strict-effect remount without clearing the current success", async () => {
  let reject!: (error: Error) => void;
  const fetcher = vi.fn().mockImplementationOnce(() => new Promise((_done, fail) => { reject = fail; }))
    .mockResolvedValueOnce(response(page()));
  vi.stubGlobal("fetch", fetcher); await open(true);
  expect(fetcher).toHaveBeenCalledTimes(2);
  await act(async () => reject(new Error("synthetic-private-error"))); await flush();
  expect(host.querySelectorAll(".activity-history-entry")).toHaveLength(1);
  expect(host.querySelector(".activity-history-panel [role=alert]")).toBeNull();
});

it("does not let a stale completion end loading while the current strict-effect request is pending", async () => {
  let first!: (value: ReturnType<typeof response>) => void;
  let second!: (value: ReturnType<typeof response>) => void;
  const fetcher = vi.fn().mockImplementationOnce(() => new Promise((done) => { first = done; }))
    .mockImplementationOnce(() => new Promise((done) => { second = done; }));
  vi.stubGlobal("fetch", fetcher); await open(true);
  expect(fetcher).toHaveBeenCalledTimes(2);
  await act(async () => first(response(page()))); await flush();
  expect(host.querySelector(".activity-history-panel")?.getAttribute("aria-busy")).toBe("true");
  await act(async () => second(response(page()))); await flush();
  expect(host.querySelector(".activity-history-panel")?.getAttribute("aria-busy")).toBe("false");
});

it("ignores a delayed response after the actual parent switches away", async () => {
  let resolve!: (value: ReturnType<typeof response>) => void;
  const fetcher = vi.fn().mockImplementationOnce(() => new Promise((done) => { resolve = done; })).mockResolvedValueOnce(response({ ...page(), entries: [], nextCursor: 0 }));
  vi.stubGlobal("fetch", fetcher); await open(); await click(en["toolbar.busMonitor"]); await click("Activity history");
  await act(async () => resolve(response(page()))); await flush();
  expect(host.textContent).toContain(en["activityHistory.empty"]);
  expect(host.querySelectorAll(".activity-history-entry")).toHaveLength(0);
});

// UI-04: cursors never update an already loaded running row (the contract's
// "refresh the current page"). While the first window holds one, it reloads
// itself; after paging further it says how to see the latest state instead.
function running() {
  const pending = page(); Object.assign(pending.entries[0], { state: "running", interrupted: false, finishedAt: null,
    downloadEvidence: { sessionId: 7, written: null, restart: null, cleanup: "pending" } });
  return pending;
}
async function wait(ms: number) { await act(async () => { vi.advanceTimersByTime(ms); }); await flush(); }

it("reloads the first window on its own while it shows a running operation, then stops", async () => {
  vi.useFakeTimers({ toFake: ["setInterval", "clearInterval", "setTimeout", "clearTimeout"] });
  try {
    const fetcher = vi.fn().mockResolvedValueOnce(response(running())).mockResolvedValue(response(page()));
    vi.stubGlobal("fetch", fetcher); await open();
    await wait(RUNNING_REFRESH_MS);
    expect(fetcher.mock.calls.map(([url]) => url)).toEqual(["/api/bus/history?after=0&limit=50", "/api/bus/history?after=0&limit=50"]);
    expect(host.textContent).toContain(en["activityHistory.written.yes"]);
    await wait(RUNNING_REFRESH_MS * 3);
    expect(fetcher).toHaveBeenCalledTimes(2);
  } finally { vi.useRealTimers(); }
});

it("does not reload by itself when nothing loaded is running", async () => {
  vi.useFakeTimers({ toFake: ["setInterval", "clearInterval", "setTimeout", "clearTimeout"] });
  try {
    const fetcher = vi.fn().mockResolvedValue(response(page())); vi.stubGlobal("fetch", fetcher); await open();
    await wait(RUNNING_REFRESH_MS * 3);
    expect(fetcher).toHaveBeenCalledTimes(1);
    expect(host.querySelector(".activity-history-running-paged")).toBeNull();
  } finally { vi.useRealTimers(); }
});

it("after paging further, asks for a refresh instead of collapsing the loaded window", async () => {
  vi.useFakeTimers({ toFake: ["setInterval", "clearInterval", "setTimeout", "clearTimeout"] });
  try {
    const first = running(); first.hasMore = true;
    const fetcher = vi.fn().mockResolvedValueOnce(response(first)).mockResolvedValueOnce(response(page(2, "synthetic-new-server")));
    vi.stubGlobal("fetch", fetcher); await open(); await click(en["activityHistory.more"]);
    await wait(RUNNING_REFRESH_MS * 3);
    expect(fetcher).toHaveBeenCalledTimes(2);
    expect(host.querySelectorAll(".activity-history-entry")).toHaveLength(2);
    expect(host.querySelector(".activity-history-running-paged")!.textContent).toBe(en["activityHistory.runningPaged"]);
  } finally { vi.useRealTimers(); }
});
