// @vitest-environment happy-dom
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
import type { ProjectTree } from "./bindings/ProjectTree";

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
  // Cross-window context records outlive a component; without this a
  // session record written by one test would decide the next one's lock.
  window.localStorage.clear();
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
  // T25 default: a search that finds nothing. The common case on a
  // developer laptop, and the answer that keeps the connect form the
  // only thing every other test in this file has to look at.
  resetBusDiscoveryForTests();
  apiMock.discoverBusInterfaces.mockResolvedValue({ interfaces: [] });
  apiMock.stopBusMonitor.mockResolvedValue({
    sessionId: 1,
    telegramCount: 0,
    droppedCount: 0,
  } satisfies BusMonitorStopResponse);
});

afterEach(() => {
  resetBusDiscoveryForTests();
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

  it("says so, without claiming staleness, when it did not start the session it attached to", async () => {
    publishProjectContext(projectTree("Kitchen ceiling"));
    apiMock.pollBusTelegrams.mockResolvedValue(telegramsResponse({ sessionId: 5, nextSince: 1 }));
    await renderPanel();
    await flushReattach();

    const notice = host!.querySelector(".bus-monitor-unverified-lock")!;
    expect(notice.getAttribute("role")).toBe("note");
    expect(notice.textContent).toContain("did not start this session");
    // Unverified is not stale: sending stays possible, because nothing
    // observed says the snapshot is wrong — only that it is unconfirmed.
    expect(host!.querySelector(".bus-monitor-stale-lock")).toBeNull();
    expect(host!.querySelector<HTMLInputElement>(".bus-compose-value")!.disabled).toBe(false);
  });

  it("stays quiet when another window recorded the very session it attached to", async () => {
    publishProjectContext(projectTree("Kitchen ceiling"));
    recordSessionContext(5); // what the other window wrote when it connected
    apiMock.pollBusTelegrams.mockResolvedValue(telegramsResponse({ sessionId: 5, nextSince: 1 }));
    await renderPanel();
    await flushReattach();

    expect(host!.querySelector(".bus-monitor-unverified-lock")).toBeNull();
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
      return host!.querySelector<HTMLInputElement>(".bus-monitor-connect input")!;
    }

    async function click(button: HTMLButtonElement) {
      await act(async () => {
        button.dispatchEvent(new MouseEvent("click", { bubbles: true }));
        await vi.advanceTimersByTimeAsync(0);
      });
    }

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
      // And the field the user actually needs is untouched.
      const input = gatewayField();
      expect(input.disabled).toBe(false);
      await act(async () => {
        setInputValue(".bus-monitor-connect input", "192.0.2.50:3671");
      });
      expect(gatewayField().value).toBe("192.0.2.50:3671");
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
      expect(gatewayField().value).toBe("192.0.2.12:3671");
      // Selecting fills the field. It does not connect.
      expect(apiMock.startBusMonitor).not.toHaveBeenCalled();

      // And the field is still a field: an address nothing answered from
      // is still allowed, because multicast not reaching a subnet says
      // nothing about whether a unicast address works.
      await act(async () => {
        setInputValue(".bus-monitor-connect input", "203.0.113.7:3671");
      });
      expect(gatewayField().value).toBe("203.0.113.7:3671");
    });
  });
});
