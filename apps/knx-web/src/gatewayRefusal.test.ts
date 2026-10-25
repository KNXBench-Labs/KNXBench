/** Issue #10: structured gateway refusals are worded per kind, never a bare "busy". */
import { afterEach, describe, expect, it, vi } from "vitest";
import {
  admitGatewayRefusal, describeGatewayRefusal, describeHolder, gatewayRefusalMessage, holderFromActivity,
} from "./gatewayRefusal";
import type { ActivitySnapshot } from "./liveActivity";
import { translateFor } from "./i18n";

const active = vi.hoisted(() => ({ language: "en" }));
vi.mock("./uiLanguage", async (importOriginal) => ({
  ...(await importOriginal<typeof import("./uiLanguage")>()),
  getActiveUiLanguage: () => active.language,
}));

const en = (key: Parameters<typeof translateFor>[1], params?: Parameters<typeof translateFor>[2]) =>
  translateFor("en", key, params);
const de = (key: Parameters<typeof translateFor>[1], params?: Parameters<typeof translateFor>[2]) =>
  translateFor("de", key, params);

/** What `api.ts`'s `request()` throws: an `Error` with `status` and `body`. */
function refusal(status: number, body: unknown): Error {
  return Object.assign(new Error(String((body as { error?: unknown })?.error ?? "")), { status, body });
}
const busy = (holder: unknown) => refusal(409, { error: "the gateway serves one tunnel", kind: "gatewayBusy", holder });

afterEach(() => { active.language = "en"; });

describe("admitGatewayRefusal", () => {
  it("admits each holder kind the server names", () => {
    for (const holder of [
      { kind: "deviceDownload", address: "1.1.5", state: "running" },
      { kind: "addressProgramming", address: "1.1.9", state: "waiting" },
      { kind: "addressProgramming", address: "1.1.9", state: "programming" },
      { kind: "busMonitor", sessionId: 3, gateway: "192.0.2.10:3671", state: "active" },
      { kind: "busMonitor", sessionId: 3, gateway: "192.0.2.10:3671", state: "closed" },
      { kind: "lineScan", sessionId: 4, probed: 37, total: 120 },
      { kind: "connecting", session: "deviceDownload" },
      { kind: "connecting", session: "busMonitorOrGroupWrite" },
    ]) {
      expect(admitGatewayRefusal(busy(holder)), JSON.stringify(holder)).toEqual({ kind: "gatewayBusy", holder });
    }
  });

  it("refuses anything malformed, so the server's own text is shown instead of a guess", () => {
    for (const error of [
      busy({ kind: "lineScan", sessionId: 4, probed: 121, total: 120 }),
      busy({ kind: "busMonitor", sessionId: 3, state: "active" }),
      busy({ kind: "deviceDownload", address: "not an address", state: "running" }),
      busy({ kind: "connecting", session: "somethingNew" }),
      busy({ kind: "teleporter" }),
      busy({ kind: "lineScan", sessionId: 4, probed: 1, total: 2, extra: true }),
      refusal(500, { error: "x", kind: "gatewayBusy", holder: { kind: "connecting", session: "lineScan" } }),
      refusal(502, { error: "x", kind: "gatewayRefused", gateway: "192.0.2.10:3671", gatewayStatus: 300, gatewayError: null }),
      refusal(502, { error: "x", kind: "gatewayTimeout" }),
      refusal(422, { error: "x", kind: "unsupportedByteOrder", address: "1.1.5" }),
      new Error("plain"),
      "not even an error",
    ]) {
      expect(admitGatewayRefusal(error)).toBeNull();
    }
  });
});

describe("describeGatewayRefusal", () => {
  it("names the KNXBench session that holds the tunnel, per kind", () => {
    const say = (holder: unknown) => describeGatewayRefusal(admitGatewayRefusal(busy(holder))!, en);
    expect(say({ kind: "busMonitor", sessionId: 3, gateway: "192.0.2.10:3671", state: "active" }))
      .toBe("The gateway is busy: the bus monitor is connected to 192.0.2.10:3671. Stop the bus monitor first.");
    expect(say({ kind: "lineScan", sessionId: 4, probed: 37, total: 120 }))
      .toBe("The gateway is busy: a line scan is running (37/120 addresses probed). Wait for it or cancel it first.");
    expect(say({ kind: "deviceDownload", address: "1.1.5", state: "running" }))
      .toBe("The gateway is busy: a download to device 1.1.5 is running. Wait until it has finished.");
    expect(say({ kind: "addressProgramming", address: "1.1.9", state: "waiting" }))
      .toBe("The gateway is busy: programming address 1.1.9 is waiting for a programming button. Stop it first.");
    expect(say({ kind: "connecting", session: "lineScan" }))
      .toBe("The gateway is busy: a line scan is just connecting. Try again in a moment.");
  });

  it("says why the gateway refused, with the spec constant and the status", () => {
    const text = describeGatewayRefusal(admitGatewayRefusal(refusal(502, {
      error: "x", kind: "gatewayRefused", gateway: "192.0.2.10:3671", gatewayStatus: 0x24, gatewayError: "E_NO_MORE_CONNECTIONS",
    }))!, en);
    expect(text).toBe("The gateway 192.0.2.10:3671 refused the connection: all of its connections are in use, "
      + "for example by ETS, a visualisation or another KNXBench (E_NO_MORE_CONNECTIONS, status 0x24).");
  });

  it("keeps an undefined refusal code as a bare number, without a guessed reason", () => {
    const text = describeGatewayRefusal(admitGatewayRefusal(refusal(502, {
      error: "x", kind: "gatewayRefused", gateway: "192.0.2.10:3671", gatewayStatus: 0x77, gatewayError: null,
    }))!, en);
    expect(text).toBe("The gateway 192.0.2.10:3671 refused the connection (status 0x77).");
    // A name this client does not know yet is shown verbatim, not reworded.
    const newer = describeGatewayRefusal(admitGatewayRefusal(refusal(502, {
      error: "x", kind: "gatewayRefused", gateway: "192.0.2.10:3671", gatewayStatus: 0x2f, gatewayError: "E_FUTURE",
    }))!, en);
    expect(newer).toBe("The gateway 192.0.2.10:3671 refused the connection (E_FUTURE, status 0x2F).");
  });

  it("explains a silent gateway and a declared byte order", () => {
    expect(describeGatewayRefusal(admitGatewayRefusal(refusal(502, {
      error: "x", kind: "gatewayTimeout", gateway: "192.0.2.10:3671",
    }))!, en)).toBe("The gateway 192.0.2.10:3671 did not answer. Check its address and that this computer can reach it.");
    expect(describeGatewayRefusal(admitGatewayRefusal(refusal(422, {
      error: "x", kind: "unsupportedByteOrder", address: "0.2.23", order: "LittleEndian",
    }))!, en)).toBe("Device 0.2.23 cannot be downloaded yet: its application program declares the byte order "
      + "LittleEndian, and KNXBench writes numbers high octet first only. Nothing was sent.");
  });

  it("speaks German in the Du form", () => {
    expect(describeGatewayRefusal(admitGatewayRefusal(busy(
      { kind: "busMonitor", sessionId: 3, gateway: "192.0.2.10:3671", state: "active" },
    ))!, de)).toBe("Das Gateway ist belegt: Der Busmonitor ist mit 192.0.2.10:3671 verbunden. Beende zuerst den Busmonitor.");
  });
});

describe("gatewayRefusalMessage", () => {
  it("words a structured refusal in the active language and leaves everything else alone", () => {
    active.language = "de";
    expect(gatewayRefusalMessage(busy({ kind: "lineScan", sessionId: 4, probed: 37, total: 120 })))
      .toBe("Das Gateway ist belegt: Ein Linienscan läuft (37/120 Adressen geprüft). Warte darauf oder brich ihn zuerst ab.");
    expect(gatewayRefusalMessage(new Error("plain"))).toBeNull();
  });
});

describe("holderFromActivity", () => {
  const snapshot = (sessions: ActivitySnapshot["sessions"], busyLocks: ActivitySnapshot["busyLocks"] = []): ActivitySnapshot => ({
    serverIncarnation: "test", coverage: "partial", historyState: "disabled", sessions, oneShot: [], oneShotDropped: 0,
    busyLocks, untracked: [],
  });

  it("names what holds the gateway before a download is even prepared", () => {
    expect(holderFromActivity(snapshot([{ kind: "busMonitor", id: 6, gateway: "192.0.2.10:3671", state: "active" }])))
      .toEqual({ kind: "busMonitor", sessionId: 6, gateway: "192.0.2.10:3671", state: "active" });
    expect(holderFromActivity(snapshot([{ kind: "lineScan", id: 4, state: "running", probed: 37, total: 120 }])))
      .toEqual({ kind: "lineScan", sessionId: 4, probed: 37, total: 120 });
    expect(holderFromActivity(snapshot([{ kind: "addressProgramming", id: 9, address: "1.1.5", state: "waiting" }])))
      .toEqual({ kind: "addressProgramming", sessionId: 9, address: "1.1.5", state: "waiting" });
    expect(holderFromActivity(snapshot([], ["managementOperation"])))
      .toEqual({ kind: "connecting", session: "managementOperation" });
  });

  it("ignores what no longer holds it, and the download page's own download", () => {
    expect(holderFromActivity(snapshot([
      { kind: "lineScan", id: 4, state: "completed", probed: 120, total: 120 },
      { kind: "addressProgramming", id: 9, address: "1.1.5", state: "finished" },
      { kind: "deviceDownload", id: 3, address: "1.1.67", state: "running", completedSteps: 1, totalSteps: 5,
        writtenOctets: 0, totalOctets: 10 },
    ], ["deviceDownload"]))).toBeNull();
  });

  it("never names the locks a running System-2 operation holds itself as a foreign holder (issue #30)", () => {
    expect(holderFromActivity(snapshot([], ["system2Operation", "deviceDownload", "managementOperation",
      "busMonitorOrGroupWrite", "lineScan"]))).toBeNull();
    expect(holderFromActivity(snapshot([], ["system2Operation"]))).toBeNull();
    expect(holderFromActivity(snapshot([
      { kind: "busMonitor", id: 6, gateway: "192.0.2.10:3671", state: "active" },
    ], ["system2Operation", "managementOperation"]))).toBeNull();
  });

  it("words a System-2 operation where it is named as the holder", () => {
    const holder = { kind: "connecting", session: "system2Operation" } as const;
    expect(describeHolder(holder, en))
      .toBe("The gateway is busy: an experimental System-2 operation is running. Try again when it has ended.");
    expect(describeHolder(holder, de))
      .toBe("Das Gateway ist belegt: Ein experimenteller System-2-Vorgang läuft. Versuch es erneut, wenn er beendet ist.");
  });

  it("words a monitor from an older server, which does not name its gateway", () => {
    const holder = holderFromActivity(snapshot([{ kind: "busMonitor", id: 6, state: "active" }]))!;
    expect(describeHolder(holder, en)).toBe("The gateway is busy: a bus monitor session is open. Stop the bus monitor first.");
  });
});

describe("serverStopping (Goal A3, KL-163)", () => {
  const stopping = (status = 503, extra: Record<string, unknown> = {}) => refusal(status, {
    error: "the server is stopping; nothing was started and nothing was sent", kind: "serverStopping", ...extra,
  });
  it("words the stop-admission refusal in both languages instead of a bus error", () => {
    expect(admitGatewayRefusal(stopping())).toEqual({ kind: "serverStopping" });
    expect(describeGatewayRefusal({ kind: "serverStopping" }, en)).toBe(en("gatewayRefusal.serverStopping"));
    expect(en("gatewayRefusal.serverStopping")).toContain("server is stopping");
    expect(de("gatewayRefusal.serverStopping")).toContain("Server wird beendet");
    active.language = "de";
    expect(gatewayRefusalMessage(stopping())).toBe(de("gatewayRefusal.serverStopping"));
  });
  it("keeps the server's own text for any other status or an unexpected shape", () => {
    expect(admitGatewayRefusal(stopping(409))).toBeNull();
    expect(admitGatewayRefusal(stopping(503, { retryAfter: 5 }))).toBeNull();
    expect(gatewayRefusalMessage(stopping(500))).toBeNull();
  });
});
