/** Device checks are reachable from the bus workspace without starting a tunnel. */
// @vitest-environment happy-dom
import { act, StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { expect, it, vi } from "vitest";

vi.mock("./BusMonitorPanel", () => ({ default: () => <p>Monitor stub</p> }));
const scanApi = vi.hoisted(() => ({ pollLineScan:vi.fn(),cancelLineScan:vi.fn(),startLineScan:vi.fn() }));
vi.mock("./api", async importOriginal => ({ ...await importOriginal<typeof import("./api")>(),...scanApi }));
vi.mock("./DeviceDownloadPanel", () => ({ default: () => <p>Download stub</p> }));
vi.mock("./AddressProgrammingPanel", () => ({ default: () => <p>Address stub</p> }));
vi.mock("./ServiceControlPanel", () => ({ default: ({ projectOpen }: { projectOpen: boolean }) => <p>Service stub {projectOpen ? "open" : "closed"}</p> }));

import BusDiagnosticsPanel from "./BusDiagnosticsPanel";
import { messages as en } from "./messages/en";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

it("reattaches the real scan child across tabs without cancellation or a second start", async () => {
  const retained: import("./api").LineScanResultsResponse = {
    sessionId:4,serverIncarnation:"retained-process",transport:"tcp",skippedFrames:2,
    status:"running",error:null,cleanupError:null,disconnectConfirmed:null,
    nextSince:1,completedCount:1,totalCount:2,omittedAddresses:[],excludedAddresses:[],
    results:[{address:"1.1.5",outcome:{kind:"occupied",maskVersion:0x07b0}}],
  };
  scanApi.pollLineScan.mockResolvedValue(retained);
  const host=document.createElement("div"); document.body.appendChild(host); const root=createRoot(host);
  const tab=(name:string) => [...host.querySelectorAll<HTMLButtonElement>(".bus-diagnostics-tabs button")].find(button=>button.textContent===name)!;
  try {
    await act(async () => root.render(<StrictMode><BusDiagnosticsPanel project={null} onTreeUpdate={vi.fn()} /></StrictMode>));
    await act(async () => { tab(en["lineScan.title"]).click(); await Promise.resolve(); });
    expect(host.textContent).toContain("Observed scan transport: TCP"); expect(host.textContent).toContain("1.1.5");
    await act(async () => tab(en["serviceControl.tab"]).click());
    await act(async () => { tab(en["lineScan.title"]).click(); await Promise.resolve(); });
    expect(host.querySelectorAll("[data-scan-outcome]")).toHaveLength(1);
    expect(scanApi.pollLineScan).toHaveBeenCalledTimes(2);
    expect(scanApi.pollLineScan).toHaveBeenNthCalledWith(2,0,undefined);
    expect(scanApi.cancelLineScan).not.toHaveBeenCalled(); expect(scanApi.startLineScan).not.toHaveBeenCalled();
  } finally { await act(async () => root.unmount()); host.remove(); vi.clearAllMocks(); }
});

it("opens an explicitly read-only device-checks view without a project", async () => {
  const host = document.createElement("div");
  document.body.appendChild(host);
  const root = createRoot(host);
  await act(async () => root.render(<BusDiagnosticsPanel project={null} onTreeUpdate={vi.fn()} />));
  const tab = [...host.querySelectorAll(".bus-diagnostics-tabs button")].find(
    (button) => button.textContent === en["deviceChecks.tab"],
  );
  expect(tab).toBeDefined();
  await act(async () => (tab as HTMLButtonElement).click());
  expect(tab?.getAttribute("aria-current")).toBe("page");
  expect(host.textContent).toContain(en["deviceChecks.projectRequired"]);
  await act(async () => root.unmount());
  host.remove();
});

it("makes the Debug action reachable without opening a tunnel on tab selection", async () => {
  const host = document.createElement("div");
  document.body.appendChild(host);
  const root = createRoot(host);
  await act(async () => root.render(<BusDiagnosticsPanel project={null} onTreeUpdate={vi.fn()} />));
  const tab = [...host.querySelectorAll(".bus-diagnostics-tabs button")].find(
    (button) => button.textContent === en["serviceControl.tab"],
  );
  expect(tab).toBeDefined();
  await act(async () => (tab as HTMLButtonElement).click());
  expect(tab?.getAttribute("aria-current")).toBe("page");
  expect(host.textContent).toContain("Service stub closed");
  await act(async () => root.unmount());
  host.remove();
});

it("shows the download tab for each status-bar request, and only then (issue #43)", async () => {
  const host = document.createElement("div");
  document.body.appendChild(host);
  const root = createRoot(host);
  const panel = (request: number) => <BusDiagnosticsPanel project={null} onTreeUpdate={vi.fn()} downloadTabRequest={request} />;
  const current = () => host.querySelector(".bus-diagnostics-tabs [aria-current=page]")?.textContent;
  try {
    await act(async () => root.render(panel(0)));
    expect(current()).toBe(en["toolbar.busMonitor"]);
    await act(async () => root.render(panel(1)));
    expect(current()).toBe(en["deviceDownload.title"]);
    expect(host.textContent).toContain("Download stub");
    const other = [...host.querySelectorAll<HTMLButtonElement>(".bus-diagnostics-tabs button")].find((b) => b.textContent === en["serviceControl.tab"])!;
    await act(async () => other.click());
    await act(async () => root.render(panel(1)));
    expect(current()).toBe(en["serviceControl.tab"]);
    await act(async () => root.render(panel(2)));
    expect(current()).toBe(en["deviceDownload.title"]);
  } finally { await act(async () => root.unmount()); host.remove(); }
});
