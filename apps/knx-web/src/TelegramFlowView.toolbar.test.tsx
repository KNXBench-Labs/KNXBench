/** Pins the real TelegramFlowView toolbar against the shared toolbar checklist (WP-49.4b). */
// @vitest-environment happy-dom
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, describe, expect, it } from "vitest";
import { flowNow, type FlowFeed } from "./flowFeed";
import { admitRows, createFlowModel, provideContext, type FlowModel } from "./flowModel";
import { snapshotJson } from "./flowTestFixtures";
import { parseFlowSnapshot } from "./flowWire";
import { resetSettingsForTests } from "./settingsStore";
import { resetUiLanguageForTests } from "./uiLanguage";
import TelegramFlowView from "./TelegramFlowView";
import { press } from "./test/comboboxChecklist";
import { describeToolbarChecklist, type ToolbarAdapter } from "./test/toolbarChecklist";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const LIGHT = 0x0801;
let host: HTMLDivElement | undefined;
let root: Root | undefined;

function model(): FlowModel {
  const m = createFlowModel({ serverIncarnation: "inc-1", sessionId: 7 });
  provideContext(m, "1", parseFlowSnapshot(snapshotJson({
    devices: [{ deviceId: 1, installationId: 1, name: "Switch", individualAddressRaw: 0x1101 }],
    groups: [{ gaRaw: LIGHT, gaId: 10, installationId: 1, name: "Light", dpt: "1.001", members: [] }],
  })), flowNow());
  admitRows(m, [{ seq: 1, service: "GroupValueWrite", source: "1.1.1", destination: "1/0/1", decoded: { kind: "value", text: "v1" }, sourceRaw: 0x1101, destinationRaw: LIGHT, observedAgeMs: 0, flowGeneration: "1" }], flowNow());
  return m;
}

async function mount(m: FlowModel | null, parentKeyDown: () => void, onOpenWindow?: () => void) {
  const feed: FlowFeed = { model: m, version: 1, source: undefined, admit: () => {}, reset: () => {} };
  host = document.createElement("div");
  document.body.appendChild(host);
  root = createRoot(host);
  await act(async () => root!.render(<div onKeyDown={parentKeyDown}><TelegramFlowView feed={feed} onOpenWindow={onOpenWindow} /></div>));
}
async function unmount() {
  if (root) await act(async () => root!.unmount());
  root = undefined; host?.remove(); host = undefined;
  resetSettingsForTests(); resetUiLanguageForTests();
}
const toolbar = () => host!.querySelector<HTMLElement>(".flow-toolbar")!;
const controls = () => [...toolbar().querySelectorAll<HTMLElement>("button")];
const byText = (text: string) => controls().find(b => b.textContent === text)!;
const click = (text: string) => act(async () => byText(text).click());

const flowAdapter = (name: string, m: () => FlowModel | null, pressedNames: string[], onOpenWindow?: () => void): ToolbarAdapter => ({
  name,
  mount: parentKeyDown => mount(m(), parentKeyDown, onOpenWindow),
  unmount,
  toolbar,
  controls,
  // Maximize/Restore and the other toggles; matched by their caption in either state.
  toggles: () => controls().filter(b => pressedNames.includes(b.textContent ?? "")),
  disable: async () => { await click("Freeze layout"); return [byText("Rearrange layout")]; },
});

describeToolbarChecklist(flowAdapter("TelegramFlowView", model, ["Auto zoom", "Maximize view", "Restore view", "Freeze layout"]));
// The empty state (no model) shows a second toolbar with the same class, so it follows the same rules.
describeToolbarChecklist({
  ...flowAdapter("TelegramFlowView (empty state)", () => null, ["Maximize view", "Restore view"]),
  disable: undefined,
});
// The Open window button renders only when the host passes onOpenWindow (both states); run the checklist with it too.
describeToolbarChecklist(flowAdapter("TelegramFlowView (with Open window)", model, ["Auto zoom", "Maximize view", "Restore view", "Freeze layout"], () => {}));
describeToolbarChecklist({
  ...flowAdapter("TelegramFlowView (empty state, with Open window)", () => null, ["Maximize view", "Restore view"], () => {}),
  disable: undefined,
});

afterEach(unmount);

describe("TelegramFlowView toolbar: parent-owned behaviour", () => {
  it("empty state: Maximize starts aria-pressed=false and reads true once maximized", async () => {
    await mount(null, () => {});
    expect(byText("Maximize view").getAttribute("aria-pressed")).toBe("false");
    await click("Maximize view");
    expect(byText("Restore view").getAttribute("aria-pressed")).toBe("true");
  });

  for (const [state, m] of [["populated", model], ["empty", () => null]] as const) {
    it(`${state} state: Open in new window is a named, plain, tab-stop button that calls the host`, async () => {
      let opened = 0;
      await mount(m(), () => {}, () => { opened += 1; });
      const open = byText("Open in new window");
      expect(open, "Open in new window renders when the host provides onOpenWindow").toBeDefined();
      expect(open.tagName).toBe("BUTTON");
      expect((open as HTMLButtonElement).type).toBe("button");
      expect(open.tabIndex).toBe(0);
      expect(open.hasAttribute("tabindex")).toBe(false);
      expect(open.hasAttribute("aria-pressed")).toBe(false);
      await click("Open in new window");
      expect(opened).toBe(1);
    });
  }

  it("Open in new window is absent when the host passes no onOpenWindow", async () => {
    await mount(model(), () => {});
    expect(controls().some(b => b.textContent === "Open in new window")).toBe(false);
  });

  it("names every toggle's pressed state in the initial state: auto zoom on, others off", async () => {
    await mount(model(), () => {});
    const pressed = Object.fromEntries(controls().filter(b => b.hasAttribute("aria-pressed")).map(b => [b.textContent, b.getAttribute("aria-pressed")]));
    expect(pressed).toEqual({ "Auto zoom": "true", "Maximize view": "false", "Freeze layout": "false" });
    // The shortcut hint lives inside the toolbar as plain text, not as a control.
    expect(toolbar().querySelector(".flow-keys")?.tagName).toBe("SPAN");
  });

  it("leaves arrows on a toolbar button to the browser: the canvas handler is not an ancestor", async () => {
    await mount(model(), () => {});
    const node = host!.querySelector<SVGGElement>("g.flow-node")!;
    const before = node.getAttribute("tabindex");
    byText("Zoom in").focus();
    const event = await press(document.activeElement as HTMLElement, "ArrowRight");
    expect(event.defaultPrevented).toBe(false);
    expect(host!.querySelector("g.flow-node")!.getAttribute("tabindex")).toBe(before);
    expect(document.activeElement).toBe(byText("Zoom in"));
  });

  describe("maximized document listener (Tab and Escape)", () => {
    const section = () => host!.querySelector<HTMLElement>(".telegram-flow")!;
    const maximize = async (parentKeyDown: () => void) => {
      await mount(model(), parentKeyDown);
      byText("Maximize view").focus();
      await click("Maximize view");
      expect(section().classList.contains("flow-maximized")).toBe(true);
    };
    const listen = () => { const seen: KeyboardEvent[] = []; const on = (e: KeyboardEvent) => { seen.push(e); }; window.addEventListener("keydown", on); return { seen, stop: () => window.removeEventListener("keydown", on) }; };

    it("Escape on a toolbar button restores the view, is prevented and stopped, and returns focus to the opener", async () => {
      let parentKeys = 0;
      await maximize(() => { parentKeys += 1; });
      const win = listen();
      try {
        byText("Zoom in").focus();
        const event = await press(document.activeElement as HTMLElement, "Escape");
        expect(event.defaultPrevented).toBe(true);
        expect(win.seen).toHaveLength(0);
        expect(parentKeys).toBe(0);
        expect(section().classList.contains("flow-maximized")).toBe(false);
        expect(document.activeElement).toBe(byText("Maximize view"));
        expect(byText("Maximize view").getAttribute("aria-pressed")).toBe("false");
      } finally { win.stop(); }
    });

    it("Escape is left alone while not maximized (positive control: the listener exists only while maximized)", async () => {
      await mount(model(), () => {});
      const win = listen();
      try {
        byText("Zoom in").focus();
        const event = await press(document.activeElement as HTMLElement, "Escape");
        expect(event.defaultPrevented).toBe(false);
        expect(win.seen).toHaveLength(1);
      } finally { win.stop(); }
    });

    it("Tab from the last control wraps to the first and Shift+Tab from the first to the last while maximized", async () => {
      await maximize(() => {});
      const reachable = [...section().querySelectorAll<HTMLElement>("button:not(:disabled), [tabindex='0'], a[href], input:not(:disabled), select:not(:disabled)")].filter(el => el.getClientRects().length > 0);
      // happy-dom has no layout; without reachable controls the trap would pin nothing.
      expect(reachable.length, "the trap must see the toolbar buttons").toBeGreaterThan(2);
      const [first, last] = [reachable[0], reachable[reachable.length - 1]];
      expect(toolbar().contains(first)).toBe(true);
      last.focus();
      const forward = await press(last, "Tab");
      expect(forward.defaultPrevented).toBe(true);
      expect(document.activeElement).toBe(first);
      const backward = await press(first, "Tab", { shiftKey: true });
      expect(backward.defaultPrevented).toBe(true);
      expect(document.activeElement).toBe(last);
      // Positive control: a Tab in the middle is not intercepted.
      reachable[1].focus();
      expect((await press(reachable[1], "Tab")).defaultPrevented).toBe(false);
    });
  });
});
