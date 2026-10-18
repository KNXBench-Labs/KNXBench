/** Pins the File menu's submenu behaviour: ARIA roles, keyboard paths, hover intent, placement. */
// @vitest-environment happy-dom
import { act, type ReactNode } from "react";
import { createPortal } from "react-dom";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { FileMenuContent, FileMenuItem, FileMenuSummary, FileSubmenu, useFileMenuItemProps } from "./FileMenu";

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

/** A child component that owns its button, like `DebugReportButton`, and declares it through the hook. */
function ChildOwnedEntry({ onClick }: { onClick: () => void }) {
  const entry = useFileMenuItemProps();
  return <button {...entry} onClick={onClick}>Child owned</button>;
}

function Menu({ h, extra }: { h: Handlers; extra?: ReactNode }) {
  return (
    <details className="file-menu" open onKeyDown={(e) => { if (e.key === "Escape") h.summaryEscape(); }}>
      <FileMenuSummary>File</FileMenuSummary>
      <FileMenuContent>
        <FileMenuItem onClick={h.newProject}>New project</FileMenuItem>
        <FileMenuItem onClick={h.open} disabled>Open project</FileMenuItem>
        <div role="separator" />
        <FileSubmenu id="exchange" label="Import & export">
          <FileMenuItem disabled onClick={h.disabledImport}>Import disabled</FileMenuItem>
          <FileMenuItem onClick={h.exportProject}>Export project</FileMenuItem>
          <FileMenuItem onClick={h.importCsv}>Import CSV</FileMenuItem>
        </FileSubmenu>
        <FileSubmenu id="history" label="History">
          <FileMenuItem onClick={h.history}>Project history</FileMenuItem>
          <FileMenuItem onClick={h.compare}>Compare with</FileMenuItem>
          {extra}
        </FileSubmenu>
        <FileSubmenu id="help" label="Help">
          <ChildOwnedEntry onClick={h.debug} />
          <FileMenuItem onClick={h.about}>About</FileMenuItem>
        </FileSubmenu>
        <FileMenuItem onClick={h.quit}>Quit</FileMenuItem>
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

  it("declares entries explicitly: components and hook users are menuitems, undeclared buttons are not", async () => {
    await mount(handlers(), <div role="group" aria-label="stray"><button>Stray button</button></div>);
    expect(item("New project").getAttribute("role")).toBe("menuitem");
    expect(item("Export project").getAttribute("role")).toBe("menuitem");
    expect(item("Child owned").getAttribute("role")).toBe("menuitem");
    expect(item("Stray button").getAttribute("role")).toBeNull();
    expect(item("Stray button").hasAttribute("data-file-menu-item")).toBe(false);
    // Nothing tags a button behind the menu's back: a late plain button stays a plain button.
    const late = document.createElement("button");
    late.textContent = "Late";
    panel("help").append(late);
    await act(async () => { await Promise.resolve(); });
    expect(late.getAttribute("role")).toBeNull();
  });

  it("hands out no item props outside a File menu", async () => {
    function Probe() {
      return <button data-probe {...useFileMenuItemProps()}>x</button>;
    }
    host = document.createElement("div");
    document.body.append(host);
    root = createRoot(host);
    await act(async () => root!.render(<Probe />));
    expect(host.querySelector("[data-probe]")!.getAttribute("role")).toBeNull();
  });

  it("makes the File control a menu button that reports whether its menu is open", async () => {
    await mount();
    const summary = host!.querySelector("summary")!;
    expect(summary.getAttribute("aria-haspopup")).toBe("menu");
    expect(summary.getAttribute("aria-expanded")).toBe("true");
    await act(async () => { details().open = false; await Promise.resolve(); });
    expect(summary.getAttribute("aria-expanded")).toBe("false");
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
      ["Project history", h.history], ["Compare with", h.compare], ["Child owned", h.debug],
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

describe("File menu focus management", () => {
  const summary = () => host!.querySelector<HTMLElement>("summary")!;
  const tabIndexes = (scope: Element) => [...scope.querySelectorAll<HTMLElement>("[data-file-menu-item]")]
    .filter((el) => el.closest('[role="menu"]') === scope)
    .map((el) => `${el.textContent}:${el.tabIndex}`);

  it("keeps one tab stop per level: the first enabled entry, then wherever focus went", async () => {
    await mount();
    expect(tabIndexes(content())).toEqual([
      "New project:0", "Open project:-1", "Import & export›:-1", "History›:-1", "Help›:-1", "Quit:-1",
    ]);
    expect(tabIndexes(panel("exchange"))).toEqual(["Import disabled:-1", "Export project:0", "Import CSV:-1"]);
    item("Quit").focus();
    await key(item("Quit"), "ArrowUp");
    expect(tabIndexes(content()).filter((s) => s.endsWith(":0"))).toEqual(["Help›:0"]);
    trigger("exchange").focus();
    await key(trigger("exchange"), "ArrowRight");
    expect(tabIndexes(panel("exchange")).filter((s) => s.endsWith(":0"))).toEqual(["Export project:0"]);
    expect(tabIndexes(content()).filter((s) => s.endsWith(":0"))).toEqual(["Import & export›:0"]);
  });

  it("includes a declared entry that appears later in the roving set", async () => {
    function Late({ show }: { show: boolean }) {
      return <Menu h={handlers()} extra={show ? <FileMenuItem>Late entry</FileMenuItem> : null} />;
    }
    host = document.createElement("div");
    document.body.append(host);
    root = createRoot(host);
    await act(async () => root!.render(<Late show={false} />));
    await act(async () => root!.render(<Late show />));
    expect(item("Late entry").tabIndex).toBe(-1);
    expect(item("Late entry").getAttribute("role")).toBe("menuitem");
  });

  it.each([["Tab", false], ["Tab", true]])("%s (shift %s) on a top-level entry closes the menu, focus goes to File and the key is not swallowed", async (_k, shift) => {
    await mount();
    item("New project").focus();
    const e = new KeyboardEvent("keydown", { key: "Tab", shiftKey: shift, bubbles: true, cancelable: true });
    await act(async () => { item("New project").dispatchEvent(e); });
    expect(details().open).toBe(false);
    expect(document.activeElement).toBe(summary());
    expect(e.defaultPrevented).toBe(false);
  });

  it("Tab inside an open submenu closes the whole menu, not just the submenu", async () => {
    await mount();
    await key(trigger("exchange"), "ArrowRight");
    const e = new KeyboardEvent("keydown", { key: "Tab", bubbles: true, cancelable: true });
    await act(async () => { document.activeElement!.dispatchEvent(e); });
    expect(details().open).toBe(false);
    expect(expanded("exchange")).toBe("false");
    expect(document.activeElement).toBe(summary());
    expect(e.defaultPrevented).toBe(false);
  });

  const strayControls = (
    <div role="group" aria-label="stray">
      <button>Stray button</button>
      <input aria-label="Stray input" />
      <a href="#stray">Stray link</a>
      <span tabIndex={0}>Stray stop</span>
      <button tabIndex={-1}>Stray programmatic</button>
    </div>
  );

  it.each([
    ["button", "Stray button"], ["input", "Stray input"], ["link", "Stray link"], ["tabindex stop", "Stray stop"],
    ["programmatic stop", "Stray programmatic"],
  ])("Tab and Shift+Tab from any focusable descendant of an open submenu close the menu (%s)", async (_kind, name) => {
    await mount(handlers(), strayControls);
    const find = () => [...host!.querySelectorAll<HTMLElement>("button, input, a, span")]
      .find((el) => el.textContent === name || el.getAttribute("aria-label") === name)!;
    for (const shiftKey of [false, true]) {
      await act(async () => { details().open = true; await Promise.resolve(); });
      await key(trigger("history"), "ArrowRight");
      find().focus();
      const e = new KeyboardEvent("keydown", { key: "Tab", shiftKey, bubbles: true, cancelable: true });
      await act(async () => { find().dispatchEvent(e); });
      expect(details().open).toBe(false);
      expect(expanded("history")).toBe("false");
      expect(document.activeElement).toBe(summary());
      expect(e.defaultPrevented).toBe(false);
    }
  });

  it("leaves Tab alone in a dialog a menu entry renders elsewhere in the document", async () => {
    const dialog = createPortal(<div role="dialog"><button>Dialog button</button></div>, document.body);
    await mount(handlers(), dialog);
    await key(trigger("history"), "ArrowRight");
    const inDialog = document.body.querySelector<HTMLElement>('[role="dialog"] button')!;
    inDialog.focus();
    const e = new KeyboardEvent("keydown", { key: "Tab", bubbles: true, cancelable: true });
    await act(async () => { inDialog.dispatchEvent(e); });
    expect(details().open).toBe(true);
    expect(e.defaultPrevented).toBe(false);
  });

  it("ArrowDown on the File control opens its menu and focuses the first entry", async () => {
    await mount();
    await act(async () => { details().open = false; await Promise.resolve(); });
    summary().focus();
    await key(summary(), "ArrowDown");
    expect(details().open).toBe(true);
    expect(document.activeElement).toBe(item("New project"));
  });

  it("ArrowUp on the File control opens its menu and focuses the last entry", async () => {
    await mount();
    await act(async () => { details().open = false; await Promise.resolve(); });
    summary().focus();
    await key(summary(), "ArrowUp");
    expect(details().open).toBe(true);
    expect(document.activeElement).toBe(item("Quit"));
  });

  it("arrow keys skip an undeclared control inside a submenu", async () => {
    await mount(handlers(), <div role="group" aria-label="stray"><button>Stray button</button></div>);
    await key(trigger("history"), "ArrowRight");
    await key(document.activeElement!, "End");
    expect(document.activeElement).toBe(item("Compare with"));
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

  describe("with a ResizeObserver", () => {
    class FakeResizeObserver {
      static instances: FakeResizeObserver[] = [];
      observed: Element[] = [];
      disconnected = false;
      constructor(readonly callback: ResizeObserverCallback) { FakeResizeObserver.instances.push(this); }
      observe(el: Element) { this.observed.push(el); }
      unobserve() {}
      disconnect() { this.disconnected = true; }
      fire() { this.callback([], this as unknown as ResizeObserver); }
    }
    beforeEach(() => { FakeResizeObserver.instances = []; vi.stubGlobal("ResizeObserver", FakeResizeObserver); });
    afterEach(() => vi.unstubAllGlobals());

    it("places the panel again when its rendered size changes, and disconnects on close", async () => {
      const style = stylePanelsAbsolute();
      window.innerWidth = 1200;
      window.innerHeight = 400;
      await mount();
      let height = 100;
      const spy = vi.spyOn(HTMLElement.prototype, "getBoundingClientRect").mockImplementation(function (this: HTMLElement) {
        if (this.dataset.fileSubmenuPanel) return rect(440, 200, 240, height);
        if (this.classList.contains("file-submenu")) return rect(100, 209, 200, 34);
        return rect(0, 0, 0, 0);
      });
      await key(trigger("history"), "ArrowRight");
      expect(panel("history").style.getPropertyValue("--file-submenu-shift")).toBe("");
      const observer = FakeResizeObserver.instances.find((o) => o.observed.includes(panel("history")))!;
      expect(observer).toBeDefined();
      // The comparison report arrives and the panel grows past the bottom edge.
      height = 300;
      await act(async () => observer.fire());
      await advance(50);
      expect(panel("history").style.getPropertyValue("--file-submenu-shift")).toBe("-108px");
      await key(document.activeElement!, "Escape");
      expect(observer.disconnected).toBe(true);
      spy.mockRestore();
      style.remove();
    });

    it("disconnects when the menu unmounts with a panel open", async () => {
      await mount();
      await key(trigger("help"), "ArrowRight");
      const observer = FakeResizeObserver.instances.find((o) => o.observed.includes(panel("help")))!;
      await act(async () => root!.unmount());
      root = undefined;
      expect(observer.disconnected).toBe(true);
    });
  });
});
