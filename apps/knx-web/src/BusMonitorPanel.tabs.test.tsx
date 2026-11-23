/** Runs the tabs checklist (WP-49.4a) against the real bus monitor view tablist. */
// @vitest-environment happy-dom
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, vi } from "vitest";
import type { BusMonitorTelegramsResponse, BusTelegramRow } from "./api";

const apiMock = vi.hoisted(() => ({
  startBusMonitor: vi.fn(), stopBusMonitor: vi.fn(), stopBoundBusMonitor: vi.fn(), pollBusTelegrams: vi.fn(),
  writeBusValue: vi.fn(), discoverBusInterfaces: vi.fn(), fetchFlowSnapshot: vi.fn(),
}));
vi.mock("./api", async (importOriginal) => ({
  ...apiMock,
  isBoundBusSession: (await importOriginal<typeof import("./api")>()).isBoundBusSession,
  errorMessage: (error: unknown) => (error instanceof Error ? error.message : String(error)),
  errorStatus: (error: unknown) => (error && typeof error === "object" && "status" in error ? (error as { status: number }).status : undefined),
}));

import BusMonitorPanel from "./BusMonitorPanel";
import { resetBusDiscoveryForTests } from "./busDiscovery";
import { resetBusConnectionForTests } from "./busConnection";
import { resetSettingsForTests } from "./settingsStore";
import { describeTabsChecklist, type TabsAdapter } from "./test/tabsChecklist";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const telegram: BusTelegramRow = {
  seq: 0, timestamp: "2026-09-11T12:00:00Z", source: "1.1.5", destination: "1/2/3", destinationName: null,
  service: "GroupValueWrite", rawPayload: "0x01 (6-bit)", decoded: { kind: "value", dpt: "DPST-1-1", text: "On" },
};
const response: BusMonitorTelegramsResponse = {
  sessionId: 1, serverIncarnation: "process-a", contextStatus: "current", projectOpen: true,
  status: "active", nextSince: 1, droppedBefore: 0, telegrams: [telegram],
};

let host: HTMLDivElement | undefined;
let root: Root | undefined;

beforeEach(() => {
  vi.useFakeTimers();
  window.localStorage.clear();
  resetBusDiscoveryForTests();
  apiMock.discoverBusInterfaces.mockResolvedValue({ interfaces: [] });
  // The mount-time reattach finds a live session holding one telegram, which shows the view tablist.
  apiMock.pollBusTelegrams.mockResolvedValue(response);
  // The flow view's snapshot never settles: the tablist, not the flow graph, is under test.
  apiMock.fetchFlowSnapshot.mockReturnValue(new Promise(() => {}));
});
afterEach(() => {
  resetBusDiscoveryForTests();
  resetBusConnectionForTests();
  vi.clearAllMocks();
  vi.useRealTimers();
  resetSettingsForTests();
});

/** The real panel inside a parent that reports every keydown reaching it. */
const adapter: TabsAdapter = {
  name: "BusMonitorPanel view tabs",
  async mount(parentKeyDown) {
    host = document.createElement("div");
    document.body.append(host);
    root = createRoot(host);
    await act(async () => root!.render(<div onKeyDown={parentKeyDown}><BusMonitorPanel projectOpen /></div>));
    await act(async () => { await vi.advanceTimersByTimeAsync(0); });
  },
  async unmount() {
    await act(async () => root!.unmount());
    host!.remove();
    host = undefined; root = undefined;
  },
  tablist: () => host!.querySelector<HTMLElement>('[role="tablist"]')!,
  tabs: () => [...host!.querySelectorAll<HTMLElement>('[role="tab"]')],
  panels: () => [...host!.querySelectorAll<HTMLElement>('[role="tabpanel"]')],
  // Only the selected tab names a panel: the Table panel is not mounted while Flow is shown.
  controls: "selected-only",
  homeEnd: "absent",
  panelStop: "none",
};

describeTabsChecklist(adapter);
