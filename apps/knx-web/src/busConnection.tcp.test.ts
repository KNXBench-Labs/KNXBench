// @vitest-environment happy-dom
/** Verify TCP endpoints, observed provenance and fail-closed metadata in the bus controller. */
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { BusMonitorStartResponse } from "./api";
import { admitActivitySnapshot, type ActivitySnapshot, type SessionActivity } from "./liveActivity";
import type { BusEndpoint } from "./serialTransport";

const apiMock = vi.hoisted(() => ({
  busActivity: vi.fn(),
  startBusMonitor: vi.fn(),
  startTcpBusMonitor: vi.fn(),
  startSerialBusMonitor: vi.fn(),
  stopBusMonitor: vi.fn(),
  stopBoundBusMonitor: vi.fn(),
}));
vi.mock("./api", async (importOriginal) => ({ ...(await importOriginal<typeof import("./api")>()), ...apiMock }));
const order = vi.hoisted(() => [] as string[]);
vi.mock("./busContext", async (importOriginal) => ({
  ...(await importOriginal<typeof import("./busContext")>()),
  recordSessionContext: vi.fn((id: number, incarnation: string) => order.push(`record:${id}:${incarnation}`)),
  forgetSessionContext: vi.fn(() => order.push("forget")),
}));
vi.mock("./achievementEvents", () => ({
  emitAchievementEvent: vi.fn((event: { type: string }) => order.push(`achievement:${event.type}`)),
}));

import {
  connectBusSession,
  connectionFromActivity,
  knownTcpSession,
  readBusConnectionForTests,
  refreshBusConnection,
  resetBusConnectionForTests,
  tcpStartErrorMessage,
} from "./busConnection";
import * as api from "./api";
import { translate } from "./i18n";

const GATEWAY = "192.0.2.10:3671";
const snapshot = (sessions: SessionActivity[] = [], serverIncarnation = "process-a"): ActivitySnapshot => ({
  serverIncarnation, coverage: "partial", historyState: "disabled", sessions, oneShot: [], oneShotDropped: 0,
  busyLocks: [], untracked: ["groupWrite", "serialAddress"],
});
const monitor = (extra: Partial<Extract<SessionActivity, { kind: "busMonitor" }>> = {}, id = 5): SessionActivity =>
  ({ kind: "busMonitor", id, gateway: GATEWAY, state: "active", ...extra });
const tcpStarted = { sessionId: 5, serverIncarnation: "process-a", assignedAddress: "1.1.250", transport: "tcp", skippedFrames: 0 } satisfies BusMonitorStartResponse;
const udpStarted = { sessionId: 5, serverIncarnation: "process-a", assignedAddress: "1.1.250" } satisfies BusMonitorStartResponse;
const serial = { path: "/synthetic/tty", source: "1.1.250", mode: "passive" } as const;
const flush = async () => { for (let i = 0; i < 6; i += 1) await Promise.resolve(); };
const calls = () => ({
  udp: apiMock.startBusMonitor.mock.calls.length,
  tcp: apiMock.startTcpBusMonitor.mock.calls.length,
  serial: apiMock.startSerialBusMonitor.mock.calls.length,
});

beforeEach(() => {
  resetBusConnectionForTests();
  order.length = 0;
  for (const fn of Object.values(apiMock)) fn.mockReset();
  apiMock.busActivity.mockResolvedValue(snapshot());
});
afterEach(() => { resetBusConnectionForTests(); vi.restoreAllMocks(); });

describe("observed transport in the activity view", () => {
  it("carries exactly the stated transport and leaves a legacy session without one", () => {
    const view = (extra: Parameters<typeof monitor>[0]) => connectionFromActivity(snapshot([monitor(extra)]));
    expect(view({ transport: "tcp" })).toEqual({ phase: "connected", sessionId: 5, serverIncarnation: "process-a", gateway: GATEWAY, transport: "tcp" });
    expect(view({ transport: "udp", state: "closed" })).toEqual({ phase: "lost", sessionId: 5, serverIncarnation: "process-a", gateway: GATEWAY, transport: "udp" });
    const legacy = view({});
    expect(legacy).toEqual({ phase: "connected", sessionId: 5, serverIncarnation: "process-a", gateway: GATEWAY });
    expect("transport" in legacy).toBe(false);
  });
  it("admits a stated transport only on an IP session and refuses anything else", () => {
    const wire = (extra: Record<string, unknown>) => ({ ...snapshot(), sessions: [{ kind: "busMonitor", id: 5, state: "active", gateway: GATEWAY, ...extra }] });
    for (const transport of ["udp", "tcp"]) expect(admitActivitySnapshot(wire({ transport })).sessions[0]).toMatchObject({ transport });
    expect("transport" in admitActivitySnapshot(wire({})).sessions[0]).toBe(false);
    for (const transport of ["TCP", "tls", "", null, 1, true, {}]) {
      expect(() => admitActivitySnapshot(wire({ transport })), JSON.stringify(transport)).toThrow();
    }
    // A serial session never states an IP transport.
    const serialWire = { ...snapshot(), sessions: [{ kind: "busMonitor", id: 5, state: "active", serial }] };
    expect(admitActivitySnapshot(serialWire).sessions[0]).toMatchObject({ serial });
    serialWire.sessions[0] = { ...serialWire.sessions[0], transport: "tcp" } as never;
    expect(() => admitActivitySnapshot(serialWire)).toThrow();
    expect(() => admitActivitySnapshot(wire({ transport: "tcp", unexpected: 1 }))).toThrow();
  });
});

describe("explicit endpoint start routes", () => {
  it("sends each endpoint kind to its own route only", async () => {
    apiMock.startBusMonitor.mockResolvedValue(udpStarted);
    apiMock.startTcpBusMonitor.mockResolvedValue(tcpStarted);
    apiMock.startSerialBusMonitor.mockResolvedValue({ sessionId: 5, serverIncarnation: "process-a", assignedAddress: serial.source, serial });
    for (const [endpoint, expected] of [
      [GATEWAY, { udp: 1, tcp: 0, serial: 0 }],
      [{ kind: "tcp", gateway: GATEWAY }, { udp: 0, tcp: 1, serial: 0 }],
      [serial, { udp: 0, tcp: 0, serial: 1 }],
    ] as const) {
      resetBusConnectionForTests(); for (const fn of Object.values(apiMock)) fn.mockClear();
      expect((await connectBusSession(endpoint)).ok).toBe(true);
      expect(calls()).toEqual(expected);
    }
    expect(apiMock.startTcpBusMonitor).not.toHaveBeenCalledWith(serial);
  });
  it("opens nothing for a shape that is neither a string, a TCP endpoint nor a valid serial target", async () => {
    for (const bad of [
      { kind: "tcp" }, { kind: "tcp", gateway: "" }, { kind: "udp", gateway: GATEWAY }, { kind: "tcp", gateway: GATEWAY, extra: 1 },
      { path: "", source: "1.1.250", mode: "passive" }, { ...serial, kind: "tcp" }, {}, [], null,
    ] as unknown as BusEndpoint[]) {
      const result = await connectBusSession(bad);
      expect(result, JSON.stringify(bad)).toMatchObject({ ok: false, reason: "failed" });
      expect(calls()).toEqual({ udp: 0, tcp: 0, serial: 0 });
    }
    expect(order).toEqual([]);
  });
  it("an older server's missing TCP route is one failed attempt: no UDP call, record, achievement or session", async () => {
    apiMock.startTcpBusMonitor.mockRejectedValue(Object.assign(new Error("not found"), { status: 404 }));
    const result = await connectBusSession({ kind: "tcp", gateway: GATEWAY });
    expect(result).toMatchObject({ ok: false, reason: "failed", error: { status: 404 } });
    expect(calls()).toEqual({ udp: 0, tcp: 1, serial: 0 });
    expect(order).toEqual([]);
    expect(readBusConnectionForTests().action).toBeNull();
    const failure = (result as { error: unknown }).error;
    expect(tcpStartErrorMessage(failure, translate)).toBe(translate("busTransport.ipTunnel.unsupported"));
    expect(tcpStartErrorMessage(Object.assign(new Error("x"), { status: 405 }), translate)).toBe(translate("busTransport.ipTunnel.unsupported"));
  });
  it("keeps an unstructured TCP 502 reason and the translated structured refusals", () => {
    const withStatus = (message: string, status: number, body?: unknown) => Object.assign(new Error(message), { status, body });
    // The server's own words survive: connection refused, Route Back mismatch, no TCP connector, non-TCP tunnel.
    for (const reason of [
      "I/O error: Connection refused (os error 111)",
      "Protocol error: TCP tunnelling is unavailable on this connector",
      "Protocol error: the connector opened a non-TCP tunnel for an explicit TCP request",
    ]) {
      expect(tcpStartErrorMessage(withStatus(reason, 502, { error: reason }), translate)).toBe(reason);
    }
    // A normal generic failure keeps its text too, like the UDP panel path.
    expect(tcpStartErrorMessage(new Error("socket refused"), translate)).toBe("socket refused");
    expect(tcpStartErrorMessage(withStatus("internal failure", 500), translate)).toBe("internal failure");
    // A structured gateway refusal is worded by the existing translation, not echoed.
    const refused = withStatus("gateway refused", 502, {
      error: "gateway refused", kind: "gatewayRefused", gateway: GATEWAY, gatewayStatus: 0x24, gatewayError: "E_NO_MORE_CONNECTIONS",
    });
    const worded = tcpStartErrorMessage(refused, translate);
    expect(worded).not.toBe("gateway refused");
    expect(worded).toBe(api.errorMessage(refused));
    // A missing route is the dedicated unsupported text, whatever the body says.
    expect(tcpStartErrorMessage(withStatus("not found", 404, { error: "not found" }), translate)).toBe(translate("busTransport.ipTunnel.unsupported"));
  });
});

describe("known TCP provenance", () => {
  it("is remembered per exact sessionId and serverIncarnation only after an admitted TCP start", async () => {
    apiMock.startTcpBusMonitor.mockResolvedValue(tcpStarted);
    expect(knownTcpSession({ sessionId: 5, serverIncarnation: "process-a" })).toBe(false);
    expect((await connectBusSession({ kind: "tcp", gateway: GATEWAY })).ok).toBe(true);
    expect(knownTcpSession({ sessionId: 5, serverIncarnation: "process-a" })).toBe(true);
    expect(knownTcpSession({ sessionId: 6, serverIncarnation: "process-a" })).toBe(false);
    expect(knownTcpSession({ sessionId: 5, serverIncarnation: "process-b" })).toBe(false);
  });
  it("a stale TCP answer is not admitted, so it is not remembered either", async () => {
    let finish!: (value: BusMonitorStartResponse) => void;
    apiMock.startTcpBusMonitor.mockReturnValue(new Promise<BusMonitorStartResponse>((resolve) => { finish = resolve; }));
    apiMock.busActivity.mockResolvedValue(snapshot([], "process-b"));
    const pending = connectBusSession({ kind: "tcp", gateway: GATEWAY });
    await refreshBusConnection(); // Server B is seen first.
    apiMock.busActivity.mockResolvedValue(snapshot([], "process-c"));
    await refreshBusConnection(); // Then a third lifetime: the epoch moved.
    finish(tcpStarted);
    expect(await pending).toEqual({ ok: false, reason: "stale" });
    expect(knownTcpSession({ sessionId: 5, serverIncarnation: "process-a" })).toBe(false);
    expect(order).toEqual([]);
  });
  it("a later poll of the same identity without TCP metadata fails closed instead of reading as legacy UDP", async () => {
    apiMock.startTcpBusMonitor.mockResolvedValue(tcpStarted);
    await connectBusSession({ kind: "tcp", gateway: GATEWAY });
    for (const lacking of [{}, { transport: "udp" as const }]) {
      apiMock.busActivity.mockResolvedValue(snapshot([monitor(lacking)]));
      await refreshBusConnection(); await flush();
      expect(readBusConnectionForTests().view, JSON.stringify(lacking)).toEqual({ phase: "unknown", reason: "malformed" });
    }
    apiMock.busActivity.mockResolvedValue(snapshot([monitor({ transport: "tcp" })]));
    await refreshBusConnection(); await flush();
    expect(readBusConnectionForTests().view).toMatchObject({ phase: "connected", sessionId: 5, transport: "tcp" });
  });
  it("a replacement session or process without metadata is the plain legacy contract, unaffected by the old TCP one", async () => {
    apiMock.startTcpBusMonitor.mockResolvedValue(tcpStarted);
    await connectBusSession({ kind: "tcp", gateway: GATEWAY });
    apiMock.busActivity.mockResolvedValue(snapshot([monitor({}, 6)]));
    await refreshBusConnection(); await flush();
    expect(readBusConnectionForTests().view).toEqual({ phase: "connected", sessionId: 6, serverIncarnation: "process-a", gateway: GATEWAY });
    apiMock.busActivity.mockResolvedValue(snapshot([monitor({})], "process-z"));
    await refreshBusConnection(); await flush();
    expect(readBusConnectionForTests().view).toMatchObject({ phase: "connected", sessionId: 5, serverIncarnation: "process-z" });
    expect(knownTcpSession({ sessionId: 5, serverIncarnation: "process-z" })).toBe(false);
  });
  it("a first observation of TCP in the activity view is remembered for later reads", async () => {
    apiMock.busActivity.mockResolvedValue(snapshot([monitor({ transport: "tcp" })]));
    await refreshBusConnection(); await flush();
    expect(knownTcpSession({ sessionId: 5, serverIncarnation: "process-a" })).toBe(true);
    apiMock.busActivity.mockResolvedValue(snapshot([monitor({})]));
    await refreshBusConnection(); await flush();
    expect(readBusConnectionForTests().view).toEqual({ phase: "unknown", reason: "malformed" });
  });
});
