/** Devices → download hand-off: tab switch, single delivery, button contract. */
// @vitest-environment happy-dom
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import type { DownloadHandoff } from "./downloadHandoff";

const seen = vi.hoisted(() => ({ handoffs: [] as Array<number | null> }));
vi.mock("./BusMonitorPanel", () => ({ default: () => <p>Monitor stub</p> }));
vi.mock("./LineScanPanel", () => ({ default: () => <p>Scan stub</p> }));
vi.mock("./AddressProgrammingPanel", () => ({ default: () => <p>Address stub</p> }));
vi.mock("./ServiceControlPanel", () => ({ default: () => <p>Service stub</p> }));
vi.mock("./DeviceDownloadPanel", async () => {
  const { useEffect } = await import("react");
  return {
    default: ({ handoff, onHandoffHandled }: { handoff: DownloadHandoff | null; onHandoffHandled?: (nonce: number) => void }) => {
      useEffect(() => {
        seen.handoffs.push(handoff?.nonce ?? null);
        if (handoff) onHandoffHandled?.(handoff.nonce);
      }, [handoff, onHandoffHandled]);
      return <p>Download stub {handoff ? `device ${handoff.deviceId}` : "none"}</p>;
    },
  };
});

import BusDiagnosticsPanel from "./BusDiagnosticsPanel";
import OpenDownloadButton, { DownloadNavigationProvider } from "./OpenDownloadButton";
import { messages as en } from "./messages/en";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

let host: HTMLDivElement;
let root: Root;
beforeEach(() => {
  seen.handoffs = [];
  host = document.createElement("div"); document.body.appendChild(host); root = createRoot(host);
});
afterEach(async () => { await act(async () => root.unmount()); host.remove(); });

const tab = (label: string) => [...host.querySelectorAll<HTMLButtonElement>(".bus-diagnostics-tabs button")].find((b) => b.textContent === label)!;
const request = (deviceId: number, nonce: number): DownloadHandoff => ({ deviceId, nonce, context: "srv/1" });
const panel = (handoff: DownloadHandoff | null) => <BusDiagnosticsPanel project={null} onTreeUpdate={vi.fn()} downloadHandoff={handoff} />;

it("opens the download tab for a new request and delivers it exactly once", async () => {
  await act(async () => root.render(panel(null)));
  expect(tab(en["toolbar.busMonitor"]).getAttribute("aria-current")).toBe("page");
  await act(async () => root.render(panel(request(10, 1))));
  expect(tab(en["deviceDownload.title"]).getAttribute("aria-current")).toBe("page");
  expect(seen.handoffs.filter((nonce) => nonce === 1)).toHaveLength(1);
  expect(host.textContent).toContain("Download stub none");

  // Leaving and coming back must not replay the answered request.
  await act(async () => tab(en["toolbar.busMonitor"]).click());
  await act(async () => tab(en["deviceDownload.title"]).click());
  expect(seen.handoffs.filter((nonce) => nonce === 1)).toHaveLength(1);
  expect(host.textContent).toContain("Download stub none");

  // A re-render with the same request is not a new click either.
  await act(async () => tab(en["toolbar.busMonitor"]).click());
  await act(async () => root.render(panel(request(10, 1))));
  expect(tab(en["toolbar.busMonitor"]).getAttribute("aria-current")).toBe("page");
});

it("treats a repeated click on the same device as a new request", async () => {
  await act(async () => root.render(panel(request(10, 1))));
  await act(async () => tab(en["toolbar.busMonitor"]).click());
  await act(async () => root.render(panel(request(10, 2))));
  expect(tab(en["deviceDownload.title"]).getAttribute("aria-current")).toBe("page");
  expect(seen.handoffs.filter((nonce) => nonce === 2)).toHaveLength(1);
});

it("offers the button only inside the workbench and passes the stable device id", async () => {
  await act(async () => root.render(<OpenDownloadButton deviceId={10} address="1.1.67" />));
  expect(host.querySelector("button")).toBeNull();
  const open = vi.fn();
  await act(async () => root.render(<DownloadNavigationProvider onOpen={open}>
    <OpenDownloadButton deviceId={10} address="1.1.67" />
  </DownloadNavigationProvider>));
  await act(async () => host.querySelector("button")!.click());
  expect(open).toHaveBeenCalledExactlyOnceWith(10);
});

it("explains instead of offering a download for a device without an individual address", async () => {
  const open = vi.fn();
  await act(async () => root.render(<DownloadNavigationProvider onOpen={open}>
    <OpenDownloadButton deviceId={12} address={null} />
  </DownloadNavigationProvider>));
  const button = host.querySelector("button")!;
  expect(button.disabled).toBe(true);
  expect(host.textContent).toContain(en["devices.downloadNeedsAddress"]);
  expect(document.getElementById(button.getAttribute("aria-describedby")!)?.textContent).toBe(en["devices.downloadNeedsAddress"]);
  await act(async () => button.click());
  expect(open).not.toHaveBeenCalled();
});
