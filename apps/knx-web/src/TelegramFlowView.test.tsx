/** U20: the flow view renders reducer state honestly and is fully usable from the keyboard. */
// @vitest-environment happy-dom
import { tableColumnSmoke } from "./tableColumnSmoke.fixture";
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, describe, expect, it } from "vitest";
import { flowNow, type FlowFeed } from "./flowFeed";
import { admitRows, createFlowModel, provideContext, type FlowModel, type FlowRowInput } from "./flowModel";
import { snapshotJson } from "./flowTestFixtures";
import { parseFlowSnapshot } from "./flowWire";
import { translateFor } from "./i18n";
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

async function render(m: FlowModel | null, source?: FlowFeed["source"]) {
  const feed: FlowFeed = { model: m, version: 1, source, admit: () => {}, reset: () => {} };
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
  it("opens image exports only for the chosen visible graph and pins a smooth camera", async () => {
    await render(model());
    expect(zoomGroup().classList.contains("flow-camera")).toBe(true);
    await act(async () => button("PNG snapshot").click());
    expect(host!.querySelector('.flow-export-dialog')).not.toBeNull();
    expect(host!.querySelector('.flow-export-consent')?.textContent).toContain("locally");
    expect(host!.querySelector('[data-export-start]')?.hasAttribute("disabled")).toBe(true);
    await act(async () => button("Close").click());
    expect(host!.querySelector('.flow-export-dialog')).toBeNull();
    await act(async () => button("GIF clip").click());
    expect(host!.querySelector<HTMLInputElement>('.flow-export-dialog input[type="number"]')!.max).toBe("30");
  });
  it("keeps capture-loss and stale-context warnings inside the maximizable/shareable view", async () => {
    await render(model(), { gaps: 3, pruned: 2, context: "stale", ended: false, error: null });
    expect(host!.querySelector(".flow-source-diagnostics")?.textContent).toContain("3");
    expect(host!.querySelector(".flow-source-diagnostics")?.textContent).toContain(translateFor("en", "busMonitor.contextStale"));
  });
  it("reserves no Inspector until selection and lets it be closed", async () => {
    await render(model());
    expect(host!.querySelector(".flow-inspector")).toBeNull();
    await act(async () => node("Switch").dispatchEvent(new MouseEvent("click", { bubbles: true })));
    expect(host!.querySelector(".flow-inspector")).not.toBeNull();
    await act(async () => button("Close details").click());
    expect(host!.querySelector(".flow-inspector")).toBeNull();
  });

  it("offers automatic fit, rearrange and an in-app maximized view", async () => {
    await render(model());
    expect(button("Auto zoom")?.getAttribute("aria-pressed")).toBe("true");
    expect(button("Rearrange layout")).toBeTruthy();
    await act(async () => button("Maximize view").click());
    expect(host!.querySelector(".telegram-flow")?.classList.contains("flow-maximized")).toBe(true);
    await act(async () => button("Restore view").click());
    expect(host!.querySelector(".telegram-flow")?.classList.contains("flow-maximized")).toBe(false);
  });

  it("manual zoom disables automatic fit until explicitly enabled", async () => {
    await render(model());
    await act(async () => button("Zoom in").click());
    expect(button("Auto zoom")?.getAttribute("aria-pressed")).toBe("false");
    await act(async () => button("Auto zoom").click());
    expect(button("Auto zoom").getAttribute("aria-pressed")).toBe("true");
  });
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
    const initial = Number(zoomGroup().getAttribute("data-zoom"));
    await act(async () => button("Zoom in").click());
    expect(Number(zoomGroup().getAttribute("data-zoom"))).toBeCloseTo(initial * 1.25);
    await key(node("Actuator"), "-");
    expect(Number(zoomGroup().getAttribute("data-zoom"))).toBeCloseTo(initial);
    await key(node("Actuator"), "0");
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

  it("offers Freeze even with motion off, so automatic static rearrangement can be held", async () => {
    await render(model());
    const freeze = button("Freeze layout");
    expect(freeze.disabled).toBe(false);
    expect(freeze.getAttribute("aria-pressed")).toBe("false");
    await act(async () => freeze.click());
    expect(freeze.getAttribute("aria-pressed")).toBe("true");
    await act(async () => { document.documentElement.setAttribute("data-motion-level", "off"); await Promise.resolve(); });
    expect(button("Freeze layout").disabled).toBe(false);
    expect(host!.textContent).toContain("Motion is off: layout changes are instantaneous and no pulses are drawn.");
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

  it("counts the reduced rendering in telegrams, in English and German (AR21 finding 4)", () => {
    expect(translateFor("en", "flow.reduced", { bundled: 80, dropped: 20 })).toBe(
      "Reduced rendering: 80 telegrams were drawn as bundled pulses, 20 not at all or only in part. Values and counts are complete.",
    );
    expect(translateFor("de", "flow.reduced", { bundled: 80, dropped: 20 })).toBe(
      "Reduzierte Darstellung: 80 Telegramme wurden als gebündelte Impulse gezeichnet, 20 gar nicht oder nur teilweise. Werte und Zählungen sind vollständig.",
    );
  });

  it("draws a fresh edge at full emphasis", async () => {
    await render(model());
    expect((host!.querySelector(".flow-edge") as SVGGElement).style.opacity).toBe("1");
  });
});

it.each(["en", "de"] as const)("%s flag tables use Flag/Value headings", async language => {
  saveUiLanguage(settingsStorage, language); resetUiLanguageForTests(); await render(model()); await key(node("Dimmer"), "Enter");
  expect([...host!.querySelectorAll(".flow-flags thead th")].map(th => th.querySelector("span")!.textContent)).toEqual(["Flag", language === "de" ? "Wert" : "Value"]);
});

it.each(["en", "de"] as const)("%s inspector owns one reset across multiple connection flag tables", async language => {
  saveUiLanguage(settingsStorage, language); resetUiLanguageForTests();
  await render(model(m => { admitRows(m, [row(2, { source: "1.1.3", sourceRaw: 0x1103 })], flowNow()); }));
  await key(node("Dimmer"), "Enter");
  const tables = host!.querySelectorAll('[data-table-id="flow-flags"]');
  expect(tables.length).toBeGreaterThan(1);
  expect([...tables].every(table => !table.querySelector(".table-column-reset"))).toBe(true);
  const name = language === "de" ? "Spaltenbreiten zurücksetzen: Objektflags" : "Reset column widths: Object flags";
  const resets = host!.querySelectorAll<HTMLButtonElement>(`button[aria-label="${name}"]`);
  expect(resets).toHaveLength(1);
  expect(resets[0].textContent).toBe(name);
  const first = tables[0].querySelector<HTMLElement>('[role="separator"]')!;
  const handles = [...tables].map(table => table.querySelector<HTMLElement>('[role="separator"]')!);
  const initial = first.getAttribute("aria-valuenow");
  await key(first, "ArrowRight");
  expect(first.getAttribute("aria-valuenow")).not.toBe(initial);
  expect(handles.every(handle => handle.getAttribute("aria-valuenow") === first.getAttribute("aria-valuenow"))).toBe(true);
  await act(async () => resets[0].click());
  expect(first.getAttribute("aria-valuenow")).toBe(initial);
  expect(handles.every(handle => handle.getAttribute("aria-valuenow") === initial)).toBe(true);
});

tableColumnSmoke([["flow-flags", 2], ["flow-values", 5]], async () => {
  await render(model()); await key(node("Dimmer"), "Enter"); return { host: host! };
});
