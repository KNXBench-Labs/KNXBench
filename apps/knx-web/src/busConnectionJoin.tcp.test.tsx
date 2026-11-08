/** Verify explicit TCP monitoring in the mounted footer and monitor, offline. */
// @vitest-environment happy-dom
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { BusMonitorStartResponse, BusMonitorTelegramsResponse } from "./api";
import type { ActivitySnapshot, SessionActivity } from "./liveActivity";

const apiMock = vi.hoisted(() => ({
  busActivity: vi.fn(),
  startBusMonitor: vi.fn(),
  startTcpBusMonitor: vi.fn(),
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
import { knownTcpSession, readBusConnectionForTests, resetBusConnectionForTests } from "./busConnection";
import { resetBusDiscoveryForTests } from "./busDiscovery";
import { savePreferredGateway } from "./gatewayPreference";
import { getSetting, resetSettingsForTests, setSetting } from "./settingsStore";
import { resetUiLanguageForTests } from "./uiLanguage";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const GATEWAY = "192.0.2.1:3671";
const snapshot = (sessions: SessionActivity[] = [], serverIncarnation = "process-a") => ({
  serverIncarnation, coverage: "partial", historyState: "disabled", sessions, oneShot: [], oneShotDropped: 0,
  busyLocks: [], untracked: [],
}) satisfies ActivitySnapshot;
const monitor = (id: number, transport?: "udp" | "tcp"): SessionActivity =>
  ({ kind: "busMonitor", id, gateway: GATEWAY, state: "active", ...(transport === undefined ? {} : { transport }) });
/** A TCP poll states its counter (default 0); `null` leaves it out; other sessions never state one. */
const poll = (sessionId: number, transport?: "udp" | "tcp", skipped: number | null = 0): BusMonitorTelegramsResponse => ({
  sessionId, serverIncarnation: "process-a", contextStatus: "current", projectOpen: true, status: "active",
  nextSince: 0, droppedBefore: 0, telegrams: [], ...(transport === undefined ? {} : { transport }),
  ...(transport === "tcp" && skipped !== null ? { skippedFrames: skipped } : {}),
});
const tcpStart = (sessionId: number): BusMonitorStartResponse =>
  ({ sessionId, serverIncarnation: "process-a", assignedAddress: "1.1.250", transport: "tcp", skippedFrames: 0 });
const notFound = () => Object.assign(new Error("no active bus session"), { status: 404 });
function deferred<T>() {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((done) => { resolve = done; });
  return { promise, resolve };
}

let host: HTMLDivElement;
let root: Root;
const flush = () => act(async () => { await vi.advanceTimersByTimeAsync(0); });
const tick = (ms: number) => act(async () => { await vi.advanceTimersByTimeAsync(ms); });
const panelButton = (label: string) =>
  [...host.querySelectorAll<HTMLButtonElement>(".bus-monitor-panel button")].find((b) => b.textContent === label);
const footerState = () => host.querySelector(".bus-connection-state")!.textContent ?? "";
const sessionLine = () => host.querySelector(".bus-monitor-session")?.textContent ?? "";
const tunnelSelects = () => [...host.querySelectorAll<HTMLSelectElement>(".bus-monitor-panel .bus-transport-fields label:nth-of-type(2) select")];
const startCalls = () => ({
  udp: apiMock.startBusMonitor.mock.calls.length,
  tcp: apiMock.startTcpBusMonitor.mock.calls.length,
  serial: apiMock.startSerialBusMonitor.mock.calls.length,
});

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
async function connectFromPanel() {
  const input = host.querySelector<HTMLInputElement>(".bus-monitor-host")!;
  await act(async () => {
    Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")!.set!.call(input, "192.0.2.1");
    input.dispatchEvent(new Event("input", { bubbles: true }));
  });
  await act(async () => panelButton("Connect")!.click());
  await flush();
}

beforeEach(() => {
  vi.useFakeTimers();
  resetBusConnectionForTests(); resetBusDiscoveryForTests(); resetSettingsForTests(); resetUiLanguageForTests();
  localStorage.clear();
  for (const fn of Object.values(apiMock)) fn.mockReset();
  apiMock.discoverBusInterfaces.mockResolvedValue({ interfaces: [] });
  apiMock.pollBusTelegrams.mockRejectedValue(notFound());
  apiMock.busActivity.mockResolvedValue(snapshot());
  savePreferredGateway(GATEWAY);
});
afterEach(async () => {
  await act(async () => root.unmount());
  host.remove();
  resetBusConnectionForTests(); resetBusDiscoveryForTests();
  vi.useRealTimers();
});

describe("a saved TCP preference", () => {
  it("never connects, discovers anything beyond the usual search or sends a request by itself", async () => {
    setSetting("preferredIpTunnelTransport", "tcp");
    await mount(); await tick(10_000);
    expect(startCalls()).toEqual({ udp: 0, tcp: 0, serial: 0 });
    expect(apiMock.stopBusMonitor).not.toHaveBeenCalled(); expect(apiMock.stopBoundBusMonitor).not.toHaveBeenCalled();
    expect(apiMock.writeBoundBusValue).not.toHaveBeenCalled(); expect(apiMock.readBusValue).not.toHaveBeenCalled();
    // Selecting it in the panel is only a preference change.
    const select = tunnelSelects()[0];
    await act(async () => { select.value = "udp"; select.dispatchEvent(new Event("change", { bubbles: true })); });
    await act(async () => { select.value = "tcp"; select.dispatchEvent(new Event("change", { bubbles: true })); });
    expect(getSetting("preferredIpTunnelTransport")).toBe("tcp");
    expect(startCalls()).toEqual({ udp: 0, tcp: 0, serial: 0 });
  });

  it("starts through the TCP route only, from the panel, and shows the observed TCP in panel and footer", async () => {
    setSetting("preferredIpTunnelTransport", "tcp");
    await mount();
    apiMock.startTcpBusMonitor.mockResolvedValueOnce(tcpStart(7));
    apiMock.pollBusTelegrams.mockResolvedValue(poll(7, "tcp"));
    apiMock.busActivity.mockResolvedValue(snapshot([monitor(7, "tcp")]));
    await connectFromPanel(); await tick(2000);
    expect(startCalls()).toEqual({ udp: 0, tcp: 1, serial: 0 });
    expect(apiMock.startTcpBusMonitor).toHaveBeenCalledExactlyOnceWith(GATEWAY);
    expect(sessionLine()).toContain(en["busTransport.ipTunnel.observed"].replace("{transport}", "TCP"));
    expect(footerState()).toBe(en["busConnection.state.connected"].replace("{gateway}", `${GATEWAY} · TCP`));
    expect(knownTcpSession({ sessionId: 7, serverIncarnation: "process-a" })).toBe(true);
    // The transport choice is locked with the session; the panel's own select is disabled.
    expect(tunnelSelects()[0].disabled).toBe(true);
  });

  it("an older server's 404 is one TCP attempt with the unsupported text, no UDP call, no session, Connect usable again", async () => {
    setSetting("preferredIpTunnelTransport", "tcp");
    await mount();
    apiMock.startTcpBusMonitor.mockRejectedValueOnce(Object.assign(new Error("not found"), { status: 404 }));
    await connectFromPanel(); await tick(2000);
    expect(startCalls()).toEqual({ udp: 0, tcp: 1, serial: 0 });
    expect(host.querySelector(".bus-monitor-panel .field-error")!.textContent).toBe(en["busTransport.ipTunnel.unsupported"]);
    expect(host.querySelector(".bus-monitor-session")).toBeNull();
    expect(readBusConnectionForTests().view.phase).toBe("disconnected");
    expect(panelButton("Connect")!.disabled).toBe(false);
  });

  it("starts from the footer with the same single TCP route and never the UDP route", async () => {
    setSetting("preferredIpTunnelTransport", "tcp");
    await mount();
    apiMock.startTcpBusMonitor.mockResolvedValueOnce(tcpStart(8));
    apiMock.busActivity.mockResolvedValue(snapshot([monitor(8, "tcp")]));
    await act(async () => host.querySelector<HTMLButtonElement>(".bus-connection-action")!.click());
    await flush(); await tick(2000);
    expect(startCalls()).toEqual({ udp: 0, tcp: 1, serial: 0 });
    expect(footerState()).toContain("· TCP");
  });
});

describe("the observed transport wins over the saved preference", () => {
  it("preference TCP, observed UDP: neither the footer nor the monitor claims TCP", async () => {
    setSetting("preferredIpTunnelTransport", "tcp");
    apiMock.busActivity.mockResolvedValue(snapshot([monitor(3, "udp")]));
    apiMock.pollBusTelegrams.mockResolvedValue(poll(3, "udp"));
    await mount(); await tick(2000);
    expect(footerState()).toBe(en["busConnection.state.connected"].replace("{gateway}", GATEWAY));
    expect(footerState()).not.toContain("TCP");
    expect(sessionLine()).toContain(en["busTransport.ipTunnel.observed"].replace("{transport}", "UDP"));
    expect(sessionLine()).not.toContain("TCP");
    expect(knownTcpSession({ sessionId: 3, serverIncarnation: "process-a" })).toBe(false);
    expect(startCalls()).toEqual({ udp: 0, tcp: 0, serial: 0 });
  });

  it("preference UDP, an adopted TCP session on mount: both show TCP and the panel is tied to that identity", async () => {
    apiMock.busActivity.mockResolvedValue(snapshot([monitor(4, "tcp")]));
    apiMock.pollBusTelegrams.mockResolvedValue(poll(4, "tcp"));
    await mount(); await tick(2000);
    expect(getSetting("preferredIpTunnelTransport")).toBeUndefined();
    expect(footerState()).toContain("· TCP");
    expect(sessionLine()).toContain(en["busTransport.ipTunnel.observed"].replace("{transport}", "TCP"));
    expect(startCalls()).toEqual({ udp: 0, tcp: 0, serial: 0 });
  });

  it("a legacy server without metadata keeps the unmarked contract and no transport line", async () => {
    apiMock.busActivity.mockResolvedValue(snapshot([monitor(5)]));
    apiMock.pollBusTelegrams.mockResolvedValue(poll(5));
    await mount(); await tick(2000);
    expect(footerState()).toBe(en["busConnection.state.connected"].replace("{gateway}", GATEWAY));
    expect(host.querySelector('[data-testid="bus-session-transport"]')).toBeNull();
    expect(host.querySelector<HTMLInputElement>(".bus-compose-destination")!.disabled).toBe(false);
  });

  it("navigation: remounting the monitor re-reads the transport from the server, not from the previous mount", async () => {
    apiMock.busActivity.mockResolvedValue(snapshot([monitor(6, "tcp")]));
    apiMock.pollBusTelegrams.mockResolvedValue(poll(6, "tcp"));
    await mount(); await tick(2000);
    expect(sessionLine()).toContain("TCP");
    await act(async () => root.unmount());
    // The session is replaced by a UDP one while the monitor is away.
    apiMock.busActivity.mockResolvedValue(snapshot([monitor(9, "udp")]));
    apiMock.pollBusTelegrams.mockResolvedValue(poll(9, "udp"));
    host.remove(); resetBusConnectionForTests();
    await mount(); await tick(2000);
    expect(sessionLine()).toContain("UDP"); expect(sessionLine()).not.toContain("TCP");
    expect(footerState()).not.toContain("TCP");
  });
});

describe("known TCP fails closed", () => {
  async function connectedTcp() {
    await mount();
    apiMock.startTcpBusMonitor.mockResolvedValueOnce(tcpStart(7));
    apiMock.pollBusTelegrams.mockResolvedValue(poll(7, "tcp"));
    apiMock.busActivity.mockResolvedValue(snapshot([monitor(7, "tcp")]));
    setSetting("preferredIpTunnelTransport", "tcp");
    await connectFromPanel(); await tick(2000);
    expect(sessionLine()).toContain("TCP");
  }
  const destination = () => host.querySelector<HTMLInputElement>(".bus-compose-destination")!;

  it("a poll without transport (or with UDP) locks group actions and never relabels the session as UDP", async () => {
    await connectedTcp();
    expect(destination().disabled).toBe(false);
    for (const lacking of [undefined, "udp" as const]) {
      apiMock.pollBusTelegrams.mockResolvedValue(poll(7, lacking));
      apiMock.busActivity.mockResolvedValue(snapshot([monitor(7, lacking)]));
      await tick(2000);
      expect(host.textContent).toContain(en["busTransport.ipTunnel.unverified"]);
      expect(host.textContent).toContain(en["busMonitor.contextUnverified"]);
      expect(destination().disabled).toBe(true);
      // Neither relabelled as UDP nor claimed as verified TCP.
      expect(sessionLine()).not.toContain("UDP"); expect(sessionLine()).not.toContain(" · TCP");
      expect(footerState()).toBe(en["busConnection.state.unknown"]); // The footer failed closed too.
    }
    // The server states TCP again: the lock lifts.
    apiMock.pollBusTelegrams.mockResolvedValue(poll(7, "tcp"));
    apiMock.busActivity.mockResolvedValue(snapshot([monitor(7, "tcp")]));
    await tick(2000);
    expect(host.textContent).not.toContain(en["busTransport.ipTunnel.unverified"]);
    expect(destination().disabled).toBe(false);
  });

  it("a replacement session without metadata is a plain legacy session and does not inherit the TCP lock", async () => {
    await connectedTcp();
    apiMock.pollBusTelegrams.mockResolvedValue(poll(8));
    apiMock.busActivity.mockResolvedValue(snapshot([monitor(8)]));
    await tick(2000);
    expect(host.textContent).not.toContain(en["busTransport.ipTunnel.unverified"]);
    expect(sessionLine()).not.toContain("TCP");
    expect(footerState()).toBe(en["busConnection.state.connected"].replace("{gateway}", GATEWAY));
  });
});

describe("pending operations and stale answers", () => {
  it("disables both connect buttons and the tunnel choices while a TCP start is pending", async () => {
    setSetting("preferredIpTunnelTransport", "tcp");
    await mount();
    const start = deferred<BusMonitorStartResponse>();
    apiMock.startTcpBusMonitor.mockReturnValueOnce(start.promise);
    await connectFromPanel();
    expect(panelButton("Connect")?.disabled ?? true).toBe(true);
    expect(host.querySelector<HTMLButtonElement>(".bus-connection-action")!.disabled).toBe(true);
    expect(tunnelSelects()[0].disabled).toBe(true);
    // A second click cannot start a second TCP session.
    await act(async () => host.querySelector<HTMLButtonElement>(".bus-connection-action")!.click());
    expect(startCalls()).toEqual({ udp: 0, tcp: 1, serial: 0 });
    apiMock.busActivity.mockResolvedValue(snapshot([monitor(7, "tcp")]));
    apiMock.pollBusTelegrams.mockResolvedValue(poll(7, "tcp"));
    await act(async () => start.resolve(tcpStart(7))); await flush(); await tick(2000);
    expect(sessionLine()).toContain("TCP");
  });

  it("a TCP start answer that crosses a server restart is not adopted, remembered or stopped", async () => {
    apiMock.busActivity.mockRejectedValue(Object.assign(new Error("down"), { status: 503 }));
    setSetting("preferredIpTunnelTransport", "tcp");
    await mount();
    const start = deferred<BusMonitorStartResponse>();
    apiMock.startTcpBusMonitor.mockReturnValueOnce(start.promise);
    await connectFromPanel();
    apiMock.busActivity.mockResolvedValue(snapshot([], "process-b"));
    await tick(2000);
    await act(async () => start.resolve(tcpStart(7))); await flush();
    expect(knownTcpSession({ sessionId: 7, serverIncarnation: "process-a" })).toBe(false);
    expect(host.querySelector(".bus-monitor-session")).toBeNull();
    expect(panelButton("Connect")).toBeDefined();
    expect(apiMock.stopBoundBusMonitor).not.toHaveBeenCalled(); expect(apiMock.stopBusMonitor).not.toHaveBeenCalled();
    expect(startCalls()).toEqual({ udp: 0, tcp: 1, serial: 0 });
  });
});

describe("skipped TCP frames", () => {
  const notice = () => host.querySelector(".bus-monitor-skipped-frames");
  const unavailable = () => host.querySelector(".bus-monitor-skipped-frames-unavailable");
  const stopNote = () => host.querySelector(".bus-monitor-stop-skipped")?.textContent ?? null;
  const frames = (count: number, catalogue: typeof en | typeof de = en) =>
    catalogue[count === 1 ? "busMonitor.skippedFrames.one" : "busMonitor.skippedFrames.other"].replace("{count}", String(count));
  async function adoptTcp(skipped: number | null) {
    apiMock.busActivity.mockResolvedValue(snapshot([monitor(7, "tcp")]));
    apiMock.pollBusTelegrams.mockResolvedValue(poll(7, "tcp", skipped));
    await mount(); await tick(2000);
  }

  it("shows nothing for a measured zero and the exact cumulative count, apart from dropped rows, once frames are skipped", async () => {
    await adoptTcp(0);
    expect(notice()).toBeNull(); expect(unavailable()).toBeNull();
    for (const count of [2, 3, 1]) {
      apiMock.pollBusTelegrams.mockResolvedValue(poll(7, "tcp", count));
      await tick(2000);
      expect(notice()!.textContent, String(count)).toBe(frames(count));
      expect(notice()!.getAttribute("role")).toBe("alert");
      expect(host.querySelector(".bus-monitor-gap-notice")).toBeNull();
    }
    expect(notice()!.textContent).not.toContain("lost");
    expect(notice()!.textContent).toContain("not as a dropped telegram");
  });

  it("says unavailable, never zero, when a TCP session states no counter", async () => {
    await adoptTcp(null);
    expect(unavailable()!.textContent).toBe(en["busMonitor.skippedFramesUnavailable"]);
    expect(notice()).toBeNull();
  });

  it("a UDP or legacy session never shows a skip notice", async () => {
    for (const transport of ["udp", undefined] as const) {
      apiMock.busActivity.mockResolvedValue(snapshot([monitor(3, transport)]));
      apiMock.pollBusTelegrams.mockResolvedValue(poll(3, transport));
      await mount(); await tick(2000);
      expect(notice()).toBeNull(); expect(unavailable()).toBeNull();
      await act(async () => root.unmount()); host.remove(); resetBusConnectionForTests();
    }
    await mount();
  });

  it("the count follows the session: a replacement without TCP clears it", async () => {
    await adoptTcp(5);
    expect(notice()!.textContent).toBe(frames(5));
    apiMock.busActivity.mockResolvedValue(snapshot([monitor(8, "udp")]));
    apiMock.pollBusTelegrams.mockResolvedValue(poll(8, "udp"));
    await tick(2000);
    expect(notice()).toBeNull();
  });

  it("the stop summary reports the count read before the disconnect started, or says unavailable", async () => {
    for (const [answer, shown] of [
      [4, en["busMonitor.skippedFramesAtStop.other"].replace("{count}", "4")],
      [1, en["busMonitor.skippedFramesAtStop.one"].replace("{count}", "1")],
      [0, null],
      [undefined, en["busMonitor.skippedFramesUnavailable"]],
    ] as const) {
      resetBusConnectionForTests();
      apiMock.busActivity.mockResolvedValue(snapshot([monitor(7, "tcp")]));
      apiMock.pollBusTelegrams.mockResolvedValue(poll(7, "tcp", 0));
      await mount(); await tick(2000);
      apiMock.stopBoundBusMonitor.mockResolvedValueOnce({
        sessionId: 7, serverIncarnation: "process-a", telegramCount: 0, droppedCount: 0,
        ...(answer === undefined ? {} : { skippedFrames: answer }),
      });
      await act(async () => panelButton("Disconnect")!.click()); await flush();
      expect(stopNote(), String(answer)).toBe(shown);
      await act(async () => root.unmount()); host.remove();
    }
    await mount();
  });

  it("a UDP stop never mentions skipped frames", async () => {
    apiMock.busActivity.mockResolvedValue(snapshot([monitor(3, "udp")]));
    apiMock.pollBusTelegrams.mockResolvedValue(poll(3, "udp"));
    await mount(); await tick(2000);
    apiMock.stopBoundBusMonitor.mockResolvedValueOnce({ sessionId: 3, serverIncarnation: "process-a", telegramCount: 0, droppedCount: 0 });
    await act(async () => panelButton("Disconnect")!.click()); await flush();
    expect(stopNote()).toBeNull();
  });

  it("German copy shows the same count in du-neutral wording", async () => {
    setSetting("uiLanguage", "de"); resetUiLanguageForTests();
    await adoptTcp(2);
    expect(notice()!.textContent).toBe(frames(2, de));
    expect(frames(2, de)).not.toBe(frames(2, en));
  });
});

describe("German du-form copy", () => {
  it("states the scope and the fail-closed notes without the formal address", () => {
    for (const key of [
      "busTransport.ipTunnel.note", "busTransport.ipTunnel.unsupported", "busTransport.ipTunnel.unverified", "busTransport.ipTunnel.sendNote",
      "busMonitor.skippedFrames.one", "busMonitor.skippedFrames.other", "busMonitor.skippedFramesUnavailable",
      "busMonitor.skippedFramesAtStop.one", "busMonitor.skippedFramesAtStop.other",
    ] as const) {
      expect(de[key], key).not.toMatch(/\b(Sie|Ihre?[nms]?|Ihnen)\b/);
      expect(de[key].length).toBeGreaterThan(20);
      expect(de[key]).not.toBe(en[key]);
    }
    expect(de["busTransport.ipTunnel.unverified"]).toMatch(/\bdu\b/);
    expect(de["busTransport.ipTunnel.note"]).toMatch(/UDP/);
  });
});
