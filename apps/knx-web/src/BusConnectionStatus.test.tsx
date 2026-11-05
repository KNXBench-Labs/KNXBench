/** Verify explicit status-bar bus actions and accurate IP/serial targets. */
// @vitest-environment happy-dom
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { BusDiscoveredInterface, BusMonitorStartResponse, BusMonitorStopResponse } from "./api";
import type { ActivitySnapshot, SessionActivity } from "./liveActivity";

const apiMock = vi.hoisted(() => ({
  busActivity: vi.fn(),
  startBusMonitor: vi.fn(),
  startSerialBusMonitor: vi.fn(),
  stopBusMonitor: vi.fn(),
  stopBoundBusMonitor: vi.fn(),
  discoverBusInterfaces: vi.fn(),
}));
vi.mock("./api", async (importOriginal) => ({
  ...(await importOriginal<typeof import("./api")>()),
  ...apiMock,
}));

import BusConnectionStatus from "./BusConnectionStatus";
import { refreshBusConnection, resetBusConnectionForTests } from "./busConnection";
import { resetBusDiscoveryForTests, searchBusInterfaces } from "./busDiscovery";
import { loadPreferredGateway, savePreferredGateway } from "./gatewayPreference";
import { resetSettingsForTests, setSetting } from "./settingsStore";
import { UI_LANGUAGE_STORAGE_KEY } from "./uiLanguage";
import { messages as en } from "./messages/en";
import { messages as de } from "./messages/de";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

function snapshot(partial: Partial<ActivitySnapshot> = {}): ActivitySnapshot {
  return {
    serverIncarnation: "process-a", coverage: "partial", historyState: "disabled", sessions: [], oneShot: [],
    oneShotDropped: 0, busyLocks: [], untracked: ["groupWrite", "serialAddress"], ...partial,
  } satisfies ActivitySnapshot;
}
const monitor = (state: "active" | "closed", gateway = "192.0.2.10:3671"): SessionActivity =>
  ({ kind: "busMonitor", id: 5, gateway, state }) satisfies SessionActivity;
const tunnel: BusDiscoveredInterface = {
  controlEndpoint: "192.0.2.20:3671", individualAddress: "1.1.0", friendlyName: "Hall interface", supportsTunnelling: true,
};
const router: BusDiscoveredInterface = {
  controlEndpoint: "192.0.2.30:3671", individualAddress: "1.0.0", friendlyName: "Router only", supportsTunnelling: false,
};
const started = { sessionId: 5, serverIncarnation: "process-a", assignedAddress: "1.1.250" } satisfies BusMonitorStartResponse;
const summary = { sessionId: 5, serverIncarnation: "process-a", telegramCount: 0, droppedCount: 0 } satisfies BusMonitorStopResponse;

let host: HTMLDivElement;
let root: Root;
const onOpenBusSettings = vi.fn();
const flush = () => act(async () => { for (let i = 0; i < 6; i += 1) await Promise.resolve(); });
const q = <T extends Element = HTMLElement>(selector: string) => host.querySelector<T & HTMLElement>(selector);
const action = () => q<HTMLButtonElement>(".bus-connection-action")!;
const trigger = () => q<HTMLButtonElement>(".bus-connection-trigger")!;
const stateText = () => q(".bus-connection-state")!.textContent;

async function render() {
  host = document.createElement("div");
  document.body.append(host);
  root = createRoot(host);
  await act(async () => root.render(<BusConnectionStatus onOpenBusSettings={onOpenBusSettings} />));
  await flush();
}
async function click(element: HTMLElement) {
  await act(async () => element.click());
  await flush();
}

beforeEach(() => {
  resetBusConnectionForTests();
  resetBusDiscoveryForTests();
  resetSettingsForTests();
  localStorage.clear();
  for (const fn of Object.values(apiMock)) fn.mockReset();
  onOpenBusSettings.mockReset();
  apiMock.busActivity.mockResolvedValue(snapshot());
  apiMock.discoverBusInterfaces.mockResolvedValue({ interfaces: [tunnel, router] });
  savePreferredGateway("192.0.2.20:3671");
});
afterEach(async () => {
  await act(async () => root.unmount());
  host.remove();
  resetBusConnectionForTests();
});

describe("BusConnectionStatus", () => {
  it("selecting an entry only saves the preference; one Connect click is one start with it", async () => {
    await searchBusInterfaces();
    await render();
    expect(stateText()).toContain(en["busConnection.state.disconnected"]);
    await click(trigger());
    expect(trigger().getAttribute("aria-expanded")).toBe("true");
    const options = [...host.querySelectorAll<HTMLButtonElement>(".bus-connection-panel .bus-discovery-option")];
    const routerOption = options.find((option) => option.textContent?.includes("Router only"))!;
    expect(routerOption.disabled).toBe(true);
    expect(routerOption.textContent).toContain(en["busConnection.notTunnelling"]);
    savePreferredGateway("192.0.2.99:3671");
    await flush();
    const hall = [...host.querySelectorAll<HTMLButtonElement>(".bus-connection-panel .bus-discovery-option")]
      .find((option) => option.textContent?.includes("Hall interface"))!;
    await click(hall);
    expect(loadPreferredGateway()).toBe("192.0.2.20:3671");
    expect(apiMock.startBusMonitor).not.toHaveBeenCalled();

    let finish!: (value: BusMonitorStartResponse) => void;
    apiMock.startBusMonitor.mockReturnValueOnce(new Promise((resolve) => { finish = resolve; }));
    await act(async () => { action().click(); action().click(); });
    expect(apiMock.startBusMonitor).toHaveBeenCalledTimes(1);
    expect(apiMock.startBusMonitor).toHaveBeenCalledWith("192.0.2.20:3671");
    expect(stateText()).toContain(en["busConnection.state.connecting"]);
    expect(action().disabled).toBe(true);
    apiMock.busActivity.mockResolvedValue(snapshot({ sessions: [monitor("active", "192.0.2.20:3671")] }));
    await act(async () => finish(started));
    await flush();
    expect(stateText()).toContain("Connected to 192.0.2.20:3671");
    expect(action().textContent).toBe(en["busConnection.disconnect"]);
  });

  it("shows connected A and selected B apart, never switches, and disconnects A on the bound route", async () => {
    apiMock.busActivity.mockResolvedValue(snapshot({ sessions: [monitor("active", "192.0.2.10:3671")] }));
    await render();
    expect(stateText()).toContain("192.0.2.10:3671");
    expect(trigger().textContent).toContain("192.0.2.20:3671");
    await click(trigger());
    expect(q(".bus-connection-panel")!.textContent).toContain(
      "Connected to 192.0.2.10:3671. 192.0.2.20:3671 is used for the next connection.",
    );
    apiMock.stopBoundBusMonitor.mockResolvedValueOnce(summary);
    apiMock.busActivity.mockResolvedValue(snapshot());
    await click(action());
    expect(apiMock.stopBoundBusMonitor).toHaveBeenCalledWith({ sessionId: 5, serverIncarnation: "process-a" });
    expect(apiMock.stopBusMonitor).not.toHaveBeenCalled();
    expect(apiMock.startBusMonitor).not.toHaveBeenCalled();
    expect(action().textContent).toBe(en["busConnection.connect"]);
    apiMock.startBusMonitor.mockResolvedValueOnce(started);
    await click(action());
    expect(apiMock.startBusMonitor).toHaveBeenCalledWith("192.0.2.20:3671");
  });

  it("offers only a bound Disconnect for a lost retained session", async () => {
    apiMock.busActivity.mockResolvedValue(snapshot({ sessions: [monitor("closed")] }));
    await render();
    expect(stateText()).toContain("Connection lost: 192.0.2.10:3671");
    expect(action().textContent).toBe(en["busConnection.disconnect"]);
    expect(action().disabled).toBe(false);
  });

  it.each([
    ["occupied", snapshot({ busyLocks: ["system2Operation"] }), en["busConnection.state.occupied"]],
    ["indeterminate", snapshot({ busyLocks: ["busMonitorOrGroupWrite"] }), en["busConnection.state.indeterminate"]],
  ])("disables Connect while %s, with a reason and a manual check", async (_name, answer, text) => {
    apiMock.busActivity.mockResolvedValue(answer);
    await render();
    expect(stateText()).toContain(text);
    expect(action().disabled).toBe(true);
    expect(action().title).toBe(en["busConnection.connectBlocked"]);
    await click(trigger());
    const calls = apiMock.busActivity.mock.calls.length;
    const check = [...host.querySelectorAll<HTMLButtonElement>(".bus-connection-tools button")]
      .find((button) => button.textContent === en["busConnection.checkStatus"])!;
    await click(check);
    expect(apiMock.busActivity.mock.calls.length).toBeGreaterThan(calls);
    expect(apiMock.startBusMonitor).not.toHaveBeenCalled();
  });

  it("never shows a failed read as disconnected or connected", async () => {
    apiMock.busActivity.mockRejectedValue(Object.assign(new Error("boom"), { status: 500 }));
    await render();
    expect(stateText()).toContain(en["busConnection.state.unknown"]);
    expect(stateText()).not.toContain(en["busConnection.state.disconnected"]);
    expect(action().disabled).toBe(true);
  });

  it("disables Connect for an unsupported saved gateway but keeps it shown", async () => {
    savePreferredGateway("gateway.example:3671");
    await render();
    expect(action().disabled).toBe(true);
    expect(action().title).toBe(en["busMonitor.invalidHost"]);
    await click(trigger());
    expect(q(".bus-connection-panel")!.textContent).toContain("gateway.example:3671");
  });

  it("words a busy refusal, retries only on a new click, and never falls back on a missing bound route", async () => {
    await render();
    const busy = Object.assign(new Error("{\"raw\":true}"), {
      status: 409, body: { error: "x", kind: "gatewayBusy", holder: { kind: "connecting", session: "system2Operation" } },
    });
    apiMock.startBusMonitor.mockRejectedValueOnce(busy);
    await click(action());
    expect(q(".bus-connection-error")!.textContent).toBe(en["gatewayRefusal.busy.connecting.system2Operation"]);
    expect(host.textContent).not.toContain("raw");
    expect(apiMock.startBusMonitor).toHaveBeenCalledTimes(1);
    apiMock.startBusMonitor.mockResolvedValueOnce(started);
    await click(action());
    expect(apiMock.startBusMonitor).toHaveBeenCalledTimes(2);

    apiMock.busActivity.mockResolvedValue(snapshot({ sessions: [monitor("active")] }));
    await act(async () => window.dispatchEvent(new Event("focus")));
    await flush();
    apiMock.stopBoundBusMonitor.mockRejectedValueOnce(Object.assign(new Error("Not Found"), { status: 404, body: null }));
    await click(action());
    expect(q(".bus-connection-error")!.textContent).toBe(en["busConnection.error.unsupported"]);
    expect(apiMock.stopBusMonitor).not.toHaveBeenCalled();
    const changed = Object.assign(new Error("changed"), { status: 409, body: { error: "changed", kind: "busSessionChanged" } });
    apiMock.stopBoundBusMonitor.mockRejectedValueOnce(changed);
    await click(action());
    expect(q(".bus-connection-error")!.textContent).toBe(en["gatewayRefusal.busSessionChanged"]);
    expect(apiMock.stopBusMonitor).not.toHaveBeenCalled();
  });

  it("closes on Escape with focus back on the trigger, and on an outside press", async () => {
    await render();
    await click(trigger());
    const first = q<HTMLButtonElement>(".bus-connection-tools button")!;
    first.focus();
    await act(async () => first.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true })));
    expect(q(".bus-connection-panel")).toBeNull();
    expect(document.activeElement).toBe(trigger());
    await click(trigger());
    await act(async () => document.body.dispatchEvent(new PointerEvent("pointerdown", { bubbles: true })));
    expect(q(".bus-connection-panel")).toBeNull();
  });

  it("opens Bus settings through its own callback and starts nothing on passive events", async () => {
    await render();
    await click(trigger());
    const settings = [...host.querySelectorAll<HTMLButtonElement>(".bus-connection-tools button")]
      .find((button) => button.textContent === en["busConnection.settings"])!;
    await click(settings);
    expect(onOpenBusSettings).toHaveBeenCalledTimes(1);
    expect(onOpenBusSettings).toHaveBeenCalledWith();
    await act(async () => {
      window.dispatchEvent(new Event("focus"));
      document.dispatchEvent(new Event("visibilitychange"));
      window.dispatchEvent(new Event("knx-desktop:context-changed"));
    });
    await flush();
    expect(apiMock.startBusMonitor).not.toHaveBeenCalled();
    expect(apiMock.stopBoundBusMonitor).not.toHaveBeenCalled();
  });

  it("speaks German and names no USB support in either language", async () => {
    setSetting(UI_LANGUAGE_STORAGE_KEY, "de");
    await render();
    expect(stateText()).toContain(de["busConnection.state.disconnected"]);
    expect(action().textContent).toBe(de["busConnection.connect"]);
    const ours = (catalogue: Record<string, string>) =>
      Object.entries(catalogue).filter(([key]) => key.startsWith("busConnection.")).map(([, text]) => text).join(" ");
    expect(ours(en)).not.toMatch(/usb/i);
    expect(ours(de)).not.toMatch(/usb/i);
  });
});

describe("R3: System-2 outranks a retained monitor in the footer", () => {
  it.each(["active", "closed"] as const)("a %s monitor plus System-2 shows the holder, a warning tone and no action", async (state) => {
    apiMock.busActivity.mockResolvedValue(snapshot({ sessions: [monitor(state)], busyLocks: ["system2Operation"] }));
    await render();
    expect(stateText()).toBe(en["busConnection.state.occupied"]);
    expect(q(".bus-connection-state")!.title).toBe(en["gatewayRefusal.busy.connecting.system2Operation"]);
    expect(q(".bus-connection-indicator")!.dataset.tone).toBe("warning");
    expect(action().textContent).toBe(en["busConnection.connect"]);
    expect(action().disabled).toBe(true);
    await click(trigger());
    expect(q(".bus-connection-panel")!.textContent).toContain(en["gatewayRefusal.busy.connecting.system2Operation"]);
    expect(apiMock.stopBoundBusMonitor).not.toHaveBeenCalled();
  });
});

describe("R8: a suppressed stop publication never invents a newer connection", () => {
  it("words a server restart with no session as changed state, in English and German", async () => {
    apiMock.busActivity.mockResolvedValue(snapshot({ sessions: [monitor("active")] }));
    await render();
    let finish!: (value: BusMonitorStopResponse) => void;
    apiMock.stopBoundBusMonitor.mockReturnValueOnce(new Promise((resolve) => { finish = resolve; }));
    await act(async () => action().click());
    apiMock.busActivity.mockResolvedValue(snapshot({ serverIncarnation: "process-b" }));
    await act(async () => window.dispatchEvent(new Event("focus")));
    await flush();
    await act(async () => finish(summary));
    await flush();
    const text = q(".bus-connection-error")!.textContent!;
    expect(text).toBe(en["busConnection.error.staleStop"]);
    expect(text).not.toMatch(/newer/i);
    expect(de["busConnection.error.staleStop"]).not.toMatch(/neuere/i);
    expect(stateText()).toContain(en["busConnection.state.disconnected"]);
    expect(apiMock.stopBusMonitor).not.toHaveBeenCalled();
  });
});


describe("serial status controller join", () => {
  it.each(["passive", "active"] as const)("selects without traffic, discloses %s, and uses the shared bound session", async mode => {
    const serial = {path:"/synthetic/tty",source:"1.1.250",mode};
    setSetting("preferredBusTransport", "serial"); setSetting("serialPath", serial.path);
    setSetting("serialSource", serial.source); setSetting("serialMode", mode);
    await render(); await act(async () => refreshBusConnection());
    expect(apiMock.startSerialBusMonitor).not.toHaveBeenCalled(); expect(apiMock.startBusMonitor).not.toHaveBeenCalled();
    await click(action());
    expect(q(".bus-connection-panel")!.textContent).toContain(en[mode === "active" ? "busTransport.activeWarning" : "busTransport.passiveHint"]);
    expect(apiMock.startSerialBusMonitor).not.toHaveBeenCalled();
    apiMock.startSerialBusMonitor.mockResolvedValueOnce({...started,serial});
    apiMock.busActivity.mockResolvedValue(snapshot({sessions:[{kind:"busMonitor",id:5,state:"active",serial}]}));
    const connect = [...host.querySelectorAll<HTMLButtonElement>(".bus-connection-panel button")].find(button => button.textContent === en["busConnection.connect"])!;
    await click(connect);
    expect(apiMock.startSerialBusMonitor).toHaveBeenCalledExactlyOnceWith(serial);
    expect(apiMock.startBusMonitor).not.toHaveBeenCalled();
    expect(stateText()).toContain(serial.path); expect(stateText()).not.toContain("192.0.2.");
    expect(trigger().textContent).toContain("Interface:"); expect(trigger().textContent).not.toContain("Gateway:");
    await act(async () => setSetting("serialPath", "/synthetic/other"));
    expect(stateText()).toContain(serial.path);
    expect(q(".bus-connection-panel")!.textContent).toContain("/synthetic/other");
    apiMock.busActivity.mockResolvedValue(snapshot());
    apiMock.stopBoundBusMonitor.mockResolvedValueOnce(mode === "active" ? {...summary,warning:"reset failed; auto-ACK state unknown"} : summary);
    await click(action());
    if(mode === "active") expect(q(".bus-connection-error")!.textContent).toBe(en["busTransport.cleanupWarning"]);
    else expect(q(".bus-connection-error")).toBeNull();
    expect(apiMock.stopBoundBusMonitor).toHaveBeenCalledExactlyOnceWith({sessionId:5,serverIncarnation:"process-a"});
    expect(apiMock.stopBusMonitor).not.toHaveBeenCalled(); expect(loadPreferredGateway()).toBe("192.0.2.20:3671");
  });
  it("disables invalid serial targets and reports an unavailable server without claiming a session", async () => {
    setSetting("preferredBusTransport","serial"); await render(); expect(action().disabled).toBe(true);
    await act(async () => { setSetting("serialPath","/synthetic/tty"); setSetting("serialSource","1.1.250"); });
    await click(action()); apiMock.startSerialBusMonitor.mockRejectedValueOnce(new Error("missing route"));
    const connect=[...host.querySelectorAll<HTMLButtonElement>(".bus-connection-panel button")].find(button=>button.textContent===en["busConnection.connect"])!;
    await click(connect); expect(q(".bus-connection-error")!.textContent).toBe(en["busTransport.unavailable"]);
    expect(stateText()).toContain(en["busConnection.state.disconnected"]); expect(apiMock.startBusMonitor).not.toHaveBeenCalled();
  });
});

it.each(["en","de"] as const)("lists and selects a candidate without replacing the observed serial session in %s", async language => {
  const m = language === "en" ? en : de;
  const serial = { path: "/synthetic/connected", source: "1.1.249", mode: "active" as const };
  const candidate = { path: "/dev/serial/by-id/usb-fictional-if00", target: "/dev/ttyACM0" };
  setSetting("uiLanguage", language); setSetting("preferredBusTransport", "serial");
  setSetting("serialPath", serial.path); setSetting("serialSource", serial.source); setSetting("serialMode", serial.mode);
  apiMock.busActivity.mockResolvedValue(snapshot({ sessions: [{ kind: "busMonitor", id: 5, state: "active", serial }] }));
  const fetcher = vi.fn().mockResolvedValue(new Response(JSON.stringify({ status: "available", candidates: [candidate], truncated: false, omittedEntries: 0 })));
  vi.stubGlobal("fetch", fetcher);
  try {
    await render(); await act(async () => refreshBusConnection()); await click(trigger());
    expect(fetcher).not.toHaveBeenCalled();
    const connected = stateText(); expect(connected).toContain(serial.path);
    const discoveries = apiMock.discoverBusInterfaces.mock.calls.length;
    const button = (text: string) => [...host.querySelectorAll<HTMLButtonElement>(".bus-connection-panel button")].find(node => node.textContent === text)!;
    await click(button(m["serialCandidates.list"]));
    expect(stateText()).toBe(connected);
    await click(button(m["serialCandidates.use"].replace("{path}", candidate.path)));
    expect(stateText()).toBe(connected);
    expect(q(".bus-connection-panel")!.textContent).toContain(candidate.path);
    expect(loadPreferredGateway()).toBe("192.0.2.20:3671");
    expect(fetcher).toHaveBeenCalledExactlyOnceWith("/api/bus/serial-candidates", { headers: undefined, method: "GET", cache: "no-store" });
    expect(apiMock.discoverBusInterfaces).toHaveBeenCalledTimes(discoveries);
    expect(apiMock.startSerialBusMonitor).not.toHaveBeenCalled(); expect(apiMock.startBusMonitor).not.toHaveBeenCalled();
    expect(apiMock.stopBoundBusMonitor).not.toHaveBeenCalled(); expect(apiMock.stopBusMonitor).not.toHaveBeenCalled();
    const fields = [...host.querySelectorAll<HTMLLabelElement>(".bus-transport-fields label")];
    expect(fields.find(label => label.textContent === m["busTransport.source"])!.querySelector("input")!.value).toBe(serial.source);
    expect(q<HTMLSelectElement>(".bus-transport-fields label:last-of-type select")!.value).toBe(serial.mode);
  } finally { vi.unstubAllGlobals(); }
});

it.each(["en","de"] as const)("preserves active-start uncertainty through the real API/error path in %s",async language=>{
  const m=language==="en"?en:de;
  setSetting("uiLanguage",language);setSetting("preferredBusTransport","serial");setSetting("serialPath","/synthetic/tty");setSetting("serialSource","1.1.250");
  const actual=await vi.importActual<typeof import("./api")>("./api");
  apiMock.startSerialBusMonitor.mockImplementation(actual.startSerialBusMonitor);
  try {
    for(const mode of ["active","passive"] as const){
      setSetting("serialMode",mode);
      vi.stubGlobal("fetch",vi.fn().mockResolvedValue(new Response(JSON.stringify({kind:"serialConnectFailed",activeAutoAckStateUncertain:mode==="active",error:mode==="active"?"initialization/reset failed; auto-ACK state unknown":"ordinary open failure"}),{status:502,headers:{"Content-Type":"application/json"}})));
      if(mode==="active") await render();
      await click(trigger());
      const connect=[...host.querySelectorAll<HTMLButtonElement>(".bus-connection-panel button")].find(button=>button.textContent===m["busConnection.connect"])!;
      await click(connect);
      expect(q(".bus-connection-error")!.textContent).toBe(m[mode==="active"?"busTransport.activeStartUncertain":"busTransport.unavailable"]);
      expect(fetch).toHaveBeenCalledTimes(1);expect(apiMock.startBusMonitor).not.toHaveBeenCalled();
      if(mode==="active") await click(trigger());
    }
  }finally{vi.unstubAllGlobals();}
});
