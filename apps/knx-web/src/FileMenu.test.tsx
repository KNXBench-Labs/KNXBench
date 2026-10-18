/** Pins the File menu's submenu behaviour: ARIA roles, keyboard paths, hover intent, placement. */
// @vitest-environment happy-dom
import { act, type ReactNode } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { FileMenuContent, FileSubmenu } from "./FileMenu";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

let host: HTMLDivElement | undefined;
let root: Root | undefined;

beforeEach(() => vi.useFakeTimers());
afterEach(async () => {
  vi.useRealTimers();
  if (root) await act(async () => root!.unmount());
  root = undefined;
  host?.remove();
  host = undefined;
});

const handlers = () => ({
  newProject: vi.fn(), open: vi.fn(), exportProject: vi.fn(), importCsv: vi.fn(), disabledImport: vi.fn(),
  history: vi.fn(), compare: vi.fn(), debug: vi.fn(), about: vi.fn(), quit: vi.fn(), summaryEscape: vi.fn(),
});
type Handlers = ReturnType<typeof handlers>;

function Menu({ h, extra }: { h: Handlers; extra?: ReactNode }) {
  return (
    <details className="file-menu" open onKeyDown={(e) => { if (e.key === "Escape") h.summaryEscape(); }}>
      <summary>File</summary>
      <FileMenuContent>
        <button onClick={h.newProject}>New project</button>
        <button onClick={h.open} disabled>Open project</button>
        <div role="separator" />
        <FileSubmenu id="exchange" label="Import & export">
          <button disabled onClick={h.disabledImport}>Import disabled</button>
          <button onClick={h.exportProject}>Export project</button>
          <button onClick={h.importCsv}>Import CSV</button>
        </FileSubmenu>
        <FileSubmenu id="history" label="History">
          <button onClick={h.history}>Project history</button>
          <button onClick={h.compare}>Compare with</button>
          {extra}
        </FileSubmenu>
        <FileSubmenu id="help" label="Help">
          <button onClick={h.debug}>Debug report</button>
          <button onClick={h.about}>About</button>
        </FileSubmenu>
        <button onClick={h.quit}>Quit</button>
      </FileMenuContent>
    </details>
  );
}

async function mount(h: Handlers = handlers(), extra?: ReactNode) {
  host = document.createElement("div");
  document.body.append(host);
  root = createRoot(host);
  await act(async () => root!.render(<Menu h={h} extra={extra} />));
  return h;
}

const trigger = (name: string) => host!.querySelector<HTMLButtonElement>(`[data-file-submenu-trigger="${name}"]`)!;
const panel = (name: string) => host!.querySelector<HTMLElement>(`[data-file-submenu-panel="${name}"]`)!;
const item = (text: string) => [...host!.querySelectorAll<HTMLButtonElement>("button")].find((b) => b.textContent === text)!;
const content = () => host!.querySelector<HTMLElement>(".file-menu-content")!;
const details = () => host!.querySelector<HTMLDetailsElement>("details")!;

async function key(target: Element, k: string) {
  await act(async () => { target.dispatchEvent(new KeyboardEvent("keydown", { key: k, bubbles: true, cancelable: true })); });
}
async function click(target: Element, detail = 1) {
  await act(async () => { target.dispatchEvent(new MouseEvent("click", { bubbles: true, cancelable: true, detail })); });
}
async function over(target: Element, pointerType = "mouse") {
  await act(async () => { target.dispatchEvent(new PointerEvent("pointerover", { bubbles: true, pointerType })); });
}
async function advance(ms: number) {
  await act(async () => { vi.advanceTimersByTime(ms); });
}
const expanded = (name: string) => trigger(name).getAttribute("aria-expanded");

describe("File menu roles", () => {
  it("exposes a menu whose submenu triggers announce a popup and start collapsed", async () => {
    await mount();
    expect(content().getAttribute("role")).toBe("menu");
    for (const name of ["exchange", "history", "help"]) {
      const t = trigger(name);
      expect(t.getAttribute("role")).toBe("menuitem");
      expect(t.getAttribute("aria-haspopup")).toBe("menu");
      expect(t.getAttribute("aria-expanded")).toBe("false");
      expect(t.getAttribute("aria-controls")).toBe(panel(name).id);
      expect(panel(name).getAttribute("role")).toBe("menu");
      expect(panel(name).getAttribute("aria-labelledby")).toBe(t.id);
      expect(panel(name).hidden).toBe(true);
    }
    expect(trigger("exchange").textContent).toContain("Import & export");
  });

  it("tags plain buttons as menuitems, including ones that appear later, and leaves report groups alone", async () => {
    await mount(handlers(), <div role="group" aria-label="report"><button>Report close</button></div>);
    expect(item("New project").getAttribute("role")).toBe("menuitem");
    expect(item("Export project").getAttribute("role")).toBe("menuitem");
    expect(item("Report close").getAttribute("role")).toBeNull();
    const late = document.createElement("button");
    late.textContent = "Late";
    panel("help").append(late);
    await act(async () => { await Promise.resolve(); });
    expect(late.getAttribute("role")).toBe("menuitem");
  });

  it("keeps the items of a closed submenu mounted so their dialogs and reports survive", async () => {
    await mount();
    expect(panel("history").contains(item("Compare with"))).toBe(true);
    expect(panel("history").hidden).toBe(true);
  });
});

describe("File menu keyboard", () => {
  it("opens a submenu with ArrowRight and focuses its first enabled item", async () => {
    await mount();
    trigger("exchange").focus();
    await key(trigger("exchange"), "ArrowRight");
    expect(expanded("exchange")).toBe("true");
    expect(panel("exchange").hidden).toBe(false);
    expect(document.activeElement).toBe(item("Export project"));
  });

  it.each([["Enter"], ["Space"]])("opens a submenu with %s (keyboard click) and focuses its first item", async () => {
    await mount();
    trigger("history").focus();
    await click(trigger("history"), 0);
    expect(expanded("history")).toBe("true");
    expect(document.activeElement).toBe(item("Project history"));
  });

  it("keeps the focused submenu open when Enter is pressed on an already open trigger", async () => {
    await mount();
    await click(trigger("history"), 0);
    trigger("history").focus();
    await click(trigger("history"), 0);
    expect(expanded("history")).toBe("true");
    expect(document.activeElement).toBe(item("Project history"));
  });

  it("opens with the pointer without stealing focus and closes on the second click", async () => {
    await mount();
    trigger("help").focus();
    await click(trigger("help"), 1);
    expect(expanded("help")).toBe("true");
    expect(document.activeElement).toBe(trigger("help"));
    await click(trigger("help"), 1);
    expect(expanded("help")).toBe("false");
  });

  it("moves with ArrowDown/ArrowUp inside a submenu, wraps, and skips disabled items", async () => {
    await mount();
    await key(trigger("exchange"), "ArrowRight");
    expect(document.activeElement).toBe(item("Export project"));
    await key(document.activeElement!, "ArrowDown");
    expect(document.activeElement).toBe(item("Import CSV"));
    await key(document.activeElement!, "ArrowDown");
    expect(document.activeElement).toBe(item("Export project"));
    await key(document.activeElement!, "ArrowUp");
    expect(document.activeElement).toBe(item("Import CSV"));
  });

  it("jumps with Home and End inside a submenu", async () => {
    await mount();
    await key(trigger("history"), "ArrowRight");
    await key(document.activeElement!, "End");
    expect(document.activeElement).toBe(item("Compare with"));
    await key(document.activeElement!, "Home");
    expect(document.activeElement).toBe(item("Project history"));
  });

  it("walks the top level with arrows over enabled items and triggers, and Home/End reach the ends", async () => {
    await mount();
    item("New project").focus();
    await key(document.activeElement!, "ArrowDown");
    // "Open project" is disabled and skipped.
    expect(document.activeElement).toBe(trigger("exchange"));
    await key(document.activeElement!, "ArrowDown");
    expect(document.activeElement).toBe(trigger("history"));
    await key(document.activeElement!, "End");
    expect(document.activeElement).toBe(item("Quit"));
    await key(document.activeElement!, "ArrowDown");
    expect(document.activeElement).toBe(item("New project"));
    await key(document.activeElement!, "ArrowUp");
    expect(document.activeElement).toBe(item("Quit"));
    await key(document.activeElement!, "Home");
    expect(document.activeElement).toBe(item("New project"));
  });

  it("closes with ArrowLeft and returns focus to the parent item", async () => {
    await mount();
    await key(trigger("help"), "ArrowRight");
    await key(document.activeElement!, "ArrowLeft");
    expect(expanded("help")).toBe("false");
    expect(panel("help").hidden).toBe(true);
    expect(document.activeElement).toBe(trigger("help"));
  });

  it("closes the submenu on Escape, returns focus to the parent, and only the next Escape reaches the menu", async () => {
    const h = await mount();
    await key(trigger("help"), "ArrowRight");
    await key(document.activeElement!, "Escape");
    expect(expanded("help")).toBe("false");
    expect(document.activeElement).toBe(trigger("help"));
    expect(h.summaryEscape).not.toHaveBeenCalled();
    await key(document.activeElement!, "Escape");
    expect(h.summaryEscape).toHaveBeenCalledTimes(1);
  });

  it("closes an open submenu on Escape from its parent row when no item took focus", async () => {
    const h = await mount();
    trigger("exchange").focus();
    await click(trigger("exchange"), 1);
    expect(expanded("exchange")).toBe("true");
    await key(trigger("exchange"), "Escape");
    expect(expanded("exchange")).toBe("false");
    expect(document.activeElement).toBe(trigger("exchange"));
    expect(h.summaryEscape).not.toHaveBeenCalled();
    await key(trigger("exchange"), "Escape");
    expect(h.summaryEscape).toHaveBeenCalledTimes(1);
  });

  it("ignores ArrowRight on a plain item and ArrowLeft at the top level", async () => {
    await mount();
    item("New project").focus();
    await key(item("New project"), "ArrowRight");
    await key(item("New project"), "ArrowLeft");
    expect(document.activeElement).toBe(item("New project"));
    for (const name of ["exchange", "history", "help"]) expect(expanded(name)).toBe("false");
  });

  it("closes a pinned submenu when focus moves to another item of the menu", async () => {
    await mount();
    await key(trigger("help"), "ArrowRight");
    const quit = item("Quit");
    await act(async () => { quit.focus(); });
    expect(expanded("help")).toBe("false");
  });

  it("keeps a submenu open when focus leaves the menu entirely", async () => {
    await mount();
    await key(trigger("help"), "ArrowRight");
    const outside = document.createElement("button");
    document.body.append(outside);
    await act(async () => { outside.focus(); });
    expect(expanded("help")).toBe("true");
    outside.remove();
  });
});

describe("File menu activation", () => {
  it("calls each original handler exactly once and no other", async () => {
    const h = await mount();
    for (const [text, fn] of [
      ["New project", h.newProject], ["Export project", h.exportProject], ["Import CSV", h.importCsv],
      ["Project history", h.history], ["Compare with", h.compare], ["Debug report", h.debug],
      ["About", h.about], ["Quit", h.quit],
    ] as const) {
      await click(item(text));
      expect(fn, text).toHaveBeenCalledTimes(1);
    }
    expect(h.open).not.toHaveBeenCalled();
    expect(h.disabledImport).not.toHaveBeenCalled();
  });

  it("does not activate any item when a trigger is opened by click, key or hover", async () => {
    const h = await mount();
    await click(trigger("exchange"));
    await click(trigger("history"), 0);
    await key(trigger("help"), "ArrowRight");
    await over(trigger("exchange"));
    await advance(500);
    for (const fn of Object.values(h)) expect(fn).not.toHaveBeenCalled();
  });

  it("does not run a disabled item, by click or by arrow navigation", async () => {
    const h = await mount();
    await key(trigger("exchange"), "ArrowRight");
    expect(document.activeElement).not.toBe(item("Import disabled"));
    await click(item("Import disabled"));
    expect(h.disabledImport).not.toHaveBeenCalled();
    expect(item("Import disabled").disabled).toBe(true);
  });

  it("keeps the activation container's own click handler informed of trigger clicks", async () => {
    // The trigger must carry the opt-out that `App` uses to keep the menu open.
    await mount();
    expect(trigger("exchange").hasAttribute("data-menu-stays-open")).toBe(true);
  });
});

describe("File menu siblings and reset", () => {
  it("closes the sibling when another submenu opens, by keyboard and by pointer", async () => {
    await mount();
    await key(trigger("exchange"), "ArrowRight");
    await click(trigger("help"), 1);
    expect(expanded("exchange")).toBe("false");
    expect(expanded("help")).toBe("true");
    await key(trigger("history"), "ArrowRight");
    expect(expanded("help")).toBe("false");
    expect(expanded("history")).toBe("true");
  });

  it("collapses every submenu when the menu closes", async () => {
    await mount();
    await key(trigger("help"), "ArrowRight");
    await act(async () => { details().open = false; await Promise.resolve(); });
    await act(async () => { details().open = true; await Promise.resolve(); });
    expect(expanded("help")).toBe("false");
    expect(panel("help").hidden).toBe(true);
  });
});

describe("File menu hover intent", () => {
  it("opens after a short intent delay and not before", async () => {
    await mount();
    await over(trigger("history"));
    await advance(60);
    expect(expanded("history")).toBe("false");
    await advance(100);
    expect(expanded("history")).toBe("true");
    // Hover opens never move focus.
    expect(document.activeElement).not.toBe(item("Project history"));
  });

  it("opens the hovered sibling and closes the previous one", async () => {
    await mount();
    await over(trigger("exchange"));
    await advance(200);
    await over(trigger("help"));
    await advance(200);
    expect(expanded("exchange")).toBe("false");
    expect(expanded("help")).toBe("true");
  });

  it("does not open on touch or pen hover", async () => {
    await mount();
    await over(trigger("history"), "touch");
    await over(trigger("history"), "pen");
    await advance(500);
    expect(expanded("history")).toBe("false");
  });

  it("keeps the submenu through a diagonal move: leaving the trigger and entering the panel within the grace delay", async () => {
    await mount();
    await over(trigger("exchange"));
    await advance(200);
    await over(content());
    await advance(100);
    expect(expanded("exchange")).toBe("true");
    await over(item("Export project"));
    await advance(1000);
    expect(expanded("exchange")).toBe("true");
  });

  it("does not switch to a sibling the pointer only crosses on its way into the panel", async () => {
    await mount();
    await over(trigger("exchange"));
    await advance(200);
    await over(trigger("history"));
    await advance(40);
    await over(item("Export project"));
    await advance(1000);
    expect(expanded("exchange")).toBe("true");
    expect(expanded("history")).toBe("false");
  });

  it("closes a hover-opened submenu after the grace delay once the pointer rests on another item", async () => {
    await mount();
    await over(trigger("exchange"));
    await advance(200);
    await over(item("Quit"));
    await advance(100);
    expect(expanded("exchange")).toBe("true");
    await advance(500);
    expect(expanded("exchange")).toBe("false");
  });

  it("does not close a click-pinned submenu when the pointer wanders off", async () => {
    await mount();
    await click(trigger("exchange"), 1);
    await over(item("Quit"));
    await advance(1000);
    expect(expanded("exchange")).toBe("true");
  });

  it("promotes a hover-opened submenu to pinned when its trigger is clicked", async () => {
    await mount();
    await over(trigger("exchange"));
    await advance(200);
    await click(trigger("exchange"), 1);
    expect(expanded("exchange")).toBe("true");
    await over(item("Quit"));
    await advance(1000);
    expect(expanded("exchange")).toBe("true");
  });
});

describe("File menu placement", () => {
  function rect(left: number, top: number, width: number, height: number): DOMRect {
    return { left, top, width, height, right: left + width, bottom: top + height, x: left, y: top, toJSON() {} } as DOMRect;
  }
  function stylePanelsAbsolute() {
    const style = document.createElement("style");
    style.textContent = ".file-submenu-panel { position: absolute; }";
    document.head.append(style);
    return style;
  }

  it("flips to the left when the right side has no room and the left does", async () => {
    const style = stylePanelsAbsolute();
    window.innerWidth = 600;
    window.innerHeight = 800;
    await mount();
    const spy = vi.spyOn(HTMLElement.prototype, "getBoundingClientRect").mockImplementation(function (this: HTMLElement) {
      if (this.dataset.fileSubmenuPanel) return this.dataset.side === "left" ? rect(100, 100, 240, 120) : rect(440, 100, 240, 120);
      if (this.classList.contains("file-submenu")) return rect(300, 100, 200, 34);
      return rect(0, 0, 0, 0);
    });
    await key(trigger("exchange"), "ArrowRight");
    expect(panel("exchange").dataset.side).toBe("left");
    spy.mockRestore();
    style.remove();
  });

  it("stays on the right when it fits", async () => {
    const style = stylePanelsAbsolute();
    window.innerWidth = 1200;
    window.innerHeight = 800;
    await mount();
    const spy = vi.spyOn(HTMLElement.prototype, "getBoundingClientRect").mockImplementation(function (this: HTMLElement) {
      if (this.dataset.fileSubmenuPanel) return rect(440, 100, 240, 120);
      if (this.classList.contains("file-submenu")) return rect(100, 100, 200, 34);
      return rect(0, 0, 0, 0);
    });
    await key(trigger("exchange"), "ArrowRight");
    expect(panel("exchange").dataset.side ?? "right").toBe("right");
    spy.mockRestore();
    style.remove();
  });

  it("gives up width instead of leaving the screen when neither side has room", async () => {
    const style = stylePanelsAbsolute();
    window.innerWidth = 500;
    await mount();
    const spy = vi.spyOn(HTMLElement.prototype, "getBoundingClientRect").mockImplementation(function (this: HTMLElement) {
      if (this.dataset.fileSubmenuPanel) return rect(396, 100, 240, 120);
      if (this.classList.contains("file-submenu")) return rect(100, 100, 200, 34);
      return rect(0, 0, 0, 0);
    });
    await key(trigger("exchange"), "ArrowRight");
    expect(panel("exchange").dataset.side).toBeUndefined();
    expect(panel("exchange").style.getPropertyValue("--file-submenu-max-width")).toBe("160px");
    spy.mockRestore();
    style.remove();
  });

  it("moves a panel up when it would run past the bottom edge", async () => {
    const style = stylePanelsAbsolute();
    window.innerWidth = 1200;
    window.innerHeight = 300;
    await mount();
    const spy = vi.spyOn(HTMLElement.prototype, "getBoundingClientRect").mockImplementation(function (this: HTMLElement) {
      if (this.dataset.fileSubmenuPanel) return rect(440, 200, 240, 150);
      if (this.classList.contains("file-submenu")) return rect(100, 209, 200, 34);
      return rect(0, 0, 0, 0);
    });
    await key(trigger("exchange"), "ArrowRight");
    // 350 - (300 - 8) = 58 px too low.
    expect(panel("exchange").style.getPropertyValue("--file-submenu-shift")).toBe("-58px");
    spy.mockRestore();
    style.remove();
  });

  it("does not place the panel when the stylesheet makes it an inline expansion", async () => {
    window.innerWidth = 300;
    await mount();
    const spy = vi.spyOn(HTMLElement.prototype, "getBoundingClientRect").mockImplementation(function (this: HTMLElement) {
      return this.dataset.fileSubmenuPanel ? rect(500, 100, 240, 120) : rect(0, 0, 200, 34);
    });
    await key(trigger("exchange"), "ArrowRight");
    expect(panel("exchange").dataset.side).toBeUndefined();
    spy.mockRestore();
  });
});
