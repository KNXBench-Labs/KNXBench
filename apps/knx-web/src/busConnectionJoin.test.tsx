/** Verify shared footer/monitor ownership and stale-session refusal. */
// @vitest-environment happy-dom
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { BusMonitorStartResponse, BusMonitorStopResponse, BusMonitorTelegramsResponse, BusTelegramRow } from "./api";
import type { ActivitySnapshot, SessionActivity } from "./liveActivity";

const apiMock = vi.hoisted(() => ({
  busActivity: vi.fn(),
  startBusMonitor: vi.fn(),
  startSerialBusMonitor: vi.fn(),
  stopBusMonitor: vi.fn(),
  stopBoundBusMonitor: vi.fn(),
  pollBusTelegrams: vi.fn(),
  discoverBusInterfaces: vi.fn(),
  fetchFlowSnapshot: vi.fn(),
  writeBusValue: vi.fn(),
  writeBoundBusValue: vi.fn(),
  readBusValue: vi.fn(),
}));
vi.mock("./api", async (importOriginal) => ({ ...(await importOriginal<typeof import("./api")>()), ...apiMock }));

import { messages as en } from "./messages/en";
import { messages as de } from "./messages/de";
import BusConnectionStatus from "./BusConnectionStatus";
import BusMonitorPanel from "./BusMonitorPanel";
import { subscribeAchievementEvents } from "./achievementEvents";
import { CONTEXT_EVENT, readSessionContext, recordSessionContext } from "./busContext";
import { refreshBusConnection, readBusConnectionForTests, resetBusConnectionForTests } from "./busConnection";
import { resetBusDiscoveryForTests } from "./busDiscovery";
import { savePreferredGateway } from "./gatewayPreference";
import { resetSettingsForTests, setSetting } from "./settingsStore";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const snapshot = (serverIncarnation: string, sessions: SessionActivity[] = []) => ({
  serverIncarnation, coverage: "partial", historyState: "disabled", sessions, oneShot: [], oneShotDropped: 0,
  busyLocks: [], untracked: [],
}) satisfies ActivitySnapshot;
const monitor = (id: number): SessionActivity => ({ kind: "busMonitor", id, gateway: "192.0.2.1:3671", state: "active" });
const telegrams = (sessionId: number, nextSince = 1, rows: BusTelegramRow[] = []): BusMonitorTelegramsResponse => ({
  sessionId, serverIncarnation: "process-a", contextStatus: "current", projectOpen: true, status: "active",
  nextSince, droppedBefore: 0, telegrams: rows,
});
const row = (seq: number): BusTelegramRow => ({
  seq, timestamp: "2026-11-02T12:00:00Z", source: "1.1.9", destination: "1/2/3", destinationName: null,
  service: "GroupValueWrite", rawPayload: "0x01 (6-bit)", decoded: { kind: "value", dpt: "DPST-1-1", text: "On" },
});
const notFound = () => Object.assign(new Error("no active bus session"), { status: 404 });
function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (reason: unknown) => void;
  const promise = new Promise<T>((done, fail) => { resolve = done; reject = fail; });
  return { promise, resolve, reject };
}

let host: HTMLDivElement;
let root: Root;
let achievements: string[];
let unsubscribe: () => void;
/** Plain since-0 reads (reattach or a new session's first poll); context-only checks are not reattaches. */
const reattachPolls = () => apiMock.pollBusTelegrams.mock.calls.filter(([since, contextOnly]) => since === 0 && !contextOnly).length;
const panelButton = (label: string) =>
  [...host.querySelectorAll<HTMLButtonElement>(".bus-monitor-panel button")].find((b) => b.textContent === label);
const panelHost = () => host.querySelector<HTMLInputElement>(".bus-monitor-host")!;
const flush = () => act(async () => { await vi.advanceTimersByTimeAsync(0); });

async function mount() {
  host = document.createElement("div");
  document.body.append(host);
  root = createRoot(host);
  await act(async () => root.render(<>
    <BusMonitorPanel projectOpen />
    <footer className="workbench-status"><BusConnectionStatus onOpenBusSettings={() => {}} /></footer>
  </>));
  await flush();
}
async function typeGatewayAndConnect() {
  const input = panelHost();
  const setter = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")!.set!;
  await act(async () => {
    setter.call(input, "192.0.2.1");
    input.dispatchEvent(new Event("input", { bubbles: true }));
  });
  await act(async () => panelButton("Connect")!.click());
}

beforeEach(() => {
  vi.useFakeTimers();
  resetBusConnectionForTests();
  resetBusDiscoveryForTests();
  resetSettingsForTests();
  localStorage.clear();
  for (const fn of Object.values(apiMock)) fn.mockReset();
  apiMock.discoverBusInterfaces.mockResolvedValue({ interfaces: [] });
  apiMock.pollBusTelegrams.mockRejectedValue(notFound());
  apiMock.busActivity.mockResolvedValue(snapshot("process-a"));
  savePreferredGateway("192.0.2.1:3671");
  achievements = [];
  unsubscribe = subscribeAchievementEvents((event) => achievements.push(event.type));
});
afterEach(async () => {
  unsubscribe();
  await act(async () => root.unmount());
  host.remove();
  resetBusConnectionForTests();
  resetBusDiscoveryForTests();
  vi.useRealTimers();
});

describe("mounted footer + monitor panel", () => {
  it("R1: the footer's first read of server B fences the panel's pending start on A", async () => {
    apiMock.busActivity.mockRejectedValue(Object.assign(new Error("down"), { status: 503 }));
    await mount();
    const start = deferred<BusMonitorStartResponse>();
    apiMock.startBusMonitor.mockReturnValueOnce(start.promise);
    await typeGatewayAndConnect();
    expect(host.querySelector<HTMLButtonElement>(".bus-connection-action")!.disabled).toBe(true);
    apiMock.busActivity.mockResolvedValue(snapshot("process-b"));
    await act(async () => { await vi.advanceTimersByTimeAsync(2000); });
    const polls = reattachPolls();
    await act(async () => { start.resolve({ sessionId: 5, serverIncarnation: "process-a", assignedAddress: "1.1.5" }); });
    await flush();
    expect({ achievements, record: readSessionContext(localStorage), reattach: reattachPolls() - polls })
      .toEqual({ achievements: [], record: null, reattach: 0 });
    expect(panelHost().disabled).toBe(false);
    expect(panelButton("Connect")).toBeDefined();
    expect(apiMock.startBusMonitor).toHaveBeenCalledTimes(1);
    expect(apiMock.stopBoundBusMonitor).not.toHaveBeenCalled();
    expect(apiMock.stopBusMonitor).not.toHaveBeenCalled();
  });

  it("R2: a delayed stop of A keeps the panel's adopted B and B's exact record", async () => {
    await mount();
    const server = { sessionId: 1 };
    apiMock.startBusMonitor.mockResolvedValueOnce({ sessionId: 1, serverIncarnation: "process-a", assignedAddress: "1.1.5" });
    apiMock.pollBusTelegrams.mockImplementation(async () => telegrams(server.sessionId));
    await typeGatewayAndConnect();
    await flush();
    expect(panelButton("Disconnect")).toBeDefined();
    const stop = deferred<BusMonitorStopResponse>();
    apiMock.stopBoundBusMonitor.mockReturnValueOnce(stop.promise);
    await act(async () => panelButton("Disconnect")!.click());
    // Another window starts B after the server released A, before A's answer.
    server.sessionId = 2;
    recordSessionContext(2, "process-a");
    const recordB = readSessionContext(localStorage);
    apiMock.busActivity.mockResolvedValue(snapshot("process-a", [monitor(2)]));
    await act(async () => { await vi.advanceTimersByTimeAsync(2000); });
    expect(readBusConnectionForTests().view).toMatchObject({ phase: "connected", sessionId: 2 });
    await act(async () => { stop.resolve({ sessionId: 1, serverIncarnation: "process-a", telegramCount: 0, droppedCount: 0 }); });
    await flush();
    expect(readSessionContext(localStorage)).toEqual(recordB);
    expect(panelButton("Disconnect")).toBeDefined();
    expect(panelHost().disabled).toBe(true);
    expect(host.textContent).not.toContain("Stopped session");
    expect(apiMock.stopBoundBusMonitor).toHaveBeenCalledWith({ sessionId: 1, serverIncarnation: "process-a" });
    expect(apiMock.stopBusMonitor).not.toHaveBeenCalled();
  });

  it("orders ref before record and detach before forget: exactly one reattach poll per transition", async () => {
    await mount();
    expect(reattachPolls()).toBe(1);
    apiMock.startBusMonitor.mockResolvedValueOnce({ sessionId: 1, serverIncarnation: "process-a", assignedAddress: "1.1.5" });
    apiMock.pollBusTelegrams.mockResolvedValue(telegrams(1));
    const atRecord: number[] = [];
    const onContext = () => atRecord.push(reattachPolls());
    window.addEventListener(CONTEXT_EVENT, onContext);
    await typeGatewayAndConnect();
    await flush();
    // Its own record triggers no reattach: the only new since-0 poll is the session's first poll.
    expect(atRecord).toEqual([1]);
    expect(reattachPolls()).toBe(2);
    expect(achievements).toEqual(["busMonitorStarted"]);
    apiMock.stopBoundBusMonitor.mockResolvedValueOnce({ sessionId: 1, serverIncarnation: "process-a", telegramCount: 0, droppedCount: 0 });
    apiMock.pollBusTelegrams.mockRejectedValue(notFound());
    await act(async () => panelButton("Disconnect")!.click());
    await flush();
    window.removeEventListener(CONTEXT_EVENT, onContext);
    // Forgetting after the detach makes the now sessionless panel ask exactly once, synchronously
    // inside the same event (its listener runs before this one): 2 before, 3 at the event.
    expect(atRecord).toEqual([1, 3]);
    expect(reattachPolls()).toBe(3);
    expect(readSessionContext(localStorage)).toBeNull();
    expect(panelButton("Connect")).toBeDefined();
  });
});

describe("re-review R6-R8 in the mounted join", () => {
  const stopA = { sessionId: 1, serverIncarnation: "process-a", telegramCount: 0, droppedCount: 0 } satisfies BusMonitorStopResponse;
  const capturedRows = () => host.querySelectorAll(".bus-monitor-panel tbody tr").length;
  const panelError = () => host.querySelector(".bus-monitor-panel span.field-error")?.textContent ?? "";

  async function attachedToA() {
    await mount();
    apiMock.startBusMonitor.mockResolvedValueOnce({ sessionId: 1, serverIncarnation: "process-a", assignedAddress: "1.1.5" });
    apiMock.pollBusTelegrams.mockResolvedValue(telegrams(1));
    await typeGatewayAndConnect();
    await flush();
    expect(panelButton("Disconnect")).toBeDefined();
    achievements.length = 0;
  }

  it("R6: an old 404 from the stop window cannot erase B's record published meanwhile", async () => {
    await attachedToA();
    const old404 = deferred<BusMonitorTelegramsResponse>();
    apiMock.pollBusTelegrams.mockReset();
    apiMock.pollBusTelegrams.mockReturnValueOnce(old404.promise).mockImplementation(() => new Promise(() => {}));
    await act(async () => { await vi.advanceTimersByTimeAsync(1000); });
    const stop = deferred<BusMonitorStopResponse>();
    apiMock.stopBoundBusMonitor.mockReturnValueOnce(stop.promise);
    await act(async () => panelButton("Disconnect")!.click());
    recordSessionContext(2, "process-a");
    const recordB = readSessionContext(localStorage);
    expect(recordB).toMatchObject({ sessionId: 2, serverIncarnation: "process-a" });
    await act(async () => { old404.reject(notFound()); });
    await flush();
    expect(readSessionContext(localStorage)).toEqual(recordB);
    // No false detach either: the stale absence is not acted on.
    expect(host.textContent).not.toContain("ended elsewhere");
    await act(async () => { stop.resolve(stopA); });
    await flush();
    expect(readSessionContext(localStorage)).toEqual(recordB);
    expect(apiMock.stopBusMonitor).not.toHaveBeenCalled();
    expect(apiMock.startBusMonitor).toHaveBeenCalledTimes(1);
  });

  it("R6 control: a fresh 404 for the owned A with nothing newer still ends it and forgets A", async () => {
    await attachedToA();
    apiMock.pollBusTelegrams.mockRejectedValue(notFound());
    await act(async () => { await vi.advanceTimersByTimeAsync(1000); });
    await flush();
    expect(readSessionContext(localStorage)).toBeNull();
    expect(panelButton("Connect")).toBeDefined();
    expect(host.textContent).toContain("ended");
  });

  it("R6: a fresh 404 for A ends A but never clears a record naming another session", async () => {
    await attachedToA();
    recordSessionContext(2, "process-a");
    const recordB = readSessionContext(localStorage);
    apiMock.pollBusTelegrams.mockRejectedValue(notFound());
    await act(async () => { await vi.advanceTimersByTimeAsync(1000); });
    await flush();
    expect(panelButton("Connect")).toBeDefined();
    expect(readSessionContext(localStorage)).toEqual(recordB);
  });

  it("R6: a successful poll answer that crossed a context change does not adopt its outdated session", async () => {
    await attachedToA();
    const old = deferred<BusMonitorTelegramsResponse>();
    apiMock.pollBusTelegrams.mockReset();
    apiMock.pollBusTelegrams.mockReturnValueOnce(old.promise).mockImplementation(() => new Promise(() => {}));
    await act(async () => { await vi.advanceTimersByTimeAsync(1000); });
    recordSessionContext(3, "process-a");
    const recordC = readSessionContext(localStorage);
    await act(async () => { old.resolve(telegrams(2, 1, [row(1)])); });
    await flush();
    expect(host.querySelector(".bus-monitor-replaced-notice")).toBeNull();
    expect(capturedRows()).toBe(0);
    expect(readSessionContext(localStorage)).toEqual(recordC);
  });

  it.each([0, 2])("R9: settling its own reattached A preserves its %i captured rows", async (rowCount) => {
    await mount();
    const start = deferred<BusMonitorStartResponse>();
    apiMock.startBusMonitor.mockReturnValueOnce(start.promise);
    await typeGatewayAndConnect();
    // Reattach can observe our own server-published A before its start response arrives.
    // Every later poll stays pending: it cannot repopulate rows after an accidental reset.
    apiMock.pollBusTelegrams.mockReset();
    apiMock.pollBusTelegrams.mockResolvedValueOnce(telegrams(1, 3, rowCount ? [row(1), row(2)] : []))
      .mockImplementation(() => new Promise(() => {}));
    await act(async () => { window.dispatchEvent(new Event("focus")); });
    await flush();
    const beforeRows = [...host.querySelectorAll(".bus-monitor-panel tbody tr")].map((entry) => entry.outerHTML);
    expect(beforeRows).toHaveLength(rowCount);
    expect(readSessionContext(localStorage)).toBeNull();
    await act(async () => { start.resolve({ sessionId: 1, serverIncarnation: "process-a", assignedAddress: "1.1.5" }); });
    await flush();
    expect(panelError()).toBe("");
    expect(achievements).toEqual(["busMonitorStarted"]);
    expect(readSessionContext(localStorage)).toMatchObject({ sessionId: 1, serverIncarnation: "process-a" });
    expect([...host.querySelectorAll(".bus-monitor-panel tbody tr")].map((entry) => entry.outerHTML)).toEqual(beforeRows);
    expect(host.querySelector(".bus-monitor-panel")!.textContent).toContain("1.1.5");
    expect(apiMock.startBusMonitor).toHaveBeenCalledTimes(1);
    expect(apiMock.stopBoundBusMonitor).not.toHaveBeenCalled();
    expect(apiMock.stopBusMonitor).not.toHaveBeenCalled();
  });

  it("R7: the panel adopting B by itself (no record, no activity) fences the delayed start A", async () => {
    await mount();
    const start = deferred<BusMonitorStartResponse>();
    apiMock.startBusMonitor.mockReturnValueOnce(start.promise);
    await typeGatewayAndConnect();
    apiMock.pollBusTelegrams.mockImplementation(async (since: number) =>
      telegrams(2, 3, since === 0 ? [row(1), row(2)] : []));
    await act(async () => { window.dispatchEvent(new Event("focus")); });
    await flush();
    expect(capturedRows()).toBe(2);
    expect(readSessionContext(localStorage)).toBeNull();
    await act(async () => { start.resolve({ sessionId: 1, serverIncarnation: "process-a", assignedAddress: "1.1.5" }); });
    await flush();
    expect({ achievements, record: readSessionContext(localStorage), rows: capturedRows() })
      .toEqual({ achievements: [], record: null, rows: 2 });
    expect(apiMock.stopBoundBusMonitor).not.toHaveBeenCalled();
  });

  it("R7: a delayed start A keeps the panel's adopted B, its rows and its exact record", async () => {
    await mount();
    const start = deferred<BusMonitorStartResponse>();
    apiMock.startBusMonitor.mockReturnValueOnce(start.promise);
    await typeGatewayAndConnect();
    // Another window stopped A and started B in the same process, and recorded B.
    apiMock.pollBusTelegrams.mockImplementation(async (since: number) =>
      telegrams(2, 3, since === 0 ? [row(1), row(2)] : []));
    apiMock.busActivity.mockResolvedValue(snapshot("process-a", [monitor(2)]));
    recordSessionContext(2, "process-a");
    await flush();
    await act(async () => { await vi.advanceTimersByTimeAsync(2000); });
    const recordB = readSessionContext(localStorage);
    expect(capturedRows()).toBe(2);
    expect(panelButton("Disconnect")).toBeDefined();
    await act(async () => { start.resolve({ sessionId: 1, serverIncarnation: "process-a", assignedAddress: "1.1.5" }); });
    await flush();
    expect({ achievements, record: readSessionContext(localStorage), rows: capturedRows() })
      .toEqual({ achievements: [], record: recordB, rows: 2 });
    expect(panelButton("Disconnect")).toBeDefined();
    expect(apiMock.startBusMonitor).toHaveBeenCalledTimes(1);
    expect(apiMock.stopBoundBusMonitor).not.toHaveBeenCalled();
    expect(apiMock.stopBusMonitor).not.toHaveBeenCalled();
  });

  it("R8: an ordinary stop whose panel already saw the end claims no newer connection", async () => {
    await attachedToA();
    const stop = deferred<BusMonitorStopResponse>();
    apiMock.stopBoundBusMonitor.mockReturnValueOnce(stop.promise);
    await act(async () => panelButton("Disconnect")!.click());
    apiMock.pollBusTelegrams.mockRejectedValue(notFound());
    await act(async () => { await vi.advanceTimersByTimeAsync(1000); });
    await flush();
    expect(readSessionContext(localStorage)).toBeNull();
    await act(async () => { stop.resolve(stopA); });
    await flush();
    expect(panelError()).not.toMatch(/newer/i);
    expect(readSessionContext(localStorage)).toBeNull();
    expect(apiMock.stopBusMonitor).not.toHaveBeenCalled();
  });
});


describe("serial monitor join",()=>{
  it.each(["passive","active"] as const)("connects exact %s target without IP and preserves the observed target on invalid later polls",async mode=>{
    const serial={path:"/synthetic/tty",source:"1.1.250",mode};
    setSetting("preferredBusTransport","serial");setSetting("serialPath",serial.path);
    setSetting("serialSource",serial.source);setSetting("serialMode",mode);
    await mount();
    expect(apiMock.startSerialBusMonitor).not.toHaveBeenCalled();
    apiMock.startSerialBusMonitor.mockResolvedValueOnce({sessionId:5,serverIncarnation:"process-a",assignedAddress:serial.source,serial});
    apiMock.pollBusTelegrams.mockResolvedValue({...telegrams(5),serial});
    apiMock.busActivity.mockResolvedValue(snapshot("process-a",[{kind:"busMonitor",id:5,state:"active",serial}]));
    await act(async()=>panelButton("Connect")!.click());await flush();
    expect(apiMock.startSerialBusMonitor).toHaveBeenCalledExactlyOnceWith(serial);
    expect(apiMock.startBusMonitor).not.toHaveBeenCalled();
    expect(host.querySelector(".bus-monitor-session")!.textContent).toContain(serial.path);
    expect(host.querySelector(".bus-monitor-session")!.textContent).not.toContain("assigned address");
    const input=host.querySelector<HTMLInputElement>(".bus-compose-destination")!;
    expect(input.disabled).toBe(mode==="passive");
    apiMock.pollBusTelegrams.mockResolvedValue(telegrams(5)); // Missing serial metadata cannot enable sends.
    await act(async()=>{await vi.advanceTimersByTimeAsync(1000);});await flush();
    expect(host.querySelector<HTMLInputElement>(".bus-compose-destination")!.disabled).toBe(true);
    expect(host.querySelector(".bus-monitor-session")!.textContent).toContain(serial.path);
    apiMock.stopBoundBusMonitor.mockResolvedValueOnce({sessionId:5,serverIncarnation:"process-a",telegramCount:0,droppedCount:0});
    apiMock.busActivity.mockResolvedValue(snapshot("process-a")); apiMock.pollBusTelegrams.mockRejectedValue(notFound());
    await act(async()=>panelButton("Disconnect")!.click());await flush();
    expect(apiMock.stopBoundBusMonitor).toHaveBeenCalledExactlyOnceWith({sessionId:5,serverIncarnation:"process-a"});
    expect(apiMock.stopBusMonitor).not.toHaveBeenCalled();
  });
});

it("adopts a replacement IP session without carrying the former serial target or passive mode",async()=>{
  const serial={path:"/synthetic/tty",source:"1.1.250",mode:"passive"} satisfies import("./serialTransport").SerialTarget;
  setSetting("preferredBusTransport","serial");setSetting("serialPath",serial.path);setSetting("serialSource",serial.source);
  await mount();
  apiMock.startSerialBusMonitor.mockResolvedValueOnce({sessionId:5,serverIncarnation:"process-a",assignedAddress:serial.source,serial});
  apiMock.pollBusTelegrams.mockResolvedValue({...telegrams(5),serial});
  await act(async()=>panelButton("Connect")!.click());await flush();
  expect(host.querySelector(".bus-monitor-session")!.textContent).toContain(serial.path);
  apiMock.pollBusTelegrams.mockResolvedValue(telegrams(7));
  apiMock.busActivity.mockResolvedValue(snapshot("process-a",[monitor(7)]));
  await act(async()=>{await vi.advanceTimersByTimeAsync(1000);});await flush();
  expect(host.querySelector(".bus-monitor-session")!.textContent).not.toContain(serial.path);
  expect(host.querySelector<HTMLInputElement>(".bus-compose-destination")!.disabled).toBe(false);
  expect(apiMock.startSerialBusMonitor).toHaveBeenCalledTimes(1);expect(apiMock.startBusMonitor).not.toHaveBeenCalled();
});


describe("F4 fresh monitor serial provenance",()=>{
  it.each(["passive","active"] as const)("keeps an admitted %s serial identity locked when its first poll omits metadata",async mode=>{
    const serial={path:"/synthetic/known",source:"1.1.250",mode};
    apiMock.busActivity.mockResolvedValue(snapshot("process-a",[{kind:"busMonitor",id:5,state:"active",serial}]));
    await refreshBusConnection(); // Real shared controller observation, before the panel exists.
    apiMock.pollBusTelegrams.mockResolvedValue(telegrams(5,1,[row(1)]));
    await mount();
    expect(host.querySelector(".bus-monitor-session")!.textContent).toContain(serial.path);
    expect(host.querySelector(".bus-monitor-unverified-lock")).not.toBeNull();
    await act(async()=>host.querySelector<HTMLTableRowElement>(".bus-monitor-table tbody tr")!.click());
    for(const button of host.querySelectorAll<HTMLButtonElement>(".bus-compose-fields button")){expect(button.disabled).toBe(true);await act(async()=>button.click());}
    expect(apiMock.readBusValue).not.toHaveBeenCalled();expect(apiMock.writeBoundBusValue).not.toHaveBeenCalled();expect(apiMock.writeBusValue).not.toHaveBeenCalled();
    expect(apiMock.startBusMonitor).not.toHaveBeenCalled();expect(apiMock.startSerialBusMonitor).not.toHaveBeenCalled();
  });
  it("retains known provenance through an unavailable activity read instead of using preferences as evidence",async()=>{
    const serial={path:"/synthetic/known",source:"1.1.250",mode:"passive"} satisfies import("./serialTransport").SerialTarget;
    apiMock.busActivity.mockResolvedValueOnce(snapshot("process-a",[{kind:"busMonitor",id:5,state:"active",serial}]));await refreshBusConnection();
    apiMock.busActivity.mockRejectedValue(new Error("unavailable"));await refreshBusConnection();
    setSetting("preferredBusTransport","ip");apiMock.pollBusTelegrams.mockResolvedValue(telegrams(5));
    await mount();expect(host.querySelector(".bus-monitor-session")!.textContent).toContain(serial.path);
    expect(host.querySelector<HTMLInputElement>(".bus-compose-destination")!.disabled).toBe(true);
  });
  it("keeps valid legacy IP first reattach enabled despite serial preferences",async()=>{
    setSetting("preferredBusTransport","serial");setSetting("serialPath","/synthetic/preference-only");
    apiMock.busActivity.mockResolvedValue(snapshot("process-a",[monitor(5)]));await refreshBusConnection();
    apiMock.pollBusTelegrams.mockResolvedValue(telegrams(5));await mount();
    expect(host.querySelector<HTMLInputElement>(".bus-compose-destination")!.disabled).toBe(false);
    expect(host.querySelector(".bus-monitor-session")!.textContent).not.toContain("/synthetic/preference-only");
  });
  it("does not adopt a late old first poll after the controller observed its replacement",async()=>{
    const serial={path:"/synthetic/old",source:"1.1.250",mode:"passive"} satisfies import("./serialTransport").SerialTarget;
    apiMock.busActivity.mockResolvedValue(snapshot("process-a",[{kind:"busMonitor",id:5,state:"active",serial}]));await refreshBusConnection();
    const pending=deferred<BusMonitorTelegramsResponse>();apiMock.pollBusTelegrams.mockReturnValueOnce(pending.promise);await mount();
    apiMock.busActivity.mockResolvedValue(snapshot("process-a",[monitor(7)]));await act(async()=>refreshBusConnection());
    await act(async()=>pending.resolve(telegrams(5)));await flush();
    expect(host.querySelector(".bus-compose-form")).toBeNull();
    apiMock.pollBusTelegrams.mockResolvedValue(telegrams(7));
    await act(async()=>window.dispatchEvent(new Event("focus")));await flush();
    expect(host.querySelector(".bus-monitor-session")!.textContent).not.toContain(serial.path);
    expect(host.querySelector<HTMLInputElement>(".bus-compose-destination")!.disabled).toBe(false);
  });
  it("rejects malformed first-poll serial metadata through the actual API without enabling actions",async()=>{
    const serial={path:"/synthetic/known",source:"1.1.250",mode:"passive"} satisfies import("./serialTransport").SerialTarget;
    apiMock.busActivity.mockResolvedValue(snapshot("process-a",[{kind:"busMonitor",id:5,state:"active",serial}]));await refreshBusConnection();
    const actual=await vi.importActual<typeof import("./api")>("./api");apiMock.pollBusTelegrams.mockImplementation(actual.pollBusTelegrams);
    vi.stubGlobal("fetch",vi.fn().mockResolvedValue(new Response(JSON.stringify({...telegrams(5),serial:{...serial,mode:"unknown"}}),{status:200,headers:{"Content-Type":"application/json"}})));
    try{await mount();expect(host.querySelector(".bus-compose-form")).toBeNull();expect(host.querySelector(".bus-connection-state")!.textContent).toContain(serial.path);expect(apiMock.writeBusValue).not.toHaveBeenCalled();}finally{vi.unstubAllGlobals();}
  });
});

it.each(["en","de"] as const)("monitor preserves conservative active-start uncertainty through the actual API in %s",async language=>{
  const messages=language==="en"?en:de;
  setSetting("uiLanguage",language);setSetting("preferredBusTransport","serial");setSetting("serialPath","/synthetic/tty");setSetting("serialSource","1.1.250");setSetting("serialMode","active");
  const actual=await vi.importActual<typeof import("./api")>("./api");apiMock.startSerialBusMonitor.mockImplementation(actual.startSerialBusMonitor);
  vi.stubGlobal("fetch",vi.fn().mockResolvedValue(new Response(JSON.stringify({kind:"serialConnectFailed",activeAutoAckStateUncertain:true,error:"initialization failed; reset failed; auto-ACK state unknown"}),{status:502,headers:{"Content-Type":"application/json"}})));
  try{
    await mount();await act(async()=>panelButton(messages["busMonitor.connect"])!.click());await flush();
    expect(host.querySelector(".bus-monitor-panel")!.textContent).toContain(messages["busTransport.activeStartUncertain"]);
    expect(apiMock.startBusMonitor).not.toHaveBeenCalled();expect(fetch).toHaveBeenCalledTimes(1);
  }finally{vi.unstubAllGlobals();}
});

it("fences a pending first serial poll when the observed server restarts with no session",async()=>{
  const serial={path:"/synthetic/old-process",source:"1.1.250",mode:"passive"} satisfies import("./serialTransport").SerialTarget;
  apiMock.busActivity.mockResolvedValue(snapshot("process-a",[{kind:"busMonitor",id:5,state:"active",serial}]));await refreshBusConnection();
  const pending=deferred<BusMonitorTelegramsResponse>();apiMock.pollBusTelegrams.mockReturnValueOnce(pending.promise);await mount();
  apiMock.busActivity.mockResolvedValue(snapshot("process-b"));await act(async()=>refreshBusConnection());
  await act(async()=>pending.resolve(telegrams(5)));await flush();
  expect(host.querySelector(".bus-compose-form")).toBeNull();
  expect(host.querySelector(".bus-monitor-session")).toBeNull();
  expect(host.querySelector(".bus-monitor-panel")!.textContent).toContain(en["busMonitor.contextUnverified"]);
  expect(apiMock.readBusValue).not.toHaveBeenCalled();expect(apiMock.writeBusValue).not.toHaveBeenCalled();expect(apiMock.writeBoundBusValue).not.toHaveBeenCalled();
});
