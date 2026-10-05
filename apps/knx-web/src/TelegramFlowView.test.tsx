/** U20: the flow view renders reducer state honestly and is fully usable from the keyboard. */
// @vitest-environment happy-dom
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, describe, expect, it } from "vitest";
import { flowNow, type FlowFeed } from "./flowFeed";
import { admitRows, createFlowModel, provideContext, type FlowModel, type FlowRowInput } from "./flowModel";
import { snapshotJson } from "./flowTestFixtures";
import { parseFlowSnapshot } from "./flowWire";
import TelegramFlowView from "./TelegramFlowView";
import { resetSettingsForTests, settingsStorage } from "./settingsStore";
import { resetUiLanguageForTests, saveUiLanguage } from "./uiLanguage";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const LIGHT = 0x0801;
let host: HTMLDivElement | undefined;
let root: Root | undefined;

afterEach(async () => {
  document.documentElement.removeAttribute("data-motion-level");
  if (root) await act(async () => root!.unmount());
  root = undefined;
  host?.remove();
  resetSettingsForTests();
  resetUiLanguageForTests();
});

function member(deviceId: number, direction: "Send" | "Receive") {
  return {
    deviceId, comObjectId: deviceId * 100, direction, active: true,
    flags: { communication: true, read: null, write: direction === "Receive", transmit: direction === "Send", update: null, readOnInit: null },
  };
}

function row(seq: number, overrides: Partial<FlowRowInput> = {}): FlowRowInput {
  return {
    seq, service: "GroupValueWrite", source: "1.1.1", destination: "1/0/1", decoded: { kind: "value", text: `v${seq}` },
    sourceRaw: 0x1101, destinationRaw: LIGHT, observedAgeMs: 0, flowGeneration: "1", ...overrides,
  };
}

function model(setup?: (m: FlowModel) => void): FlowModel {
  const m = createFlowModel({ serverIncarnation: "inc-1", sessionId: 7 });
  provideContext(m, "1", parseFlowSnapshot(snapshotJson({
    devices: [
      { deviceId: 1, installationId: 1, name: "Switch", individualAddressRaw: 0x1101 },
      { deviceId: 2, installationId: 1, name: "Dimmer", individualAddressRaw: 0x1102 },
      { deviceId: 3, installationId: 1, name: "Actuator", individualAddressRaw: 0x1103 },
    ],
    groups: [{ gaRaw: LIGHT, gaId: 10, installationId: 1, name: "Light", dpt: "1.001", members: [member(1, "Send"), member(2, "Receive"), member(3, "Receive")] }],
  })), flowNow());
  admitRows(m, [row(1)], flowNow());
  setup?.(m);
  return m;
}

async function render(m: FlowModel | null) {
  const feed: FlowFeed = { model: m, version: 1, admit: () => {}, reset: () => {} };
  host = document.createElement("div");
  document.body.appendChild(host);
  root = createRoot(host);
  await act(async () => root!.render(<TelegramFlowView feed={feed} />));
}

const node = (label: string) => host!.querySelector<SVGGElement>(`g.flow-node[aria-label^="${label}."]`)!;
const key = (target: Element, k: string, shiftKey = false) =>
  act(async () => { target.dispatchEvent(new KeyboardEvent("keydown", { key: k, shiftKey, bubbles: true, cancelable: true })); });
const button = (label: string) => Array.from(host!.querySelectorAll("button")).find((b) => b.textContent === label)!;
const zoomGroup = () => host!.querySelector<SVGGElement>("g[data-zoom]")!;

describe("TelegramFlowView", () => {
  it("explains itself and says when nothing was observed yet", async () => {
    await render(null);
    expect(host!.textContent).toContain("not proof that the device received");
    expect(host!.textContent).toContain("No group telegrams observed");
    expect(host!.querySelector("svg")).toBeNull();
  });

  it("draws senders, configured members and current values, hiding decoration from assistive technology", async () => {
    await render(model());
    expect(node("Switch").getAttribute("aria-label")).toBe("Switch. Project device.");
    expect(host!.querySelectorAll(".flow-edge-configured")).toHaveLength(2);
    expect([...host!.querySelectorAll(".flow-edge-label")].map((label) => label.textContent)).toEqual(["1/0/1", "1/0/1"]);
    expect(node("Switch").querySelector(".flow-badge")!.textContent).toBe("1/0/1 v1");
    expect(node("Dimmer").querySelector(".flow-badge-inferred")!.textContent).toBe("◇ 1/0/1 v1");
    expect(host!.querySelector(".flow-badges")!.getAttribute("aria-hidden")).toBe("true");
    expect(host!.querySelector(".flow-edge")!.closest("[aria-hidden]")).not.toBeNull();
    expect(host!.querySelector("[aria-live]")).toBeNull();
    expect(node("Switch").getAttribute("aria-label")).not.toContain("v1");
  });

  it("moves between nodes in name order and opens the Inspector from the keyboard", async () => {
    await render(model());
    expect(node("Actuator").tabIndex).toBe(0);
    expect(node("Dimmer").tabIndex).toBe(-1);
    await key(node("Actuator"), "ArrowRight");
    expect(node("Dimmer").tabIndex).toBe(0);
    expect(node("Actuator").tabIndex).toBe(-1);
    await key(node("Dimmer"), "Enter");
    const inspector = host!.querySelector(".flow-inspector")!;
    expect(inspector.querySelector("h3")!.textContent).toBe("Dimmer");
    expect([...inspector.querySelectorAll(".flow-values td")].map((cell) => cell.textContent)).toEqual([
      "v1", "configured member (inferred from the project)", "Switch (1.1.1)", "1",
    ]);
    expect(inspector.textContent).toContain("From Switch · configured in the project");
    expect(inspector.textContent).toContain("1/0/1: observed 1× (project evidence from interpretation 1)");
    expect(inspector.querySelector(".flow-flags caption")!.textContent).toBe("Object #200 · Receive · active");
    expect([...inspector.querySelectorAll(".flow-flags tbody tr")].map((r) => r.textContent)).toEqual([
      "Communicationyes", "Readunknown", "Writeyes", "Transmitno", "Updateunknown", "Read on initunknown",
    ]);
    await key(node("Dimmer"), "End");
    expect(node("Switch").tabIndex).toBe(0);
    await key(node("Switch"), "Home");
    expect(node("Actuator").tabIndex).toBe(0);
  });

  it("names ambiguous and raw senders in the Inspector", async () => {
    const m = createFlowModel({ serverIncarnation: "inc-1", sessionId: 7 });
    provideContext(m, "1", parseFlowSnapshot(snapshotJson({ status: "historical", groupAddressStyle: null, devices: [], groups: [] })), flowNow());
    admitRows(m, [row(1)], flowNow());
    await render(m);
    await act(async () => { node("1.1.1").dispatchEvent(new MouseEvent("click", { bubbles: true })); });
    expect(host!.querySelector(".flow-inspector")!.textContent).toContain("interpretation the server no longer holds");
    expect(host!.textContent).toContain("Senders drawn without project participants: 1.");
  });

  it("states refusals, waiting rows and the end of the session", async () => {
    await render(model((m) => {
      m.counters.refusedNodes = 2;
      m.closed = true;
      admitRows(m, [row(2, { flowGeneration: "2" })], flowNow());
    }));
    expect(host!.textContent).toContain("Map full: 2 nodes");
    expect(host!.textContent).toContain("Telegrams waiting for their participant list: 1.");
    expect(host!.textContent).toContain("The monitor session has ended");
  });

  it("zooms, pans and resets with buttons and keys", async () => {
    await render(model());
    await act(async () => button("Zoom in").click());
    expect(zoomGroup().getAttribute("data-zoom")).toBe("1.25");
    await key(node("Actuator"), "-");
    expect(zoomGroup().getAttribute("data-zoom")).toBe("1");
    await key(node("Actuator"), "ArrowLeft", true);
    expect(zoomGroup().getAttribute("transform")).toBe("translate(60 0) scale(1)");
    expect(node("Actuator").tabIndex).toBe(0);
    await key(node("Actuator"), "0");
    expect(zoomGroup().getAttribute("transform")).toBe("translate(0 0) scale(1)");
  });

  it("speaks German", async () => {
    saveUiLanguage(settingsStorage, "de");
    await render(model());
    expect(button("Vergrößern")).toBeTruthy();
    expect(node("Switch").getAttribute("aria-label")).toBe("Switch. Projektgerät.");
    expect(host!.textContent).toContain("kein Nachweis, dass das Gerät das Telegramm empfangen");
  });
});

// U21: motion controls, the activity leader and honest load reporting.
describe("TelegramFlowView motion", () => {
  async function rerender(m: FlowModel, version: number) {
    const feed: FlowFeed = { model: m, version, admit: () => {}, reset: () => {} };
    await act(async () => root!.render(<TelegramFlowView feed={feed} />));
  }

  it("offers Freeze with motion on, and disables it with an explanation when motion is off", async () => {
    await render(model());
    const freeze = button("Freeze layout");
    expect(freeze.disabled).toBe(false);
    expect(freeze.getAttribute("aria-pressed")).toBe("false");
    await act(async () => freeze.click());
    expect(freeze.getAttribute("aria-pressed")).toBe("true");
    await act(async () => { document.documentElement.setAttribute("data-motion-level", "off"); await Promise.resolve(); });
    expect(button("Freeze layout").disabled).toBe(true);
    expect(host!.textContent).toContain("Motion is off: the layout stays still and no pulses are drawn.");
  });

  it("names the most active sender of the last 60 s and marks its node", async () => {
    await render(model());
    expect(host!.querySelector(".flow-leader")!.textContent).toBe("Most active sender (last 60 s): Switch");
    expect(node("Switch").classList.contains("flow-node-leader")).toBe(true);
    expect(node("Dimmer").classList.contains("flow-node-leader")).toBe(false);
  });

  it("says when pulses were bundled, and that values and counts stay complete", async () => {
    const m = model((x) => admitRows(x, Array.from({ length: 30 }, (_, i) => row(i + 2)), flowNow()));
    await render(m);
    await rerender(m, 2);
    expect(host!.querySelector(".flow-reduced")!.textContent).toContain("telegrams were drawn as bundled pulses");
    expect(host!.querySelector(".flow-reduced")!.textContent).toContain("Values and counts are complete.");
  });

  it("draws a fresh edge at full emphasis", async () => {
    await render(model());
    expect((host!.querySelector(".flow-edge") as SVGGElement).style.opacity).toBe("1");
  });
});
