// @vitest-environment happy-dom
import { readFileSync } from "node:fs";
import { act } from "react";
import { createRoot } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type {
  BusDiscoverResponse,
  BusDiscoveredInterface,
  BusMonitorStartResponse,
  BusMonitorStopResponse,
  BusMonitorTelegramsResponse,
  BusTelegramRow,
} from "./api";

const apiMock = vi.hoisted(() => ({
  startBusMonitor: vi.fn(),
  stopBusMonitor: vi.fn(),
  pollBusTelegrams: vi.fn(),
  writeBusValue: vi.fn(),
  // T25: the panel searches for interfaces on mount, so every test in
  // this file reaches this one whether it cares about discovery or not.
  discoverBusInterfaces: vi.fn(),
  // U20: the flow view's participant snapshot (a read).
  fetchFlowSnapshot: vi.fn(),
}));

vi.mock("./api", () => ({
  ...apiMock,
  errorMessage: (error: unknown) => (error instanceof Error ? error.message : String(error)),
  // Duck-typed, mirroring `api.ts`'s real `errorStatus` — a mocked
  // rejection just needs a `.status` number property, no class export to
  // thread through this factory.
  errorStatus: (error: unknown) =>
    error && typeof error === "object" && "status" in error && typeof (error as { status: unknown }).status === "number"
      ? (error as { status: number }).status
      : undefined,
}));

const captureMock = vi.hoisted(() => ({ saveBusCapture: vi.fn() }));
vi.mock("./busMonitorCapture", async (importOriginal) => ({
  ...await importOriginal<typeof import("./busMonitorCapture")>(),
  saveBusCapture: captureMock.saveBusCapture,
}));

import BusMonitorPanel from "./BusMonitorPanel";
import { CAPTURE_CAPACITY } from "./busMonitorCapture";
// Not mocked: `busContext` is the unit under test here as much as the
// panel is. Its whole job is a pair of `localStorage` records, which
// happy-dom implements for real, so a mock would only prove that the mock
// agrees with itself.
import { publishProjectContext, recordSessionContext } from "./busContext";
// Not mocked either: the discovery store is module state the panel reads
// through `useSyncExternalStore`, and a mock of it would only prove the
// mock agrees with itself. It does need resetting between tests — see
// `resetBusDiscoveryForTests`'s own comment on why module state outlives
// a component.
import { resetBusDiscoveryForTests } from "./busDiscovery";
import { splitGatewayEndpoint } from "./gatewayEndpoint";
import { savePreferredGateway } from "./gatewayPreference";
import { getSetting, initSettings, resetSettingsForTests, setSetting } from "./settingsStore";
import { UI_LANGUAGE_STORAGE_KEY } from "./uiLanguage";
import type { ProjectTree } from "./bindings/ProjectTree";
import { snapshotJson } from "./flowTestFixtures";
import { parseFlowSnapshot } from "./flowWire";

// `act()` only flushes reliably when this is set (React 19's own check,
// `isConcurrentActEnvironment`) — `LogPanel.test.tsx` never needs it
// because its one `act()` call wraps the *initial* `render()`, which is
// enough for React to infer the environment on its own; this file's
// `act()` calls wrap later `dispatchEvent`s and `vi.advanceTimersByTimeAsync`
// calls too, which need it set explicitly.
(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

// Matches the named `POLL_INTERVAL_MS` constant in `BusMonitorPanel.tsx`
// (not imported — that constant is deliberately module-private, same as
// every other internal constant in this codebase's components).
const POLL_INTERVAL_MS = 1000;

let host: HTMLDivElement | undefined;

function row(overrides: Partial<BusTelegramRow>): BusTelegramRow {
  return {
    seq: 0,
    timestamp: "2026-09-11T12:00:00Z",
    source: "1.1.5",
    destination: "1/2/3",
    destinationName: null,
    service: "GroupValueWrite",
    rawPayload: "0x01 (6-bit)",
    decoded: { kind: "value", dpt: "DPST-1-1", text: "On" },
    ...overrides,
  };
}

it("opens full telegram details from the keyboard without sending a value", async () => {
  apiMock.pollBusTelegrams.mockResolvedValue(telegramsResponse({ telegrams: [row({ seq: 7, destinationName: "Example light" })] }));
  const root = await renderPanel();
  await flushReattach();
  const telegram = host!.querySelector<HTMLElement>("tbody tr")!;
  expect(telegram.tabIndex).toBe(0);
  await act(async () => telegram.dispatchEvent(new KeyboardEvent("keydown", { key: "Enter", bubbles: true })));
  const details = host!.querySelector<HTMLElement>(".telegram-details")!;
  expect(details).not.toBeNull();
  expect(details.textContent).toContain("Example light");
  expect(details.textContent).toContain("0x01 (6-bit)");
  expect(details.textContent).toContain("DPST-1-1");
  expect(apiMock.writeBusValue).not.toHaveBeenCalled();
  await act(async () => root.unmount());
});

it("shows priority, hop count and repeat evidence without inventing it for other frames", async () => {
  apiMock.pollBusTelegrams.mockResolvedValue(telegramsResponse({ telegrams: [
    row({ seq: 1, control: { priority: "urgent", repeated: true, hopCount: 0 } }),
    row({ seq: 2, control: { priority: "system", repeated: false, hopCount: 7 } }),
    row({ seq: 3, control: { priority: "normal", repeated: null, hopCount: 6 } }),
    row({ seq: 4, service: "SessionClosed", control: null }),
    row({ seq: 5 }), // An older server omits this additive field.
  ] }));
  const root = await renderPanel();
  await flushReattach();
  expect([...host!.querySelectorAll("thead th")].map((cell) => cell.textContent)).toContain("Control");
  const rows = [...host!.querySelectorAll("tbody tr")];
  expect(rows[0].querySelector(".bus-monitor-control")?.textContent).toContain("Urgent");
  expect(rows[0].querySelector(".bus-monitor-control")?.textContent).toContain("Hop count: 0");
  expect(rows[0].querySelector(".bus-monitor-control")?.textContent).toContain("Repeated");
  expect(rows[1].querySelector(".bus-monitor-control")?.textContent).toContain("Not repeated");
  expect(rows[1].querySelector(".bus-monitor-control")?.textContent).toContain("Hop count: 7");
  expect(rows[2].querySelector(".bus-monitor-control")?.textContent).toContain("Normal");
  expect(rows[2].querySelector(".bus-monitor-control")?.textContent).not.toMatch(/repeated/i);
  expect(rows[3].querySelector(".bus-monitor-control")?.textContent).toBe("—");
  expect(rows[4].querySelector(".bus-monitor-control")?.textContent).toBe("—");
  await act(async () => rows[2].dispatchEvent(new KeyboardEvent("keydown", { key: "Enter", bubbles: true })));
  const details = host!.querySelector(".telegram-details")!;
  expect(details.textContent).toContain("Hop count: 6");
  expect(details.querySelector(".bus-monitor-control-repeat")).toBeNull();
  expect(apiMock.writeBusValue).not.toHaveBeenCalled();
  await act(async () => root.unmount());
});

it("localizes monitor control labels but keeps unknown priorities explicit", async () => {
  setSetting(UI_LANGUAGE_STORAGE_KEY, "de");
  apiMock.pollBusTelegrams.mockResolvedValue(telegramsResponse({ telegrams: [
    row({ seq: 6, control: { priority: "urgent", repeated: false, hopCount: 0 } }),
    row({ seq: 7, control: { priority: "future-priority", repeated: null, hopCount: 1 } }),
  ] }));
  const root = await renderPanel();
  await flushReattach();
  const cells = [...host!.querySelectorAll(".bus-monitor-control")];
  expect(cells[0].textContent).toContain("Dringend");
  expect(cells[0].textContent).toContain("Hop-Zähler: 0");
  expect(cells[0].textContent).toContain("Nicht wiederholt");
  expect(cells[1].textContent).toContain("Unbekannte Priorität (future-priority)");
  expect(cells[1].textContent).not.toContain("wiederholt");
  await act(async () => root.unmount());
});

function telegramsResponse(overrides: Partial<BusMonitorTelegramsResponse>): BusMonitorTelegramsResponse {
  return {
    sessionId: 1,
    serverIncarnation: "process-a",
    contextStatus: "current",
    projectOpen: true,
    status: "active",
    nextSince: 1,
    droppedBefore: 0,
    telegrams: [],
    ...overrides,
  };
}

// The shape every "no session yet" mount reattach expects to see rejected
// with — `errorStatus()` above reads `.status` off exactly this shape,
// same as the real `request()` in `api.ts` attaches to a genuine `404`.
function notFoundError(): Error {
  const error = new Error("no active bus session") as Error & { status: number };
  error.status = 404;
  return error;
}

async function renderPanel(projectOpen = true) {
  host = document.createElement("div");
  document.body.appendChild(host);
  const root = createRoot(host);
  await act(async () => {
    root.render(<BusMonitorPanel projectOpen={projectOpen} />);
  });
  return root;
}

// The mount-time reattach effect (`GET /telegrams` against whatever
// session may already exist) fires its `pollBusTelegrams(0)` call inside
// a `useEffect`, so `renderPanel()`'s own `act()` does not wait for that
// promise to settle — same reasoning as `connect()`'s own comment below,
// this is vitest's documented way to drain a pending microtask chain
// while fake timers are active.
async function flushReattach() {
  await act(async () => {
    await vi.advanceTimersByTimeAsync(0);
  });
}

function clickButton(label: string) {
  const button = Array.from(host!.querySelectorAll("button")).find((b) => b.textContent === label);
  if (!button) throw new Error(`no button labelled "${label}"`);
  button.dispatchEvent(new MouseEvent("click", { bubbles: true }));
}

// A plain `input.value = x` does not make React's controlled `<input>`
// see a change: React installs its own tracking setter on the DOM node,
// so a direct assignment updates that tracker too, and the "input" event
// that follows then looks like a no-op to React's change-detection and
// never calls `onChange`. Going through the *prototype's* native setter
// instead bypasses React's override, leaving the tracker stale so the
// event is recognised as a genuine change — the same trick
// `@testing-library/react`'s `fireEvent.change` uses internally.
function setInputValue(selector: string, value: string) {
  const input = host!.querySelector<HTMLInputElement>(selector)!;
  const setter = Object.getOwnPropertyDescriptor(window.HTMLInputElement.prototype, "value")!.set!;
  setter.call(input, value);
  input.dispatchEvent(new Event("input", { bubbles: true }));
}

// Types a gateway address and clicks Connect. `vi.advanceTimersByTimeAsync(0)`
// (not a bare `await Promise.resolve()`) is what actually drains the
// `startBusMonitor` → `setSession` → effect → `pollBusTelegrams` chain
// under fake timers — it is vitest's own documented way to flush pending
// microtasks while fake timers are active, needed because plain awaits
// inside `act()` are not enough to observe the initial, immediate poll
// `BusMonitorPanel`'s effect fires as soon as `session` is set.
async function connect(gateway = "192.0.2.1:3671") {
  const fields = splitGatewayEndpoint(gateway);
  await act(async () => {
    setInputValue(".bus-monitor-host", fields.host);
    setInputValue(".bus-monitor-port", fields.port);
  });
  await act(async () => {
    clickButton("Connect");
    await vi.advanceTimersByTimeAsync(0);
  });
}

beforeEach(() => {
  vi.useFakeTimers();
  captureMock.saveBusCapture.mockResolvedValue(true);
  // Cross-window context records outlive a component; without this a
  // session record written by one test would decide the next one's lock.
  window.localStorage.clear();
  apiMock.startBusMonitor.mockResolvedValue({
    sessionId: 1,
    serverIncarnation: "process-a",
    assignedAddress: "1.1.5",
  } satisfies BusMonitorStartResponse);
  // Default: no session exists yet when the panel mounts, same as every
  // test before the mount-reattach fix existed. Tests that want to
  // exercise reattaching to a pre-existing session override this
  // (`mockResolvedValueOnce`, so only the mount-time call is affected)
  // before calling `renderPanel()`.
  apiMock.pollBusTelegrams.mockRejectedValue(notFoundError());
  // T25 default: a search that finds nothing. The common case on a
  // developer laptop, and the answer that keeps the connect form the
  // only thing every other test in this file has to look at.
  resetBusDiscoveryForTests();
  apiMock.discoverBusInterfaces.mockResolvedValue({ interfaces: [] });
  apiMock.stopBusMonitor.mockResolvedValue({
    sessionId: 1,
    serverIncarnation: "process-a",
    telegramCount: 0,
    droppedCount: 0,
  } satisfies BusMonitorStopResponse);
});

afterEach(() => {
  resetBusDiscoveryForTests();
  host?.remove();
  host = undefined;
  vi.clearAllMocks();
  vi.unstubAllGlobals();
  vi.useRealTimers();
  resetSettingsForTests();
});

it("edits host and port separately and composes the existing start payload", async () => {
  await renderPanel();
  const hostInput = host!.querySelector<HTMLInputElement>(".bus-monitor-host")!;
  const portInput = host!.querySelector<HTMLInputElement>(".bus-monitor-port")!;
  expect(hostInput.getAttribute("aria-label")).toBe("Gateway host");
  expect(portInput.getAttribute("aria-label")).toBe("Gateway port");
  expect(portInput.value).toBe("3671");
  await act(async () => {
    setInputValue(".bus-monitor-host", "192.0.2.17");
    setInputValue(".bus-monitor-port", "4750");
  });
  await act(async () => {
    clickButton("Connect");
    await vi.advanceTimersByTimeAsync(0);
  });
  expect(apiMock.startBusMonitor).toHaveBeenCalledWith("192.0.2.17:4750");
});

it("explains both endpoint fields and does not claim discovery is absent", async () => {
  await renderPanel();
  const hint = host!.querySelector<HTMLElement>(".bus-monitor-connect [role='tooltip']")!;
  expect(hint.textContent).toContain("separate");
  expect(hint.textContent).toContain("Search");
  expect(hint.textContent).not.toContain("does not search");
});

it("shows a port validation error and does not contact the gateway", async () => {
  await renderPanel();
  await act(async () => {
    setInputValue(".bus-monitor-host", "192.0.2.17");
    setInputValue(".bus-monitor-port", "65536");
  });
  expect(host!.querySelector<HTMLButtonElement>(".bus-monitor-connect button")!.disabled).toBe(true);
  expect(host!.querySelector(".bus-monitor-port-error")!.textContent).toContain("1–65535");
  expect(apiMock.startBusMonitor).not.toHaveBeenCalled();
});

it("retains unsupported hostname preferences but refuses to open a tunnel", async () => {
  savePreferredGateway("gateway.local:4921");
  await renderPanel();
  const hostInput = host!.querySelector<HTMLInputElement>(".bus-monitor-host")!;
  expect(hostInput.value).toBe("gateway.local");
  expect(hostInput.getAttribute("aria-invalid")).toBe("true");
  expect(host!.querySelector<HTMLInputElement>(".bus-monitor-port")!.value).toBe("4921");
  expect(host!.querySelector(".bus-monitor-host-error")!.textContent).toContain("Hostnames and IPv6");
  expect(host!.querySelector<HTMLButtonElement>(".bus-monitor-connect button")!.disabled).toBe(true);
  expect(apiMock.startBusMonitor).not.toHaveBeenCalled();
  expect(getSetting("preferredGateway")).toBe("gateway.local:4921");
});

it("explains where a pasted combined endpoint's port belongs", async () => {
  await renderPanel();
  await act(async () => setInputValue(".bus-monitor-host", "192.0.2.17:3671"));
  expect(host!.querySelector(".bus-monitor-host-error")!.textContent).toContain("separate port field");
  expect(host!.querySelector<HTMLButtonElement>(".bus-monitor-connect button")!.disabled).toBe(true);
  expect(apiMock.startBusMonitor).not.toHaveBeenCalled();
});

it("separates a discovered endpoint without connecting automatically", async () => {
  apiMock.discoverBusInterfaces.mockResolvedValue({
    interfaces: [{
      controlEndpoint: "192.0.2.40:4921",
      individualAddress: "1.1.1",
      friendlyName: "Test interface",
      supportsTunnelling: true,
    }],
  } satisfies BusDiscoverResponse);
  await renderPanel();
  const option = host!.querySelector<HTMLButtonElement>(".bus-discovery-option")!;
  await act(async () => option.dispatchEvent(new MouseEvent("click", { bubbles: true })));
  expect(host!.querySelector<HTMLInputElement>(".bus-monitor-host")!.value).toBe("192.0.2.40");
  expect(host!.querySelector<HTMLInputElement>(".bus-monitor-port")!.value).toBe("4921");
  expect(apiMock.startBusMonitor).not.toHaveBeenCalled();
});

it("seeds a fresh gateway field from the cached preference without writing it back", async () => {
  savePreferredGateway("192.0.2.10:3671");
  await renderPanel();
  expect(host!.querySelector<HTMLInputElement>(".bus-monitor-host")!.value).toBe("192.0.2.10");
  expect(host!.querySelector<HTMLInputElement>(".bus-monitor-port")!.value).toBe("3671");
  expect(getSetting("preferredGateway")).toBe("192.0.2.10:3671");
  expect(apiMock.startBusMonitor).not.toHaveBeenCalled();
});

it("adopts the authoritative gateway after a normal 404 reattach", async () => {
  savePreferredGateway("192.0.2.10:3671");
  let resolveGet!: (response: Response) => void;
  vi.stubGlobal("fetch", vi.fn(() => new Promise<Response>((resolve) => { resolveGet = resolve; })));
  const hydration = initSettings();
  await renderPanel();
  await flushReattach();
  resolveGet({
    ok: true,
    status: 200,
    json: () => Promise.resolve({
      schemaVersion: 1,
      status: "ok",
      settings: { preferredGateway: "192.0.2.20:3671" },
    }),
  } as Response);
  await act(async () => { await hydration; });
  expect(host!.querySelector<HTMLInputElement>(".bus-monitor-host")!.value).toBe("192.0.2.20");
  expect(host!.querySelector<HTMLInputElement>(".bus-monitor-port")!.value).toBe("3671");
});

it("keeps manual gateway input typed before authoritative hydration", async () => {
  let resolveGet!: (response: Response) => void;
  vi.stubGlobal("fetch", vi.fn(() => new Promise<Response>((resolve) => { resolveGet = resolve; })));
  const hydration = initSettings();
  await renderPanel();
  await act(async () => setInputValue(".bus-monitor-host", "192.0.2.30"));
  resolveGet({
    ok: true,
    status: 200,
    json: () => Promise.resolve({
      schemaVersion: 1,
      status: "ok",
      settings: { preferredGateway: "192.0.2.20:3671" },
    }),
  } as Response);
  await act(async () => { await hydration; });
  expect(host!.querySelector<HTMLInputElement>(".bus-monitor-host")!.value).toBe("192.0.2.30");
  expect(host!.querySelector<HTMLInputElement>(".bus-monitor-port")!.value).toBe("3671");
  expect(getSetting("preferredGateway")).toBe("192.0.2.20:3671");
});

it("keeps a discovered gateway selected before authoritative hydration", async () => {
  apiMock.discoverBusInterfaces.mockResolvedValue({
    interfaces: [{
      controlEndpoint: "192.0.2.40:3671",
      individualAddress: "1.1.1",
      friendlyName: "Test interface",
      supportsTunnelling: true,
    }],
  } satisfies BusDiscoverResponse);
  let resolveGet!: (response: Response) => void;
  vi.stubGlobal("fetch", vi.fn(() => new Promise<Response>((resolve) => { resolveGet = resolve; })));
  const hydration = initSettings();
  await renderPanel();
  await flushReattach();
  const option = host!.querySelector<HTMLButtonElement>(".bus-discovery-option")!;
  await act(async () => option.dispatchEvent(new MouseEvent("click", { bubbles: true })));
  resolveGet({
    ok: true,
    status: 200,
    json: () => Promise.resolve({
      schemaVersion: 1,
      status: "ok",
      settings: { preferredGateway: "192.0.2.20:3671" },
    }),
  } as Response);
  await act(async () => { await hydration; });
  expect(host!.querySelector<HTMLInputElement>(".bus-monitor-host")!.value).toBe("192.0.2.40");
  expect(host!.querySelector<HTMLInputElement>(".bus-monitor-port")!.value).toBe("3671");
  expect(getSetting("preferredGateway")).toBe("192.0.2.20:3671");
});

describe("BusMonitorPanel", () => {
  describe("mount-time reattach", () => {
    it("shows the Connect form when GET /telegrams 404s (no session exists)", async () => {
      await renderPanel();
      await flushReattach();

      expect(apiMock.pollBusTelegrams).toHaveBeenCalledWith(0);
      expect(host!.querySelector(".bus-monitor-connect")).not.toBeNull();
      expect(host!.querySelector(".bus-monitor-session")).toBeNull();
      expect(host!.querySelector(".bus-monitor-table")).toBeNull();
    });

    it("reattaches to an existing session, adopting its rows/cursor/status and its droppedBefore reaching the gap notice", async () => {
      apiMock.pollBusTelegrams.mockReset();
      apiMock.pollBusTelegrams.mockResolvedValueOnce(
        telegramsResponse({
          sessionId: 5,
          status: "active",
          nextSince: 3,
          droppedBefore: 2,
          telegrams: [row({ seq: 0, destination: "1/2/3" })],
        }),
      );
      apiMock.pollBusTelegrams.mockResolvedValue(telegramsResponse({ sessionId: 5, nextSince: 3, droppedBefore: 2 }));

      await renderPanel();
      await flushReattach();

      expect(host!.querySelector(".bus-monitor-session")!.textContent).toContain("Session 5");
      expect(host!.textContent).toContain("1/2/3");
      expect(host!.querySelector(".bus-monitor-gap-notice")!.textContent).toContain("2");
      expect(host!.textContent).toContain("Disconnect");
      expect(host!.querySelector(".bus-monitor-connect > button")!.textContent).toBe("Disconnect");

      // The reattach poll already delivered the first batch of rows — the
      // per-session polling effect must not immediately re-poll with a
      // redundant request right on top of it.
      expect(apiMock.pollBusTelegrams).toHaveBeenCalledTimes(1);

      // The next interval tick resumes from the reattach response's own
      // cursor (3), not from zero — a reattach must never restart the
      // stream, or every row before it would look freshly "dropped."
      await act(async () => {
        await vi.advanceTimersByTimeAsync(POLL_INTERVAL_MS);
      });
      expect(apiMock.pollBusTelegrams).toHaveBeenLastCalledWith(3);
    });

    it("reattaches to a session the gateway already closed — rows shown read-only, Stop still works", async () => {
      apiMock.pollBusTelegrams.mockReset();
      apiMock.pollBusTelegrams.mockResolvedValueOnce(
        telegramsResponse({ sessionId: 7, status: "closed", nextSince: 1, telegrams: [row({ seq: 0 })] }),
      );
      apiMock.pollBusTelegrams.mockResolvedValue(telegramsResponse({ sessionId: 7, status: "closed", nextSince: 1 }));

      await renderPanel();
      await flushReattach();

      expect(host!.querySelector(".bus-monitor-session")!.textContent).toContain("closed by gateway");
      expect(host!.querySelector(".bus-monitor-table")).not.toBeNull();
      // Task 5 review, fix 2 — a session reattached already-closed is exactly
      // the case the compose form must refuse to send through.
      expect(host!.querySelector<HTMLInputElement>(".bus-compose-value")!.disabled).toBe(true);

      await act(async () => {
        clickButton("Disconnect");
        await vi.advanceTimersByTimeAsync(0);
      });
      expect(apiMock.stopBusMonitor).toHaveBeenCalledTimes(1);
      expect(host!.textContent).toContain("Stopped session");
    });

    it("does not swallow a non-404 error from the reattach check", async () => {
      apiMock.pollBusTelegrams.mockReset();
      apiMock.pollBusTelegrams.mockRejectedValue(new Error("gateway unreachable"));

      await renderPanel();
      await flushReattach();

      expect(host!.querySelector(".field-error")!.textContent).toBe("gateway unreachable");
      expect(host!.querySelector(".bus-monitor-session")).toBeNull();
    });
  });

  it("renders rows from a mocked poll", async () => {
    await renderPanel();
    await flushReattach();
    apiMock.pollBusTelegrams.mockResolvedValue(
      telegramsResponse({
        telegrams: [
          row({ seq: 0, destination: "1/2/3", destinationName: "Living room light" }),
          row({ seq: 1, destination: "4/5/6", service: "GroupValueRead", decoded: null, rawPayload: null }),
        ],
        nextSince: 2,
      }),
    );

    await connect();

    expect(apiMock.pollBusTelegrams).toHaveBeenCalledWith(0);
    expect(host!.textContent).toContain("1/2/3");
    expect(host!.textContent).toContain("Living room light");
    expect(host!.textContent).toContain("4/5/6");
    expect(host!.querySelectorAll(".bus-monitor-table tbody tr")).toHaveLength(2);
  });

  it("labels unresolved, conflicting, unsupported and malformed decoded payloads without inferring from prose", async () => {
    await renderPanel();
    await flushReattach();
    apiMock.pollBusTelegrams.mockResolvedValue(telegramsResponse({ nextSince: 4, telegrams: [
      row({ seq: 0, decoded: { kind: "unresolved", text: "no project open" } }),
      row({ seq: 1, decoded: { kind: "conflict", text: "conflicting DPTs: DPST-1-1, DPST-5-1" } }),
      row({ seq: 2, decoded: { kind: "error", dpt: "DPST-40-1", reason: "unsupportedDpt", text: "unsupported datapoint type", error: "unsupported datapoint type" } }),
      row({ seq: 3, decoded: { kind: "error", dpt: "DPST-1-1", reason: "decodeFailed", text: "wrong payload length", error: "wrong payload length" } }),
    ] }));
    await connect();
    const states = Array.from(host!.querySelectorAll(".bus-monitor-decode-state"), (node) => node.textContent);
    expect(states).toEqual(["No DPT assigned", "Conflicting DPTs", "Unsupported DPT", "Decode failed"]);
    expect(host!.querySelectorAll(".bus-monitor-decoded-error")).toHaveLength(2);
    expect(host!.textContent).toContain("DPST-40-1");
    expect(host!.textContent).toContain("wrong payload length");
  });

  it("does not guess the cause of a legacy decode error without a structured reason", async () => {
    await renderPanel();
    await flushReattach();
    apiMock.pollBusTelegrams.mockResolvedValue(telegramsResponse({ nextSince: 1, telegrams: [
      row({ seq: 0, decoded: { kind: "error", text: "unsupported datapoint type", error: "unsupported datapoint type" } }),
    ] }));
    await connect();
    expect(host!.querySelector(".bus-monitor-decode-state")!.textContent).toBe("Decode error (reason unknown)");
    expect(host!.textContent).toContain("unsupported datapoint type");
  });

  it("exports retained raw and decoded rows after disconnect without fetching or supplying a file path", async () => {
    const retained = [
      row({ seq: 0, rawPayload: "0x01 (6-bit)", decoded: null }),
      row({ seq: 1, decoded: { kind: "value", dpt: "DPST-1-1", text: "On" } }),
      row({ seq: 2, decoded: { kind: "error", dpt: "DPST-1-1", reason: "decodeFailed", text: "wrong length", error: "wrong length" } }),
    ];
    apiMock.pollBusTelegrams.mockRejectedValueOnce(notFoundError());
    apiMock.pollBusTelegrams.mockResolvedValue(telegramsResponse({ nextSince: 3, droppedBefore: 4, telegrams: retained }));
    await renderPanel();
    await flushReattach();
    await connect();
    // Disconnect's context signal checks for a surviving session. Model the
    // stopped server's 404, not the previous live poll's canned response.
    apiMock.pollBusTelegrams.mockRejectedValue(notFoundError());
    await act(async () => { clickButton("Disconnect"); await vi.advanceTimersByTimeAsync(0); });
    expect(host!.querySelectorAll(".bus-monitor-table tbody tr")).toHaveLength(3);
    expect(host!.querySelector(".bus-monitor-retained")?.textContent).toContain("Retained capture from a closed session");
    expect(host!.querySelector(".bus-monitor-export-note")?.textContent).toContain("addresses and payloads");
    const polls = apiMock.pollBusTelegrams.mock.calls.length;
    await act(async () => { clickButton("Export capture"); await vi.advanceTimersByTimeAsync(0); });
    expect(captureMock.saveBusCapture).toHaveBeenCalledWith(retained, expect.objectContaining({
      sessionId: 1, serverIncarnation: "process-a", status: "closed",
      serverDroppedBefore: 4, clientPrunedCount: 0, exportedAt: expect.any(String),
    }));
    expect(apiMock.pollBusTelegrams).toHaveBeenCalledTimes(polls);
  });

  it("treats a cancelled export quietly and reports a save failure without losing the capture", async () => {
    apiMock.pollBusTelegrams.mockRejectedValueOnce(notFoundError());
    apiMock.pollBusTelegrams.mockResolvedValue(telegramsResponse({ nextSince: 1, telegrams: [row({ seq: 0 })] }));
    await renderPanel();
    await flushReattach();
    await connect();
    captureMock.saveBusCapture.mockResolvedValueOnce(false).mockRejectedValueOnce(new Error("disk full"));
    await act(async () => { clickButton("Export capture"); await vi.advanceTimersByTimeAsync(0); });
    expect(host!.querySelector(".field-error")).toBeNull();
    expect(host!.querySelector<HTMLButtonElement>(".bus-monitor-export")!.disabled).toBe(false);
    await act(async () => { clickButton("Export capture"); await vi.advanceTimersByTimeAsync(0); });
    expect(host!.querySelector(".field-error")!.textContent).toContain("disk full");
    expect(host!.querySelectorAll(".bus-monitor-table tbody tr")).toHaveLength(1);
    expect(host!.querySelector<HTMLButtonElement>(".bus-monitor-export")!.disabled).toBe(false);
  });

  it("clicking a row prefills the compose form's destination above the table", async () => {
    await renderPanel();
    await flushReattach();
    apiMock.pollBusTelegrams.mockResolvedValue(
      telegramsResponse({
        telegrams: [row({ seq: 0, destination: "1/2/3", decoded: { kind: "value", dpt: "DPST-1-1", text: "On" } })],
        nextSince: 1,
      }),
    );
    await connect();

    const tr = host!.querySelector(".bus-monitor-table tbody tr")!;
    await act(async () => {
      tr.dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });

    const destinationInput = host!.querySelector<HTMLInputElement>(".bus-compose-destination")!;
    expect(destinationInput.value).toBe("1/2/3");
    const dptInput = host!.querySelector<HTMLInputElement>(".bus-compose-dpt")!;
    expect(dptInput.value).toBe("DPST-1-1");
  });

  it("keeps the existing text and service filters labelled and wide enough to reach", async () => {
    apiMock.pollBusTelegrams.mockRejectedValueOnce(notFoundError());
    apiMock.pollBusTelegrams.mockResolvedValue(telegramsResponse({ nextSince: 1, telegrams: [row({ seq: 0 })] }));
    await renderPanel();
    await flushReattach();
    await connect();
    const search = host!.querySelector<HTMLInputElement>(".bus-monitor-filters input[type=text]")!;
    expect(search.getAttribute("aria-label")).toBe("Filter by destination or name");
    expect(host!.querySelectorAll(".bus-monitor-filter input[type=checkbox]")).toHaveLength(4);
    const scroll = host!.querySelector<HTMLElement>(".monitor-table-scroll")!;
    expect(scroll.getAttribute("role")).toBe("region");
    expect(scroll.getAttribute("aria-label")).toBe("Telegram table; scroll horizontally");
    expect(scroll.tabIndex).toBe(0);
    const css = readFileSync("src/styles.css", "utf8");
    expect(css).toMatch(/\.bus-monitor-filters input\[type="text"\]\s*\{[^}]*flex:\s*1 1 18rem;[^}]*min-width:\s*min\(100%, 18rem\);/s);
  });

  it("filters rows client-side, without re-fetching", async () => {
    await renderPanel();
    await flushReattach();
    apiMock.pollBusTelegrams.mockResolvedValue(
      telegramsResponse({
        telegrams: [
          row({ seq: 0, destination: "1/2/3", destinationName: "Living room light", service: "GroupValueWrite" }),
          row({ seq: 1, destination: "4/5/6", destinationName: null, service: "GroupValueRead", decoded: null, rawPayload: null }),
        ],
        nextSince: 2,
      }),
    );
    await connect();
    expect(apiMock.pollBusTelegrams).toHaveBeenCalledTimes(2); // reattach (404) + first poll

    const pollCallsBefore = apiMock.pollBusTelegrams.mock.calls.length;

    // Text filter narrows to the matching destination only.
    await act(async () => {
      setInputValue(".bus-monitor-filters input[type=text]", "1/2/3");
    });
    // Statistics deliberately cover all retained rows, independently of
    // the current table filter. Assert on the filtered table, not the
    // collapsed statistics disclosure's hidden textContent.
    const table = () => host!.querySelector(".bus-monitor-table")!.textContent;
    expect(table()).toContain("1/2/3");
    expect(table()).not.toContain("4/5/6");
    expect(apiMock.pollBusTelegrams.mock.calls.length).toBe(pollCallsBefore);

    // Clearing it brings the other row back.
    await act(async () => {
      setInputValue(".bus-monitor-filters input[type=text]", "");
    });
    expect(table()).toContain("4/5/6");
    expect(apiMock.pollBusTelegrams.mock.calls.length).toBe(pollCallsBefore);

    // The service checkbox filter hides a row by service instead.
    const writeCheckbox = Array.from(host!.querySelectorAll<HTMLInputElement>(".bus-monitor-filter"))
      .find((label) => label.textContent === "GroupValueWrite")!
      .querySelector<HTMLInputElement>("input")!;
    await act(async () => {
      writeCheckbox.dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });
    expect(table()).not.toContain("1/2/3");
    expect(table()).toContain("4/5/6");
    expect(apiMock.pollBusTelegrams.mock.calls.length).toBe(pollCallsBefore);
  });

  it("shows the gap notice, with the current count, when droppedBefore grows", async () => {
    await renderPanel();
    await flushReattach();
    apiMock.pollBusTelegrams.mockResolvedValueOnce(telegramsResponse({ droppedBefore: 0, nextSince: 1 }));

    await connect();
    expect(host!.querySelector(".bus-monitor-gap-notice")).toBeNull();

    apiMock.pollBusTelegrams.mockResolvedValue(telegramsResponse({ droppedBefore: 7, nextSince: 1 }));
    await act(async () => {
      await vi.advanceTimersByTimeAsync(POLL_INTERVAL_MS);
    });

    const notice = host!.querySelector(".bus-monitor-gap-notice");
    expect(notice).not.toBeNull();
    expect(notice!.textContent).toContain("7");

    // A later, bigger jump updates the same banner rather than stacking a
    // second one.
    apiMock.pollBusTelegrams.mockResolvedValue(telegramsResponse({ droppedBefore: 15, nextSince: 1 }));
    await act(async () => {
      await vi.advanceTimersByTimeAsync(POLL_INTERVAL_MS);
    });
    expect(host!.querySelectorAll(".bus-monitor-gap-notice")).toHaveLength(1);
    expect(host!.querySelector(".bus-monitor-gap-notice")!.textContent).toContain("15");
  });

  it("renders the server's error text when start fails, and never polls beyond the mount-time reattach check", async () => {
    apiMock.startBusMonitor.mockRejectedValue(new Error("gateway refused the connection"));

    await renderPanel();
    await flushReattach();
    expect(apiMock.pollBusTelegrams).toHaveBeenCalledTimes(1); // the 404 reattach check only

    await connect();

    expect(host!.querySelector(".field-error")!.textContent).toBe("gateway refused the connection");
    expect(apiMock.pollBusTelegrams).toHaveBeenCalledTimes(1); // still just the reattach check
    expect(host!.querySelector(".bus-monitor-table")).toBeNull();
  });

  describe("new-row entry highlight (design D34)", () => {
    // jsdom does not run CSS animations, so these assert on the marker
    // class `BusMonitorPanel.tsx` computes — never on computed styles or on
    // whether anything visibly animates. A test that faked the latter would
    // be lying about what jsdom can actually observe.
    function rowElement(seq: number): Element {
      return Array.from(host!.querySelectorAll(".bus-monitor-table tbody tr")).find(
        (tr) => tr.querySelector("td")!.textContent === String(seq),
      )!;
    }

    it("marks rows from the most recent poll as new, and not rows from an earlier one", async () => {
      await renderPanel();
      await flushReattach();
      apiMock.pollBusTelegrams.mockResolvedValue(
        telegramsResponse({ telegrams: [row({ seq: 0 }), row({ seq: 1 })], nextSince: 2 }),
      );
      await connect();

      expect(rowElement(0).className).toContain("bus-monitor-row-new");
      expect(rowElement(1).className).toContain("bus-monitor-row-new");

      apiMock.pollBusTelegrams.mockResolvedValue(
        telegramsResponse({ telegrams: [row({ seq: 2 })], nextSince: 3 }),
      );
      await act(async () => {
        await vi.advanceTimersByTimeAsync(POLL_INTERVAL_MS);
      });

      // The new batch carries the marker...
      expect(rowElement(2).className).toContain("bus-monitor-row-new");
      // ...and the previous poll's rows no longer do — the marker does not
      // accumulate across polls.
      expect(rowElement(0).className ?? "").not.toContain("bus-monitor-row-new");
      expect(rowElement(1).className ?? "").not.toContain("bus-monitor-row-new");
    });

    it("clears the marker on a poll that returns no telegrams, rather than leaving the previous batch marked forever", async () => {
      await renderPanel();
      await flushReattach();
      apiMock.pollBusTelegrams.mockResolvedValue(
        telegramsResponse({ telegrams: [row({ seq: 0 })], nextSince: 1 }),
      );
      await connect();
      expect(rowElement(0).className).toContain("bus-monitor-row-new");

      apiMock.pollBusTelegrams.mockResolvedValue(telegramsResponse({ telegrams: [], nextSince: 1 }));
      await act(async () => {
        await vi.advanceTimersByTimeAsync(POLL_INTERVAL_MS);
      });

      expect(rowElement(0).className ?? "").not.toContain("bus-monitor-row-new");
    });

    it("does not mark rows adopted by the mount-time reattach's bulk load", async () => {
      apiMock.pollBusTelegrams.mockReset();
      apiMock.pollBusTelegrams.mockResolvedValueOnce(
        telegramsResponse({ sessionId: 5, nextSince: 1, telegrams: [row({ seq: 0 })] }),
      );
      apiMock.pollBusTelegrams.mockResolvedValue(telegramsResponse({ sessionId: 5, nextSince: 1 }));

      await renderPanel();
      await flushReattach();

      expect(rowElement(0).className ?? "").not.toContain("bus-monitor-row-new");
    });

    it("gives a SessionClosed row that just arrived both its existing marker class and the new-row class", async () => {
      await renderPanel();
      await flushReattach();
      apiMock.pollBusTelegrams.mockResolvedValue(
        telegramsResponse({
          telegrams: [row({ seq: 0, service: "SessionClosed", decoded: null, rawPayload: null })],
          nextSince: 1,
          status: "closed",
        }),
      );
      await connect();

      const tr = rowElement(0);
      expect(tr.className).toContain("bus-monitor-row-marker");
      expect(tr.className).toContain("bus-monitor-row-new");
    });
  });

  it("surfaces a stop-time drain-task warning instead of swallowing it", async () => {
    apiMock.stopBusMonitor.mockResolvedValue({
      sessionId: 1,
      serverIncarnation: "process-a",
      telegramCount: 12,
      droppedCount: 0,
      warning: "drain task panicked during teardown",
    } satisfies BusMonitorStopResponse);

    // The default poll mock rejects with a `404`, which Task 4 turned from
    // an inert poll error into "the session ended elsewhere" — it now
    // detaches and shows the Connect form, so there would be no Disconnect
    // button left to click. This test is about the stop response, not
    // about the poll, so it gets a session that is still alive: a `404`
    // for the mount reattach, telegrams afterwards.
    apiMock.pollBusTelegrams.mockRejectedValueOnce(notFoundError());
    apiMock.pollBusTelegrams.mockResolvedValue(telegramsResponse({ nextSince: 1 }));

    await renderPanel();
    await flushReattach();
    await connect();
    await act(async () => {
      clickButton("Disconnect");
      await vi.advanceTimersByTimeAsync(0);
    });

    expect(host!.textContent).toContain("drain task panicked during teardown");
    expect(host!.querySelector(".bus-monitor-warning")).not.toBeNull();
  });
});

// Task 4's stale lock, from the panel's side. `busContext.test.ts` proves
// the rules in isolation; these prove the panel actually asks, and acts on
// the answer, at the three moments that matter — a project edited under a
// running session, a session it did not start, and a session replaced or
// ended by somebody else.
describe("BusMonitorPanel and the shared session's context", () => {
  function projectTree(name: string): ProjectTree {
    return {
      schema_version: 3,
      errors: 0,
      warnings: 0,
      can_undo: false,
      can_redo: false,
      is_modified: false,
      server_incarnation: "process-a",
      snapshot_revision: 1,
      group_address_style: "ThreeLevel",
      installations: [
        {
          id: 1,
          name: "Installation",
          topology: [],
          buildings: [],
          unassigned: [],
          group_addresses: [
            { id: 1, name, address: "1/2/3", range: null, dpts: ["DPST-1-1"], links: [] },
          ],
          group_ranges: [],
        },
      ],
    };
  }

  // Advance one poll interval and drain the reply. The lock is re-read on
  // every tick precisely so it does not depend on a cross-window `storage`
  // event arriving, which is what this waits for.
  async function tick() {
    await act(async () => {
      await vi.advanceTimersByTimeAsync(POLL_INTERVAL_MS);
    });
  }

  describe("pause without losing the server cursor", () => {
    it("does not overlap slow polls or duplicate rows while the server is still answering", async () => {
      apiMock.pollBusTelegrams.mockRejectedValueOnce(notFoundError());
      let finishPoll!: (response: BusMonitorTelegramsResponse) => void;
      apiMock.pollBusTelegrams.mockImplementationOnce(() => new Promise((resolve) => { finishPoll = resolve; }));
      apiMock.pollBusTelegrams.mockResolvedValue(telegramsResponse({ nextSince: 1 }));
      await renderPanel();
      await flushReattach();
      await connect();
      await tick();
      await tick();
      expect(apiMock.pollBusTelegrams).toHaveBeenCalledTimes(2); // mount 404 and one outstanding poll
      await act(async () => finishPoll(telegramsResponse({ nextSince: 1, telegrams: [row({ seq: 0 })] })));
      expect(host!.querySelectorAll("tbody tr")).toHaveLength(1);
      await tick();
      expect(apiMock.pollBusTelegrams).toHaveBeenLastCalledWith(1);
      expect(host!.querySelectorAll("tbody tr")).toHaveLength(1);
    });

    it("pauses row polling but keeps context-only checks and resumes the held cursor", async () => {
      apiMock.pollBusTelegrams.mockRejectedValueOnce(notFoundError());
      apiMock.pollBusTelegrams.mockResolvedValue(telegramsResponse({
        nextSince: 1, telegrams: [row({ seq: 0 })],
      }));
      await renderPanel();
      await flushReattach();
      await connect();
      expect(host!.querySelectorAll("tbody tr")).toHaveLength(1);
      const count = apiMock.pollBusTelegrams.mock.calls.length;

      await act(async () => clickButton("Pause"));
      expect(host!.querySelector(".bus-monitor-paused")?.textContent).toContain("server's finite buffer");
      await tick();
      await tick();
      expect(apiMock.pollBusTelegrams.mock.calls.slice(count)).toEqual([[1, true], [1, true], [1, true]]);
      expect(host!.querySelectorAll("tbody tr")).toHaveLength(1);
      expect(apiMock.stopBusMonitor).not.toHaveBeenCalled();

      apiMock.pollBusTelegrams.mockResolvedValueOnce(telegramsResponse({
        nextSince: 5,
        droppedBefore: 3,
        telegrams: [row({ seq: 4, destination: "1/2/4" })],
      }));
      await act(async () => {
        clickButton("Resume");
        await vi.advanceTimersByTimeAsync(0);
      });
      expect(apiMock.pollBusTelegrams).toHaveBeenLastCalledWith(1);
      expect(host!.querySelectorAll("tbody tr")).toHaveLength(2);
      expect(host!.querySelector(".bus-monitor-gap-notice")!.textContent).toContain("3");
    });

    it("ignores an in-flight reply after pausing, then sees a gateway-close marker on resume", async () => {
      apiMock.pollBusTelegrams.mockRejectedValueOnce(notFoundError());
      apiMock.pollBusTelegrams.mockResolvedValue(telegramsResponse({ nextSince: 1, telegrams: [row({ seq: 0 })] }));
      await renderPanel();
      await flushReattach();
      await connect();
      let resolvePoll!: (response: BusMonitorTelegramsResponse) => void;
      apiMock.pollBusTelegrams.mockImplementationOnce(() => new Promise((resolve) => { resolvePoll = resolve; }));
      await tick();
      await act(async () => clickButton("Pause"));
      await act(async () => resolvePoll(telegramsResponse({ nextSince: 2, telegrams: [row({ seq: 1 })] })));
      expect(host!.querySelectorAll("tbody tr")).toHaveLength(1);
      apiMock.pollBusTelegrams.mockResolvedValueOnce(telegramsResponse({
        nextSince: 3, status: "closed", telegrams: [row({ seq: 1 }), row({ seq: 2, service: "SessionClosed", decoded: null })],
      }));
      await act(async () => { clickButton("Resume"); await vi.advanceTimersByTimeAsync(0); });
      expect(apiMock.pollBusTelegrams).toHaveBeenLastCalledWith(1);
      expect(host!.querySelectorAll("tbody tr")).toHaveLength(3);
      expect(host!.querySelector(".bus-monitor-session")!.textContent).toContain("closed by gateway");
    });

    it("disconnects while paused and starts a later session from a fresh cursor", async () => {
      apiMock.pollBusTelegrams.mockRejectedValueOnce(notFoundError());
      apiMock.pollBusTelegrams.mockResolvedValue(telegramsResponse({ nextSince: 1, telegrams: [row({ seq: 0 })] }));
      await renderPanel();
      await flushReattach();
      await connect();
      await act(async () => clickButton("Pause"));
      await tick();
      const pollsWhilePaused = apiMock.pollBusTelegrams.mock.calls.length;
      apiMock.pollBusTelegrams.mockRejectedValue(notFoundError());
      await act(async () => { clickButton("Disconnect"); await vi.advanceTimersByTimeAsync(0); });
      expect(apiMock.stopBusMonitor).toHaveBeenCalledTimes(1);
      await tick();
      expect(apiMock.pollBusTelegrams.mock.calls.length).toBe(pollsWhilePaused + 1); // reattach check only
      apiMock.startBusMonitor.mockResolvedValueOnce({ sessionId: 2, serverIncarnation: "process-a", assignedAddress: "1.1.5" });
      apiMock.pollBusTelegrams.mockResolvedValue(telegramsResponse({ sessionId: 2, nextSince: 0 }));
      await connect();
      expect(host!.querySelector(".bus-monitor-pause")!.textContent).toBe("Pause");
      expect(apiMock.pollBusTelegrams).toHaveBeenLastCalledWith(0);
      expect(host!.querySelectorAll("tbody tr")).toHaveLength(0);
    });
  });

  it("bounds a large captured batch and its statistics without stalling the next server cursor", async () => {
    apiMock.pollBusTelegrams.mockRejectedValueOnce(notFoundError());
    const batch = Array.from({ length: CAPTURE_CAPACITY * 5 }, (_, seq) => row({
      seq, destination: `1/${seq % 20}/${seq % 200}`, source: `1.1.${seq % 100}`,
      service: `Service-${seq % 80}`,
    }));
    apiMock.pollBusTelegrams.mockResolvedValueOnce(telegramsResponse({ nextSince: batch.length, telegrams: batch }));
    apiMock.pollBusTelegrams.mockResolvedValue(telegramsResponse({ nextSince: batch.length }));
    await renderPanel();
    await flushReattach();
    await connect();
    expect(host!.querySelectorAll(".bus-monitor-table tbody tr")).toHaveLength(CAPTURE_CAPACITY);
    expect(host!.querySelector(".bus-monitor-client-pruned")!.textContent).toContain(String(batch.length - CAPTURE_CAPACITY));
    expect(host!.querySelectorAll(".bus-monitor-stats-services li").length).toBeLessThanOrEqual(10);
    expect(host!.querySelectorAll(".bus-monitor-stats-destinations li").length).toBeLessThanOrEqual(10);
    expect(host!.querySelectorAll(".bus-monitor-stats-sources li").length).toBeLessThanOrEqual(10);
    await tick();
    expect(apiMock.pollBusTelegrams).toHaveBeenLastCalledWith(batch.length);
  });

  it("locks the table and the compose form when the project changes under a running session", async () => {
    publishProjectContext(projectTree("Kitchen ceiling"));
    apiMock.pollBusTelegrams.mockRejectedValueOnce(notFoundError());
    apiMock.pollBusTelegrams.mockResolvedValue(
      telegramsResponse({ telegrams: [row({ seq: 0 })], nextSince: 1 }),
    );
    await renderPanel();
    await flushReattach();
    await connect();

    expect(host!.querySelector(".bus-monitor-stale-lock")).toBeNull();

    apiMock.pollBusTelegrams.mockResolvedValue(telegramsResponse({ contextStatus: "stale" }));
    publishProjectContext(projectTree("Kitchen ceiling, renamed mid-session"));
    await tick();

    const notice = host!.querySelector(".bus-monitor-stale-lock")!;
    expect(notice.getAttribute("role")).toBe("alert");
    expect(notice.textContent).toContain("sending is locked");
    // The rows are kept — they were decoded correctly when they arrived —
    // but marked as belonging to a snapshot the project has moved past.
    expect(host!.querySelector(".bus-monitor-table")!.className).toContain("bus-monitor-stale-table");
    expect(host!.querySelector<HTMLInputElement>(".bus-compose-value")!.disabled).toBe(true);
  });

  it("locks from the actual server context when no browser-profile record changes", async () => {
    publishProjectContext(projectTree("Unchanged browser record"));
    recordSessionContext(5, "process-a");
    apiMock.pollBusTelegrams.mockResolvedValue(telegramsResponse({
      sessionId: 5, ...{ contextStatus: "stale", projectOpen: true },
    }));
    await renderPanel();
    await flushReattach();
    expect(host!.querySelector(".bus-monitor-stale-lock")).not.toBeNull();
    expect(host!.querySelector<HTMLInputElement>(".bus-compose-value")!.disabled).toBe(true);
    expect(apiMock.writeBusValue).not.toHaveBeenCalled();
  });

  it.each(["unavailable", "future-status", undefined])("does not let a local record verify an %s server comparison", async (contextStatus) => {
    publishProjectContext(projectTree("Local record claims freshness"));
    recordSessionContext(5, "process-a");
    apiMock.pollBusTelegrams.mockResolvedValue(telegramsResponse({
      sessionId: 5, contextStatus: contextStatus as BusMonitorTelegramsResponse["contextStatus"],
      projectOpen: null,
    }));
    await renderPanel();
    await flushReattach();
    expect(host!.querySelector(".bus-monitor-unverified-lock")).not.toBeNull();
    expect(host!.querySelector<HTMLInputElement>(".bus-compose-value")!.disabled).toBe(true);
    expect(apiMock.writeBusValue).not.toHaveBeenCalled();
  });

  it("uses the actual absence of a server project rather than a persisted browser tree", async () => {
    publishProjectContext(projectTree("Persisted tree from before server restart"));
    apiMock.pollBusTelegrams.mockResolvedValue(telegramsResponse({ projectOpen: false }));
    await renderPanel(true);
    await flushReattach();
    expect(host!.querySelector(".bus-compose-form")!.textContent).toContain("No project open");
    expect(host!.querySelector(".bus-monitor-unverified-lock")).toBeNull();
  });

  it("keeps context freshness polling while rows and cursor are paused", async () => {
    apiMock.pollBusTelegrams.mockResolvedValue(telegramsResponse({ telegrams: [row({seq: 0})], nextSince: 1 }));
    await renderPanel();
    await flushReattach();
    await act(async () => clickButton("Pause"));
    apiMock.pollBusTelegrams.mockResolvedValue(telegramsResponse({ contextStatus: "stale", nextSince: 999, telegrams: [row({seq: 998})] }));
    await tick();
    expect(apiMock.pollBusTelegrams).toHaveBeenLastCalledWith(1, true);
    expect(host!.querySelectorAll("tbody tr")).toHaveLength(1);
    expect(host!.querySelector(".bus-monitor-stale-lock")).not.toBeNull();
    expect(host!.querySelector<HTMLInputElement>(".bus-compose-value")!.disabled).toBe(true);
    await act(async () => { clickButton("Resume"); await vi.advanceTimersByTimeAsync(0); });
    expect(apiMock.pollBusTelegrams).toHaveBeenLastCalledWith(1);
  });

  it.each([
    {serverIncarnation: ""}, {sessionId: 1.5}, {projectOpen: null},
  ])("does not accept malformed current-context evidence %j", async malformed => {
    apiMock.pollBusTelegrams.mockResolvedValue(telegramsResponse(malformed));
    await renderPanel();
    await flushReattach();
    expect(host!.querySelector(".bus-monitor-unverified-lock")).not.toBeNull();
    expect(host!.querySelector<HTMLInputElement>(".bus-compose-value")!.disabled).toBe(true);
  });

  it("does not let a late current reply erase an observed context change", async () => {
    publishProjectContext(projectTree("Before edit"));
    recordSessionContext(5, "process-a");
    apiMock.pollBusTelegrams.mockResolvedValue(telegramsResponse({sessionId: 5}));
    await renderPanel();
    await flushReattach();
    let finish!: (response: BusMonitorTelegramsResponse) => void;
    apiMock.pollBusTelegrams.mockImplementationOnce(() => new Promise(resolve => {finish = resolve;}));
    await tick();
    await act(async () => publishProjectContext(projectTree("Changed while reply was pending")));
    await act(async () => finish(telegramsResponse({sessionId: 5})));
    expect(host!.querySelector<HTMLInputElement>(".bus-compose-value")!.disabled).toBe(true);
    expect(host!.querySelector(".bus-monitor-stale-lock")).not.toBeNull();
    await tick();
    expect(host!.querySelector<HTMLInputElement>(".bus-compose-value")!.disabled).toBe(false);
  });

  it("ignores an obsolete reattach rejection after a newer connection succeeded", async () => {
    let reject!: (reason: Error) => void;
    apiMock.pollBusTelegrams.mockImplementationOnce(() => new Promise((_, failure) => { reject = failure; }));
    await renderPanel();
    apiMock.pollBusTelegrams.mockResolvedValue(telegramsResponse({sessionId: 1}));
    await connect();
    await act(async () => reject(new Error("obsolete reattach error")));
    expect(host!.textContent).not.toContain("obsolete reattach error");
    expect(host!.querySelector(".bus-monitor-session")).not.toBeNull();
  });

  it("does not keep a previous current comparison after polling fails", async () => {
    apiMock.pollBusTelegrams.mockResolvedValue(telegramsResponse({sessionId: 5}));
    await renderPanel();
    await flushReattach();
    expect(host!.querySelector<HTMLInputElement>(".bus-compose-value")!.disabled).toBe(false);
    apiMock.pollBusTelegrams.mockRejectedValue(new Error("comparison unavailable"));
    await tick();
    expect(host!.querySelector(".bus-monitor-unverified-lock")).not.toBeNull();
    expect(host!.querySelector<HTMLInputElement>(".bus-compose-value")!.disabled).toBe(true);
  });

  it("reports a legacy server comparison as unverified rather than fresh or stale", async () => {
    publishProjectContext(projectTree("Kitchen ceiling"));
    apiMock.pollBusTelegrams.mockResolvedValue(telegramsResponse({ sessionId: 5, nextSince: 1, contextStatus: undefined, projectOpen: undefined }));
    await renderPanel();
    await flushReattach();

    const notice = host!.querySelector(".bus-monitor-unverified-lock")!;
    expect(notice.getAttribute("role")).toBe("note");
    expect(notice.textContent).toContain("cannot verify");
    // Unknown is not proof of staleness or permission to resolve a write DPT.
    expect(host!.querySelector(".bus-monitor-stale-lock")).toBeNull();
    expect(host!.querySelector<HTMLInputElement>(".bus-compose-value")!.disabled).toBe(true);
  });

  it("stays quiet when another window recorded the very session it attached to", async () => {
    publishProjectContext(projectTree("Kitchen ceiling"));
    recordSessionContext(5, "process-a"); // what the other window wrote when it connected
    apiMock.pollBusTelegrams.mockResolvedValue(telegramsResponse({ sessionId: 5, nextSince: 1 }));
    await renderPanel();
    await flushReattach();

    expect(host!.querySelector(".bus-monitor-unverified-lock")).toBeNull();
    expect(host!.querySelector(".bus-monitor-stale-lock")).toBeNull();
  });

  it("does not verify a reused numeric session id from a restarted server", async () => {
    publishProjectContext({
      ...projectTree("Kitchen ceiling"),
      server_incarnation: "process-b",
    });
    recordSessionContext(5, "process-a");
    apiMock.pollBusTelegrams.mockResolvedValue(telegramsResponse({
      sessionId: 5,
      serverIncarnation: "process-b",
      nextSince: 1,
      contextStatus: undefined,
      projectOpen: undefined,
    }));
    await renderPanel();
    await flushReattach();

    expect(host!.querySelector(".bus-monitor-unverified-lock")).not.toBeNull();
    expect(host!.querySelector(".bus-monitor-stale-lock")).toBeNull();
  });

  it("drops the old rows and announces the new identity when the session is replaced", async () => {
    apiMock.pollBusTelegrams.mockRejectedValueOnce(notFoundError());
    apiMock.pollBusTelegrams.mockResolvedValue(
      telegramsResponse({ sessionId: 1, telegrams: [row({ seq: 0 })], nextSince: 1 }),
    );
    await renderPanel();
    await flushReattach();
    await connect();
    expect(host!.querySelectorAll("tbody tr")).toHaveLength(1);

    apiMock.pollBusTelegrams.mockResolvedValue(
      telegramsResponse({ sessionId: 2, telegrams: [], nextSince: 0 }),
    );
    await tick();

    const notice = host!.querySelector(".bus-monitor-replaced-notice")!;
    expect(notice.getAttribute("role")).toBe("alert");
    expect(notice.textContent).toContain("session 2");
    expect(host!.querySelectorAll("tbody tr")).toHaveLength(0);
  });

  it("treats the same numeric id from a new server process as a replacement", async () => {
    apiMock.pollBusTelegrams.mockRejectedValueOnce(notFoundError());
    apiMock.pollBusTelegrams.mockResolvedValue(
      telegramsResponse({ sessionId: 1, telegrams: [row({ seq: 0 })], nextSince: 1 }),
    );
    await renderPanel();
    await flushReattach();
    await connect();
    expect(host!.querySelectorAll("tbody tr")).toHaveLength(1);

    apiMock.pollBusTelegrams.mockResolvedValue(telegramsResponse({
      sessionId: 1,
      serverIncarnation: "process-b",
      telegrams: [],
      nextSince: 0,
      contextStatus: "unavailable",
      projectOpen: null,
    }));
    await tick();

    expect(host!.querySelectorAll("tbody tr")).toHaveLength(0);
    expect(host!.querySelector(".bus-monitor-replaced-notice")).not.toBeNull();
    expect(host!.querySelector(".bus-monitor-unverified-lock")).not.toBeNull();
  });

  it("returns to the Connect form and says the session ended elsewhere on a mid-session 404", async () => {
    apiMock.pollBusTelegrams.mockRejectedValueOnce(notFoundError());
    apiMock.pollBusTelegrams.mockResolvedValue(telegramsResponse({ nextSince: 1 }));
    await renderPanel();
    await flushReattach();
    await connect();

    apiMock.pollBusTelegrams.mockRejectedValue(notFoundError());
    await tick();

    const notice = host!.querySelector(".bus-monitor-ended-elsewhere")!;
    expect(notice.getAttribute("role")).toBe("alert");
    expect(notice.textContent).toContain("ended elsewhere");
    expect(host!.querySelector(".bus-monitor-connect")).not.toBeNull();
    // And it did not try to clean up a session that is already gone.
    expect(apiMock.stopBusMonitor).not.toHaveBeenCalled();
  });
  // T25: the interface search. Every address here is RFC 5737
  // documentation space, and the search itself is the one KNXnet/IP
  // exchange that writes nothing to any device — these tests talk to a
  // mocked `POST /api/bus/discover` and to nothing else.
  describe("interface discovery", () => {
    const hallway: BusDiscoveredInterface = {
      controlEndpoint: "192.0.2.11:3671",
      individualAddress: "1.1.0",
      friendlyName: "Hallway interface",
      supportsTunnelling: true,
    };
    const workshop: BusDiscoveredInterface = {
      controlEndpoint: "192.0.2.12:3671",
      individualAddress: "1.1.1",
      friendlyName: "Workshop router",
      supportsTunnelling: false,
    };

    function options() {
      return Array.from(host!.querySelectorAll<HTMLButtonElement>(".bus-discovery-option"));
    }

    function searchButton() {
      return host!.querySelector<HTMLButtonElement>(".bus-discovery-search")!;
    }

    function gatewayField() {
      return host!.querySelector<HTMLInputElement>(".bus-monitor-host")!;
    }

    function portField() {
      return host!.querySelector<HTMLInputElement>(".bus-monitor-port")!;
    }

    async function click(button: HTMLButtonElement) {
      await act(async () => {
        button.dispatchEvent(new MouseEvent("click", { bubbles: true }));
        await vi.advanceTimersByTimeAsync(0);
      });
    }

    it("shows retained discovery octets without inventing a programming-mode verdict", async () => {
      apiMock.discoverBusInterfaces.mockResolvedValue({ interfaces: [{
        ...hallway,
        deviceInfo: { medium: 129, status: 128, projectInstallationId: 4660,
          serialNumber: [1, 2, 3, 4, 5, 6], routingMulticast: "224.0.23.12",
          macAddress: [170, 187, 204, 221, 238, 255] },
      }] });
      await renderPanel();
      await flushReattach();
      const info = host!.querySelector(".bus-discovery-info")!;
      expect(info).not.toBeNull();
      expect(Object.fromEntries(Array.from(info.querySelectorAll("dt"), (term) =>
        [term.textContent, term.nextElementSibling?.textContent]))).toEqual({
          "KNX medium (raw)": "0x81", "Device status (raw)": "0x80",
          "Project-installation identifier": "4660", "KNX serial number": "010203040506",
          "Routing multicast address": "224.0.23.12", "MAC address": "aa:bb:cc:dd:ee:ff",
        });
      expect(info.textContent).not.toContain("Programming mode");
      expect(apiMock.startBusMonitor).not.toHaveBeenCalled();
    });

    it("searches without being asked and offers what answered", async () => {
      apiMock.discoverBusInterfaces.mockResolvedValue({ interfaces: [hallway, workshop] });
      await renderPanel();
      await flushReattach();

      expect(apiMock.discoverBusInterfaces).toHaveBeenCalledTimes(1);
      const found = options();
      expect(found).toHaveLength(2);
      // The friendly name, not just the address — the whole point of
      // asking the interfaces who they are.
      expect(found[0]!.textContent).toContain("Hallway interface");
      expect(found[0]!.textContent).toContain("192.0.2.11:3671");
      expect(found[0]!.textContent).toContain("1.1.0");
      expect(found[0]!.textContent).toContain("Tunnelling");
      expect(found[1]!.textContent).toContain("Workshop router");
      expect(found[1]!.textContent).not.toContain("Tunnelling");
      expect(host!.querySelectorAll(".bus-discovery-info-unavailable")).toHaveLength(2);
      expect(host!.querySelector(".bus-discovery-info-unavailable")!.textContent).toContain("not provided");
      expect(host!.querySelector(".bus-discovery-status")!.textContent).toBe("2 interfaces answered.");
      // An offer, not a decision: nothing connected, nothing typed.
      expect(apiMock.startBusMonitor).not.toHaveBeenCalled();
      expect(gatewayField().value).toBe("");
    });

    it("says nothing answered without turning that into a failure", async () => {
      await renderPanel();
      await flushReattach();

      expect(options()).toHaveLength(0);
      expect(host!.querySelector(".bus-discovery-status")!.textContent).toBe("No interfaces answered.");
      // The reason it might be empty, in the CLI hint's own words.
      expect(host!.querySelector(".bus-discovery-hint")!.textContent).toContain("host networking");
      // …and the measured host-side cause: a firewall dropping the answer.
      expect(host!.querySelector(".bus-discovery-hint")!.textContent).toContain("firewall");
      // And the field the user actually needs is untouched.
      const input = gatewayField();
      expect(input.disabled).toBe(false);
      await act(async () => {
        setInputValue(".bus-monitor-host", "192.0.2.50");
      });
      expect(gatewayField().value).toBe("192.0.2.50");
      expect(portField().value).toBe("3671");
      const connectButton = Array.from(host!.querySelectorAll("button")).find(
        (b) => b.textContent === "Connect",
      )!;
      expect(connectButton.disabled).toBe(false);
    });

    it("disables the search button while a search is pending", async () => {
      await renderPanel();
      await flushReattach();
      expect(searchButton().disabled).toBe(false);

      let release: (response: BusDiscoverResponse) => void = () => {};
      apiMock.discoverBusInterfaces.mockReturnValueOnce(
        new Promise<BusDiscoverResponse>((resolve) => {
          release = resolve;
        }),
      );
      await click(searchButton());

      expect(searchButton().disabled).toBe(true);
      expect(searchButton().getAttribute("aria-busy")).toBe("true");
      expect(searchButton().textContent).toBe("Searching…");
      // A second click during the fixed search window buys a second
      // timeout, not a faster answer — so it buys nothing at all.
      await click(searchButton());
      expect(apiMock.discoverBusInterfaces).toHaveBeenCalledTimes(2);

      await act(async () => {
        release({ interfaces: [hallway] });
        await vi.advanceTimersByTimeAsync(0);
      });
      expect(searchButton().disabled).toBe(false);
      expect(options()).toHaveLength(1);
    });

    it("keeps a failed search out of the user's way", async () => {
      apiMock.discoverBusInterfaces.mockRejectedValue(new Error("multicast went nowhere"));
      await renderPanel();
      await flushReattach();

      // No error wall: no alert, no field error, nothing blocking.
      expect(host!.querySelector("[role='alert']")).toBeNull();
      expect(host!.querySelector(".field-error")).toBeNull();
      const status = host!.querySelector(".bus-discovery-status")!;
      expect(status.getAttribute("role")).toBe("status");
      expect(status.textContent).toContain("could not be run");
      // The server's own words survive, quietly.
      expect(host!.querySelector(".bus-discovery-detail")!.textContent).toBe("multicast went nowhere");
      expect(gatewayField().disabled).toBe(false);
    });

    it("fills the gateway field from a found interface and keeps taking free text", async () => {
      apiMock.discoverBusInterfaces.mockResolvedValue({ interfaces: [hallway, workshop] });
      await renderPanel();
      await flushReattach();

      await click(options()[1]!);
      expect(gatewayField().value).toBe("192.0.2.12");
      expect(portField().value).toBe("3671");
      // Selecting fills the field. It does not connect.
      expect(apiMock.startBusMonitor).not.toHaveBeenCalled();

      // And the field is still a field: an address nothing answered from
      // is still allowed, because multicast not reaching a subnet says
      // nothing about whether a unicast address works.
      await act(async () => {
        setInputValue(".bus-monitor-host", "203.0.113.7");
      });
      expect(gatewayField().value).toBe("203.0.113.7");
    });
  });
});

// U20: the telegram-flow view is fed by this panel's own poll loop.
describe("BusMonitorPanel telegram flow", () => {
  const LIGHT = 0x0801;

  function flowRow(seq: number, overrides: Partial<BusTelegramRow> = {}): BusTelegramRow {
    return row({
      seq, source: "1.1.1", destination: "1/0/1", decoded: { kind: "value", dpt: "DPST-1-1", text: "On" },
      sourceRaw: 0x1101, destinationRaw: LIGHT, observedAgeMs: 0, flowGeneration: "1", ...overrides,
    });
  }

  function flowSnapshot() {
    return parseFlowSnapshot(snapshotJson({
      serverIncarnation: "process-a", sessionId: 1,
      devices: [
        { deviceId: 1, installationId: 1, name: "Switch", individualAddressRaw: 0x1101 },
        { deviceId: 2, installationId: 1, name: "Dimmer", individualAddressRaw: 0x1102 },
      ],
      groups: [{
        gaRaw: LIGHT, gaId: 10, installationId: 1, name: "Light", dpt: "1.001",
        members: [1, 2].map((deviceId) => ({
          deviceId, comObjectId: deviceId * 100, direction: deviceId === 1 ? "Send" : "Receive", active: true,
          flags: { communication: true, read: null, write: true, transmit: null, update: null, readOnInit: null },
        })),
      }],
    }));
  }

  beforeEach(() => {
    // The flow clock is `performance.now()`; fake it with the timers.
    vi.useRealTimers();
    vi.useFakeTimers({ toFake: ["setTimeout", "clearTimeout", "setInterval", "clearInterval", "Date", "performance"] });
    apiMock.fetchFlowSnapshot.mockResolvedValue(flowSnapshot());
  });

  const tab = (label: string) => Array.from(host!.querySelectorAll<HTMLButtonElement>('[role="tab"]')).find((b) => b.textContent === label)!;
  const flowNode = (label: string) => host!.querySelector<SVGGElement>(`g.flow-node[aria-label^="${label}."]`);
  const tick = (ms = POLL_INTERVAL_MS) => act(async () => { await vi.advanceTimersByTimeAsync(ms); });

  // Answers by argument, not by call order: panels other tests left mounted
  // also react to `connect()`'s context publication and poll, and a queue
  // of one-shot answers would hand them this test's telegrams.
  const server = { connected: false, sessionId: 1, rows: [] as BusTelegramRow[] };

  async function connectWith(first: BusTelegramRow[]) {
    Object.assign(server, { connected: false, sessionId: 1, rows: first });
    apiMock.startBusMonitor.mockImplementation(async () => {
      server.connected = true;
      return { sessionId: server.sessionId, serverIncarnation: "process-a", assignedAddress: "1.1.250" };
    });
    apiMock.pollBusTelegrams.mockReset();
    apiMock.pollBusTelegrams.mockImplementation(async (since: number) => {
      if (!server.connected) throw notFoundError();
      const telegrams = server.rows.filter((r) => r.seq >= since);
      const nextSince = Math.max(since, ...server.rows.map((r) => r.seq + 1));
      return telegramsResponse({ sessionId: server.sessionId, telegrams, nextSince, flowGeneration: "1" });
    });
    const root = await renderPanel();
    await flushReattach();
    await connect();
    await tick(0);
    return root;
  }

  it("draws the polled telegrams without a second poll loop and fetches each generation once", async () => {
    const root = await connectWith([flowRow(1)]);
    // The same two intervals with the table and then with the flow view:
    // the flow view adds no request of its own. (Panels other tests left
    // mounted poll too, so only the difference between the windows counts.)
    const polls = () => apiMock.pollBusTelegrams.mock.calls.length;
    let start = polls();
    await tick();
    await tick();
    const withTable = polls() - start;
    await act(async () => tab("Flow").click());
    expect(tab("Flow").getAttribute("aria-selected")).toBe("true");
    expect(flowNode("Switch")).not.toBeNull();
    expect(flowNode("Dimmer")!.querySelector(".flow-badge-inferred")!.textContent).toBe("◇ 1/0/1 On");
    start = polls();
    await tick();
    await tick();
    expect(polls() - start).toBe(withTable);
    expect(withTable).toBeGreaterThan(0);
    // Fetched for (session 1, generation 1), and never again while the
    // generation stays the same (once per feed: `flowFeed.test.tsx`).
    expect(apiMock.fetchFlowSnapshot).toHaveBeenCalledWith(1, "1");
    const fetches = apiMock.fetchFlowSnapshot.mock.calls.length;
    await tick();
    await tick();
    expect(apiMock.fetchFlowSnapshot.mock.calls.length).toBe(fetches);
    expect(host!.querySelector(".bus-monitor-table")).toBeNull();
    expect(host!.querySelector('input[aria-label="Filter by destination or name"]')).toBeNull();
    await act(async () => tab("Telegrams").click());
    expect(host!.querySelector('input[aria-label="Filter by destination or name"]')).not.toBeNull();
    root.unmount();
  });

  it("keeps the map across tab changes and expires a value seven seconds after it was observed", async () => {
    const root = await connectWith([flowRow(1, { observedAgeMs: 500 })]);
    await act(async () => tab("Flow").click());
    await act(async () => tab("Telegrams").click());
    expect(host!.querySelector(".bus-monitor-table")).not.toBeNull();
    await act(async () => tab("Flow").click());
    expect(flowNode("Dimmer")!.querySelector(".flow-badge")).not.toBeNull();
    await tick(6_400);
    expect(flowNode("Dimmer")!.querySelector(".flow-badge")).not.toBeNull();
    await tick(200);
    expect(flowNode("Dimmer")!.querySelector(".flow-badge")).toBeNull();
    expect(flowNode("Dimmer")).not.toBeNull();
    root.unmount();
  });

  it("starts a new map when the server answers for another session", async () => {
    const root = await connectWith([flowRow(1)]);
    await act(async () => tab("Flow").click());
    expect(flowNode("Switch")).not.toBeNull();
    Object.assign(server, { sessionId: 2, rows: [] });
    await tick();
    expect(flowNode("Switch")).toBeNull();
    expect(host!.textContent).toContain("No group telegrams observed");
    root.unmount();
  });

  it("adopts a running session's backlog without reviving old values as current", async () => {
    Object.assign(server, { connected: true, sessionId: 1, rows: [flowRow(1, { observedAgeMs: 30_000 })] });
    apiMock.pollBusTelegrams.mockReset();
    apiMock.pollBusTelegrams.mockImplementation(async (since: number) => telegramsResponse({
      sessionId: 1, telegrams: server.rows.filter((r) => r.seq >= since), nextSince: 2, flowGeneration: "1",
    }));
    const root = await renderPanel();
    await flushReattach();
    await tick(0);
    await act(async () => tab("Flow").click());
    expect(flowNode("Switch")).not.toBeNull();
    expect(flowNode("Dimmer")).not.toBeNull();
    expect(host!.querySelector(".flow-badge")).toBeNull();
    root.unmount();
  });

  it("shows an empty map for a new session that has not delivered telegrams yet", async () => {
    const root = await connectWith([flowRow(1)]);
    await act(async () => tab("Flow").click());
    expect(flowNode("Switch")).not.toBeNull();
    apiMock.stopBusMonitor.mockImplementation(async () => {
      server.connected = false;
      return { sessionId: 1, serverIncarnation: "process-a", telegramCount: 1, droppedCount: 0 };
    });
    await act(async () => { clickButton("Disconnect"); await vi.advanceTimersByTimeAsync(0); });
    Object.assign(server, { sessionId: 2, rows: [] });
    await connect();
    await tick(0);
    await act(async () => tab("Flow").click());
    expect(flowNode("Switch")).toBeNull();
    expect(host!.textContent).toContain("No group telegrams observed");
    root.unmount();
  });

  it("keeps the monitor's loss notice in view while the flow is shown", async () => {
    const root = await connectWith([flowRow(1)]);
    await act(async () => tab("Flow").click());
    apiMock.pollBusTelegrams.mockImplementation(async (since: number) => telegramsResponse({
      sessionId: 1, telegrams: server.rows.filter((r) => r.seq >= since), nextSince: 2, droppedBefore: 5, flowGeneration: "1",
    }));
    await tick();
    expect(host!.querySelector(".bus-monitor-gap-notice")).not.toBeNull();
    expect(flowNode("Switch")).not.toBeNull();
    root.unmount();
  });

  it("switches views with the arrow keys", async () => {
    const root = await connectWith([flowRow(1)]);
    await act(async () => { tab("Telegrams").dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowRight", bubbles: true })); });
    expect(tab("Flow").getAttribute("aria-selected")).toBe("true");
    expect(document.activeElement).toBe(tab("Flow"));
    root.unmount();
  });
});
