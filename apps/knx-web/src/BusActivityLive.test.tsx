/** Real-parent tests for the live activity snapshot: partial, volatile, polled while visible. */
// @vitest-environment happy-dom
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
vi.mock("./BusMonitorPanel", () => ({ default: () => <p>Monitor stub</p> }));
vi.mock("./LineScanPanel", () => ({ default: () => <p>Scan stub</p> }));
vi.mock("./DeviceInspectionPanel", () => ({ default: () => <p>Checks stub</p> }));
vi.mock("./DeviceDownloadPanel", () => ({ default: () => <p>Download stub</p> }));
vi.mock("./AddressProgrammingPanel", () => ({ default: () => <p>Address stub</p> }));
vi.mock("./ServiceControlPanel", () => ({ default: () => <p>Service stub</p> }));
import BusDiagnosticsPanel from "./BusDiagnosticsPanel";
import { LIVE_POLL_MS } from "./BusActivityLive";
import { admitActivitySnapshot } from "./liveActivity";
import { messages as en } from "./messages/en";
(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

let host: HTMLDivElement; let root: Root;
let visibility: DocumentVisibilityState = "visible";

function snapshot(overrides: Record<string, unknown> = {}) {
  return {
    serverIncarnation: "synthetic-server-a", coverage: "partial", historyState: "configured",
    sessions: [
      { kind: "deviceDownload", id: 3, address: "1.1.67", state: "running", completedSteps: 2, totalSteps: 5, writtenOctets: 40, totalOctets: 120 },
      { kind: "lineScan", id: 4, state: "cancelled", probed: 12, total: 255 },
    ],
    oneShot: [{ id: 9, kind: "deviceCompare", address: "1.1.5", state: "finished",
      startedAt: "2026-10-06T08:00:00Z", finishedAt: "2026-10-06T08:00:02Z" }],
    oneShotDropped: 2,
    busyLocks: ["managementOperation"],
    untracked: ["groupWrite", "serialAddress"],
    ...overrides,
  };
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
async function open() {
  host = document.createElement("div"); document.body.appendChild(host); root = createRoot(host);
  await act(async () => root.render(<BusDiagnosticsPanel project={null} onTreeUpdate={vi.fn()} />));
  await click(en["activityLive.tab"]);
}
async function tick(ms = LIVE_POLL_MS) { await act(async () => { vi.advanceTimersByTime(ms); }); await flush(); }
const panel = () => host.querySelector(".activity-live-panel")!;
const urls = (fetcher: ReturnType<typeof vi.fn>) => fetcher.mock.calls.map(([url]) => url);

beforeEach(() => {
  vi.useFakeTimers({ toFake: ["setInterval", "clearInterval", "setTimeout", "clearTimeout"] });
  visibility = "visible";
  vi.spyOn(document, "visibilityState", "get").mockImplementation(() => visibility);
});
afterEach(async () => {
  if (root) await act(async () => root.unmount());
  host?.remove(); vi.unstubAllGlobals(); vi.restoreAllMocks(); vi.useRealTimers();
});

it("shows held sessions, one-shot records, eviction, busy locks and exclusions as a partial, volatile view", async () => {
  const fetcher = vi.fn().mockResolvedValue(response(snapshot())); vi.stubGlobal("fetch", fetcher);
  await open();
  const text = panel().textContent!;
  expect(text).toContain(en["activityLive.partial"]);
  expect(text).toContain(en["activityLive.volatile"]);
  expect(text).toContain(en["activityLive.historyState.configured"]);
  const sessions = [...panel().querySelectorAll(".activity-live-session")].map((node) => node.textContent);
  expect(sessions).toEqual([
    `${en["activityHistory.kind.deviceDownload"]} — 1.1.67${en["activityLive.state.running"]}Step 2 of 5 · 40 of 120 bytes`,
    `${en["activityHistory.kind.lineScan"]}${en["activityLive.state.cancelled"]}12 of 255 addresses probed`,
  ]);
  const oneShot = panel().querySelector(".activity-live-oneshot")!.textContent!;
  expect(oneShot).toContain(`${en["activityHistory.kind.deviceCompare"]} — 1.1.5`);
  expect(oneShot).toContain(en["activityHistory.state.finished"]);
  expect(text).toContain("2 older completed records were evicted from this live view.");
  expect(text).toContain(`${en["activityLive.busy"]}: ${en["activityLive.lock.managementOperation"]}`);
  expect(text).toContain(`${en["activityLive.untracked"]}: ${en["activityHistory.kind.groupWrite"]}, ${en["activityHistory.kind.serialAddress"]}`);
  expect(urls(fetcher)).toEqual(["/api/bus/activity"]);
});

it("does not read an empty session list as an idle bus", async () => {
  vi.stubGlobal("fetch", vi.fn().mockResolvedValue(response(snapshot({ sessions: [], oneShot: [], oneShotDropped: 0, busyLocks: [] }))));
  await open();
  expect(panel().textContent).toContain(en["activityLive.noSessions"]);
  expect(panel().textContent).not.toContain(en["activityLive.busy"]);
});

it("polls while visible, pauses while hidden and stops when the tab closes", async () => {
  const finished = snapshot({ sessions: [{ kind: "deviceDownload", id: 3, address: "1.1.67", state: "finished",
    completedSteps: 5, totalSteps: 5, writtenOctets: 120, totalOctets: 120 }] });
  const fetcher = vi.fn().mockResolvedValueOnce(response(snapshot())).mockResolvedValue(response(finished));
  vi.stubGlobal("fetch", fetcher);
  await open();
  await tick();
  expect(fetcher).toHaveBeenCalledTimes(2);
  expect(panel().querySelector(".activity-live-session")!.textContent).toContain(en["activityLive.state.finished"]);
  visibility = "hidden";
  await tick(); await tick();
  expect(fetcher).toHaveBeenCalledTimes(2);
  visibility = "visible";
  await tick();
  expect(fetcher).toHaveBeenCalledTimes(3);
  await click(en["toolbar.busMonitor"]);
  await tick(); await tick();
  expect(fetcher).toHaveBeenCalledTimes(3);
});

it("says when the server restarted, because earlier live entries are gone", async () => {
  const fetcher = vi.fn().mockResolvedValueOnce(response(snapshot()))
    .mockResolvedValue(response(snapshot({ serverIncarnation: "synthetic-server-b" })));
  vi.stubGlobal("fetch", fetcher);
  await open();
  expect(panel().querySelector(".activity-live-restart")).toBeNull();
  await tick();
  expect(panel().querySelector(".activity-live-restart")!.textContent).toBe(en["activityLive.restarted"]);
});

it.each([["unavailable", true], ["disabled", false]] as const)("shows history storage %s distinctly", async (state, warning) => {
  vi.stubGlobal("fetch", vi.fn().mockResolvedValue(response(snapshot({ historyState: state }))));
  await open();
  const line = panel().querySelector(".activity-live-history-state")!;
  expect(line.textContent).toBe(en[`activityLive.historyState.${state}`]);
  expect(line.getAttribute("role")).toBe(warning ? "alert" : null);
});

it("refuses a whole snapshot with an unknown state instead of showing a plausible part", async () => {
  vi.stubGlobal("fetch", vi.fn().mockResolvedValue(response(snapshot({ sessions: [{ kind: "lineScan", id: 4, state: "paused", probed: 1, total: 2 }] }))));
  await open();
  expect(panel().querySelector("[role=alert]")!.textContent).toBe(en["activityLive.malformed"]);
  expect(panel().querySelectorAll(".activity-live-session, .activity-live-oneshot")).toHaveLength(0);
});

it("clears the last snapshot when the server stops answering, without showing its error text", async () => {
  const fetcher = vi.fn().mockResolvedValueOnce(response(snapshot()))
    .mockResolvedValue(response({ error: "synthetic-private-value" }, 503));
  vi.stubGlobal("fetch", fetcher);
  await open();
  await tick();
  expect(panel().querySelector("[role=alert]")!.textContent).toBe(en["activityLive.unavailable"]);
  expect(panel().querySelectorAll(".activity-live-session")).toHaveLength(0);
  expect(panel().textContent).not.toContain("synthetic-private-value");
});

it.each([
  ["an unknown field", { extra: 1 }],
  ["complete coverage", { coverage: "complete" }],
  ["an unknown busy lock", { busyLocks: ["deviceDownload", "somethingElse"] }],
  ["a negative eviction count", { oneShotDropped: -1 }],
  ["more bytes written than planned", { sessions: [{ kind: "deviceDownload", id: 3, address: "1.1.67", state: "running", completedSteps: 2, totalSteps: 5, writtenOctets: 121, totalOctets: 120 }] }],
  ["more steps done than planned", { sessions: [{ kind: "deviceDownload", id: 3, address: "1.1.67", state: "running", completedSteps: 6, totalSteps: 5, writtenOctets: 0, totalOctets: 120 }] }],
  ["more addresses probed than exist", { sessions: [{ kind: "lineScan", id: 4, state: "running", probed: 256, total: 255 }] }],
  ["a one-shot record without its evidence", { oneShot: [{ id: 2, kind: "serviceControlWrite", address: "1.1.5", state: "running", startedAt: "2026-10-06T08:00:00Z", finishedAt: null }] }],
])("admission refuses %s", (_name, overrides) => {
  expect(() => admitActivitySnapshot(snapshot(overrides))).toThrow();
});

it("admits a session without optional rounds and an address-programming session with them", () => {
  const admitted = admitActivitySnapshot(snapshot({ sessions: [
    { kind: "addressProgramming", id: 5, address: "1.1.9", state: "waiting", rounds: 3 },
    { kind: "busMonitor", id: 6, state: "active" },
  ] }));
  expect(admitted.sessions.map((session) => session.kind)).toEqual(["addressProgramming", "busMonitor"]);
});
