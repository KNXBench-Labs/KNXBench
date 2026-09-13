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
  writeBusValue: vi.fn(),
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
  // Default: no session exists yet when the panel mounts, same as every
  // test before the mount-reattach fix existed. Tests that want to
  // exercise reattaching to a pre-existing session override this
  // (`mockResolvedValueOnce`, so only the mount-time call is affected)
  // before calling `renderPanel()`.
  apiMock.pollBusTelegrams.mockRejectedValue(notFoundError());
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
      expect(host!.querySelector(".bus-monitor-connect button")!.textContent).toBe("Disconnect");

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
    expect(host!.textContent).toContain("1/2/3");
    expect(host!.textContent).not.toContain("4/5/6");
    expect(apiMock.pollBusTelegrams.mock.calls.length).toBe(pollCallsBefore);

    // Clearing it brings the other row back.
    await act(async () => {
      setInputValue(".bus-monitor-filters input[type=text]", "");
    });
    expect(host!.textContent).toContain("4/5/6");
    expect(apiMock.pollBusTelegrams.mock.calls.length).toBe(pollCallsBefore);

    // The service checkbox filter hides a row by service instead.
    const writeCheckbox = Array.from(host!.querySelectorAll<HTMLInputElement>(".bus-monitor-filter"))
      .find((label) => label.textContent === "GroupValueWrite")!
      .querySelector<HTMLInputElement>("input")!;
    await act(async () => {
      writeCheckbox.dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });
    expect(host!.textContent).not.toContain("1/2/3");
    expect(host!.textContent).toContain("4/5/6");
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
      telegramCount: 12,
      droppedCount: 0,
      warning: "drain task panicked during teardown",
    } satisfies BusMonitorStopResponse);

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
