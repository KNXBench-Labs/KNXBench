/** Device checks are reachable from the bus workspace without starting a tunnel. */
// @vitest-environment happy-dom
import { act } from "react";
import { createRoot } from "react-dom/client";
import { expect, it, vi } from "vitest";

vi.mock("./BusMonitorPanel", () => ({ default: () => <p>Monitor stub</p> }));
vi.mock("./LineScanPanel", () => ({ default: () => <p>Scan stub</p> }));
vi.mock("./DeviceDownloadPanel", () => ({ default: () => <p>Download stub</p> }));
vi.mock("./AddressProgrammingPanel", () => ({ default: () => <p>Address stub</p> }));
vi.mock("./ServiceControlPanel", () => ({ default: ({ projectOpen }: { projectOpen: boolean }) => <p>Service stub {projectOpen ? "open" : "closed"}</p> }));

import BusDiagnosticsPanel from "./BusDiagnosticsPanel";
import { messages as en } from "./messages/en";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

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
