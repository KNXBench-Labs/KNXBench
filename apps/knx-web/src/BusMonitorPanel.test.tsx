// @vitest-environment happy-dom
import { act } from "react";
import { createRoot } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type {
  BusMonitorStartResponse,
  BusMonitorStopResponse,
  BusMonitorTelegramsResponse,
  BusTelegramRow,
} from "./api";

const apiMock = vi.hoisted(() => ({
  startBusMonitor: vi.fn(),
  stopBusMonitor: vi.fn(),
  pollBusTelegrams: vi.fn(),
}));

vi.mock("./api", () => ({
  ...apiMock,
  errorMessage: (error: unknown) => (error instanceof Error ? error.message : String(error)),
}));

import BusMonitorPanel from "./BusMonitorPanel";

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

function telegramsResponse(overrides: Partial<BusMonitorTelegramsResponse>): BusMonitorTelegramsResponse {
  return {
    sessionId: 1,
    status: "active",
    nextSince: 1,
    droppedBefore: 0,
    telegrams: [],
    ...overrides,
  };
}

async function renderPanel() {
  host = document.createElement("div");
  document.body.appendChild(host);
  const root = createRoot(host);
  await act(async () => {
    root.render(<BusMonitorPanel />);
  });
  return root;
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
async function connect(gateway = "192.168.1.10:3671") {
  await act(async () => {
    setInputValue(".bus-monitor-connect input", gateway);
  });
  await act(async () => {
    clickButton("Connect");
    await vi.advanceTimersByTimeAsync(0);
  });
}

beforeEach(() => {
  vi.useFakeTimers();
  apiMock.startBusMonitor.mockResolvedValue({
    sessionId: 1,
    assignedAddress: "1.1.5",
  } satisfies BusMonitorStartResponse);
  apiMock.pollBusTelegrams.mockResolvedValue(telegramsResponse({}));
  apiMock.stopBusMonitor.mockResolvedValue({
    sessionId: 1,
    telegramCount: 0,
    droppedCount: 0,
  } satisfies BusMonitorStopResponse);
});

afterEach(() => {
  host?.remove();
  host = undefined;
  vi.clearAllMocks();
  vi.useRealTimers();
});

describe("BusMonitorPanel", () => {
  it("renders rows from a mocked poll", async () => {
    apiMock.pollBusTelegrams.mockResolvedValue(
      telegramsResponse({
        telegrams: [
          row({ seq: 0, destination: "1/2/3", destinationName: "Living room light" }),
          row({ seq: 1, destination: "4/5/6", service: "GroupValueRead", decoded: null, rawPayload: null }),
        ],
        nextSince: 2,
      }),
    );

    await renderPanel();
    await connect();

    expect(apiMock.pollBusTelegrams).toHaveBeenCalledTimes(1);
    expect(apiMock.pollBusTelegrams).toHaveBeenCalledWith(0);
    expect(host!.textContent).toContain("1/2/3");
    expect(host!.textContent).toContain("Living room light");
    expect(host!.textContent).toContain("4/5/6");
    expect(host!.querySelectorAll(".bus-monitor-table tbody tr")).toHaveLength(2);
  });

  it("filters rows client-side, without re-fetching", async () => {
    apiMock.pollBusTelegrams.mockResolvedValue(
      telegramsResponse({
        telegrams: [
          row({ seq: 0, destination: "1/2/3", destinationName: "Living room light", service: "GroupValueWrite" }),
          row({ seq: 1, destination: "4/5/6", destinationName: null, service: "GroupValueRead", decoded: null, rawPayload: null }),
        ],
        nextSince: 2,
      }),
    );

    await renderPanel();
    await connect();
    expect(apiMock.pollBusTelegrams).toHaveBeenCalledTimes(1);

    // Text filter narrows to the matching destination only.
    await act(async () => {
      setInputValue(".bus-monitor-filters input[type=text]", "1/2/3");
    });
    expect(host!.textContent).toContain("1/2/3");
    expect(host!.textContent).not.toContain("4/5/6");
    expect(apiMock.pollBusTelegrams).toHaveBeenCalledTimes(1);

    // Clearing it brings the other row back.
    await act(async () => {
      setInputValue(".bus-monitor-filters input[type=text]", "");
    });
    expect(host!.textContent).toContain("4/5/6");
    expect(apiMock.pollBusTelegrams).toHaveBeenCalledTimes(1);

    // The service checkbox filter hides a row by service instead.
    const writeCheckbox = Array.from(host!.querySelectorAll<HTMLInputElement>(".bus-monitor-filter"))
      .find((label) => label.textContent === "GroupValueWrite")!
      .querySelector<HTMLInputElement>("input")!;
    await act(async () => {
      writeCheckbox.dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });
    expect(host!.textContent).not.toContain("1/2/3");
    expect(host!.textContent).toContain("4/5/6");
    expect(apiMock.pollBusTelegrams).toHaveBeenCalledTimes(1);
  });

  it("shows the gap notice, with the current count, when droppedBefore grows", async () => {
    apiMock.pollBusTelegrams.mockResolvedValueOnce(telegramsResponse({ droppedBefore: 0, nextSince: 1 }));

    await renderPanel();
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

  it("renders the server's error text when start fails, and never polls", async () => {
    apiMock.startBusMonitor.mockRejectedValue(new Error("gateway refused the connection"));

    await renderPanel();
    await connect();

    expect(host!.querySelector(".field-error")!.textContent).toBe("gateway refused the connection");
    expect(apiMock.pollBusTelegrams).not.toHaveBeenCalled();
    expect(host!.querySelector(".bus-monitor-table")).toBeNull();
  });

  it("surfaces a stop-time drain-task warning instead of swallowing it", async () => {
    apiMock.stopBusMonitor.mockResolvedValue({
      sessionId: 1,
      telegramCount: 12,
      droppedCount: 0,
      warning: "drain task panicked during teardown",
    } satisfies BusMonitorStopResponse);

    await renderPanel();
    await connect();
    await act(async () => {
      clickButton("Disconnect");
      await vi.advanceTimersByTimeAsync(0);
    });

    expect(host!.textContent).toContain("drain task panicked during teardown");
    expect(host!.querySelector(".bus-monitor-warning")).not.toBeNull();
  });
});
