/** Pins the File menu's submenu behaviour: ARIA roles, keyboard paths, hover intent, placement. */
// @vitest-environment happy-dom
import { act, type ReactNode } from "react";
import { createPortal } from "react-dom";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import {
  choosePanelPlacement, choosePanelShift, FileMenuContent, FileMenuItem, FileMenuSummary, FileSubmenu, chooseMenuFit,
  layoutGeometry, menuGeometry, MIN_PANEL_WIDTH_PX, useFileMenuItemProps,
} from "./FileMenu";

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
const submenu = (name: string) => host!.querySelector<HTMLElement>(`[data-file-submenu="${name}"]`)!;
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

  it("keeps a submenu open while a pointer press moves focus to another row, so the click lands on that row", async () => {
    await mount();
    await click(trigger("exchange"), 1);
    trigger("exchange").focus();
    // A mouse press focuses its target between pointerdown and click. Folding the open (inline) panel
    // there would move the pressed row away from under the pointer and lose the click.
    await act(async () => {
      trigger("history").dispatchEvent(new PointerEvent("pointerdown", { bubbles: true, pointerType: "mouse" }));
      trigger("history").focus();
    });
    expect(expanded("exchange")).toBe("true");
    await act(async () => { window.dispatchEvent(new PointerEvent("pointerup", { pointerType: "mouse" })); });
    await click(trigger("history"), 1);
    expect(expanded("history")).toBe("true");
    expect(expanded("exchange")).toBe("false");
    // Without a press in flight, focus moving to another row still folds the submenu (keyboard path).
    await act(async () => { item("Quit").focus(); });
    expect(expanded("history")).toBe("false");
  });

  // Chromium's tap path (touch, and pen without hover) sends the pointer events first and the
  // compatibility mouse events after `pointerup`: the focus move happens in that later mousedown.
  for (const pointerType of ["touch", "pen"] as const) {
    it(`keeps a submenu open through a ${pointerType} tap whose mousedown follows pointerup, so the tap lands on the row`, async () => {
      await mount();
      await click(trigger("exchange"), 1);
      trigger("exchange").focus();
      const history = trigger("history");
      await act(async () => {
        history.dispatchEvent(new PointerEvent("pointerdown", { bubbles: true, pointerType }));
        window.dispatchEvent(new PointerEvent("pointerup", { pointerType }));
        history.dispatchEvent(new MouseEvent("mousedown", { bubbles: true, cancelable: true }));
        history.focus();
      });
      expect(expanded("exchange"), "the compatibility mousedown's focus move does not fold").toBe("true");
      await act(async () => { window.dispatchEvent(new MouseEvent("mouseup")); });
      await click(history, 1);
      expect(expanded("history")).toBe("true");
      expect(expanded("exchange")).toBe("false");
      // After the tap a focus move to another row is a keyboard move again and folds.
      await act(async () => { item("Quit").focus(); });
      expect(expanded("history")).toBe("false");
    });

    it(`keeps a submenu open through a ${pointerType} press in mouse order (mousedown before pointerup)`, async () => {
      await mount();
      await click(trigger("exchange"), 1);
      trigger("exchange").focus();
      const history = trigger("history");
      await act(async () => {
        history.dispatchEvent(new PointerEvent("pointerdown", { bubbles: true, pointerType }));
        history.dispatchEvent(new MouseEvent("mousedown", { bubbles: true, cancelable: true }));
        history.focus();
      });
      expect(expanded("exchange")).toBe("true");
      await act(async () => {
        window.dispatchEvent(new PointerEvent("pointerup", { pointerType }));
        window.dispatchEvent(new MouseEvent("mouseup"));
      });
      await click(history, 1);
      expect(expanded("history")).toBe("true");
    });
  }

  it("forgets a press abandoned outside the window, so a later focus move folds again", async () => {
    await mount();
    await click(trigger("exchange"), 1);
    trigger("exchange").focus();
    await act(async () => {
      trigger("history").dispatchEvent(new PointerEvent("pointerdown", { bubbles: true, pointerType: "mouse" }));
      // The button is released over another window: no pointerup, no mouseup reach this page.
      window.dispatchEvent(new Event("blur"));
    });
    await act(async () => { item("Quit").focus(); });
    expect(expanded("exchange")).toBe("false");
  });

  it("forgets a cancelled touch press (a scroll took it over)", async () => {
    await mount();
    await click(trigger("exchange"), 1);
    trigger("exchange").focus();
    await act(async () => {
      trigger("history").dispatchEvent(new PointerEvent("pointerdown", { bubbles: true, pointerType: "touch" }));
      window.dispatchEvent(new PointerEvent("pointercancel", { pointerType: "touch" }));
    });
    await act(async () => { item("Quit").focus(); });
    expect(expanded("exchange")).toBe("false");
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

/** The stylesheet's two layouts, as far as the menu can see them: a flyout is absolutely positioned. */
function layoutStyle(position: "absolute" | "static") {
  const style = document.createElement("style");
  style.textContent = `.file-submenu-panel { position: ${position}; }`;
  document.head.append(style);
  return style;
}

describe("File menu hover intent", () => {
  let layout: HTMLStyleElement;
  beforeEach(() => { layout = layoutStyle("absolute"); });
  afterEach(() => layout.remove());

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

describe("File menu hover intent in an inline layout", () => {
  let layout: HTMLStyleElement | undefined;
  afterEach(() => { layout?.remove(); layout = undefined; window.innerWidth = 1024; });

  it("opens nothing on hover where the stylesheet makes the panels inline", async () => {
    layout = layoutStyle("static");
    await mount();
    await over(trigger("history"));
    await advance(600);
    expect(expanded("history")).toBe("false");
    await over(trigger("exchange"));
    await advance(600);
    expect(expanded("exchange")).toBe("false");
  });

  it("keeps a clicked submenu open under a hover on a sibling row, and a click on the sibling swaps them", async () => {
    layout = layoutStyle("static");
    await mount();
    await click(trigger("exchange"), 1);
    await over(trigger("history"));
    await advance(600);
    expect(expanded("exchange")).toBe("true");
    expect(expanded("history")).toBe("false");
    await click(trigger("history"), 1);
    expect(expanded("history")).toBe("true");
    expect(expanded("exchange")).toBe("false");
    await over(trigger("help"));
    await advance(600);
    expect(expanded("history")).toBe("true");
    expect(expanded("help")).toBe("false");
  });

  it("still opens and closes by keyboard", async () => {
    layout = layoutStyle("static");
    await mount();
    await key(trigger("history"), "ArrowRight");
    expect(expanded("history")).toBe("true");
    await key(document.activeElement!, "ArrowLeft");
    expect(expanded("history")).toBe("false");
  });

  it("opens nothing on hover where no side keeps the panel's minimum width (per-instance inline)", async () => {
    layout = layoutStyle("absolute");
    window.innerWidth = 600;
    const spy = vi.spyOn(HTMLElement.prototype, "getBoundingClientRect").mockImplementation(function (this: HTMLElement) {
      const box = (left: number, top: number, width: number, height: number) =>
        ({ left, top, width, height, right: left + width, bottom: top + height, x: left, y: top, toJSON() {} }) as DOMRect;
      if (this.dataset.fileSubmenuPanel) return box(396, 100, 240, 120);
      if (this.classList.contains("file-submenu")) return box(100, 100, 290, 34);
      return box(0, 0, 0, 0);
    });
    await mount();
    await over(trigger("exchange"));
    await advance(600);
    expect(expanded("exchange")).toBe("false");
    expect(panel("exchange").hidden).toBe(true);
    // The probe leaves nothing behind: no layout mark on the row, no stray unhidden panel.
    expect(host!.querySelector("[data-layout]")).toBeNull();
    await click(trigger("exchange"), 1);
    expect(expanded("exchange")).toBe("true");
    await over(trigger("history"));
    await advance(600);
    expect(expanded("exchange")).toBe("true");
    expect(expanded("history")).toBe("false");
    spy.mockRestore();
  });

  it("a flyout still opens on hover where the panel fits beside its row", async () => {
    layout = layoutStyle("absolute");
    await mount();
    await over(trigger("history"));
    await advance(200);
    expect(expanded("history")).toBe("true");
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
      if (this.classList.contains("file-submenu")) return rect(300, 100, 134, 34);
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
      if (this.classList.contains("file-submenu")) return rect(100, 100, 334, 34);
      return rect(0, 0, 0, 0);
    });
    await key(trigger("exchange"), "ArrowRight");
    expect(panel("exchange").dataset.side ?? "right").toBe("right");
    spy.mockRestore();
    style.remove();
  });

  it("gives up width, down to the shared minimum, instead of leaving the screen when neither side fits the whole panel", async () => {
    const style = stylePanelsAbsolute();
    window.innerWidth = 640;
    await mount();
    const spy = vi.spyOn(HTMLElement.prototype, "getBoundingClientRect").mockImplementation(function (this: HTMLElement) {
      if (this.dataset.fileSubmenuPanel) return rect(396, 100, 240, 120);
      if (this.classList.contains("file-submenu")) return rect(100, 100, 290, 34);
      return rect(0, 0, 0, 0);
    });
    await key(trigger("exchange"), "ArrowRight");
    expect(panel("exchange").dataset.side).toBeUndefined();
    // 640 - 8 - 396 = 236 px of room, above the 220 px minimum.
    expect(panel("exchange").style.getPropertyValue("--file-submenu-max-width")).toBe("236px");
    expect(submenu("exchange").dataset.layout).toBeUndefined();
    spy.mockRestore();
    style.remove();
  });

  it("caps to the room beside the row while the entry animation still holds the panel 6 px to the side", async () => {
    const style = stylePanelsAbsolute();
    window.innerWidth = 640;
    await mount();
    const spy = vi.spyOn(HTMLElement.prototype, "getBoundingClientRect").mockImplementation(function (this: HTMLElement) {
      // At open the slide-in starts at translateX(-6px): the panel's box reads 390, its place is 396.
      if (this.dataset.fileSubmenuPanel) return rect(390, 100, 240, 120);
      if (this.classList.contains("file-submenu")) return rect(100, 100, 290, 34);
      return rect(0, 0, 0, 0);
    });
    await key(trigger("exchange"), "ArrowRight");
    expect(panel("exchange").style.getPropertyValue("--file-submenu-max-width")).toBe("236px");
    spy.mockRestore();
    style.remove();
  });

  it("moves a capped panel up by the height it has once its entries wrap", async () => {
    const style = stylePanelsAbsolute();
    window.innerWidth = 640;
    window.innerHeight = 300;
    await mount();
    const spy = vi.spyOn(HTMLElement.prototype, "getBoundingClientRect").mockImplementation(function (this: HTMLElement) {
      if (this.dataset.fileSubmenuPanel) {
        // 240 px wide it fits (100..220); capped to 236 px two entries wrap and it runs to 320.
        return this.style.getPropertyValue("--file-submenu-max-width") ? rect(396, 100, 236, 220) : rect(396, 100, 240, 120);
      }
      if (this.classList.contains("file-submenu")) return rect(100, 109, 290, 34);
      return rect(0, 0, 0, 0);
    });
    await key(trigger("exchange"), "ArrowRight");
    expect(panel("exchange").style.getPropertyValue("--file-submenu-max-width")).toBe("236px");
    // 320 - (300 - 8) = 28 px too low.
    expect(panel("exchange").style.getPropertyValue("--file-submenu-shift")).toBe("-28px");
    spy.mockRestore();
    style.remove();
  });

  it("expands inline, never narrower than the minimum, when neither side has the minimum", async () => {
    const style = stylePanelsAbsolute();
    window.innerWidth = 600;
    await mount();
    const spy = vi.spyOn(HTMLElement.prototype, "getBoundingClientRect").mockImplementation(function (this: HTMLElement) {
      if (this.dataset.fileSubmenuPanel) return rect(396, 100, 240, 120);
      if (this.classList.contains("file-submenu")) return rect(100, 100, 290, 34);
      return rect(0, 0, 0, 0);
    });
    await key(trigger("exchange"), "ArrowRight");
    // 600 - 8 - 396 = 196 px on the right, 100 - 6 - 8 = 86 px on the left: both below 220 px.
    expect(submenu("exchange").dataset.layout).toBe("inline");
    expect(panel("exchange").dataset.side).toBeUndefined();
    expect(panel("exchange").style.getPropertyValue("--file-submenu-max-width")).toBe("");
    spy.mockRestore();
    style.remove();
  });

  it("states its minimum width on the panel, the one value the stylesheet reads", async () => {
    await mount();
    expect(panel("exchange").style.getPropertyValue("--file-submenu-min-width")).toBe("220px");
  });

  it("leaves inline mode when the window grows wide enough again, and when the panel closes", async () => {
    const style = stylePanelsAbsolute();
    window.innerWidth = 600;
    await mount();
    const spy = vi.spyOn(HTMLElement.prototype, "getBoundingClientRect").mockImplementation(function (this: HTMLElement) {
      if (this.dataset.fileSubmenuPanel) return rect(396, 100, 240, 120);
      if (this.classList.contains("file-submenu")) return rect(100, 100, 290, 34);
      return rect(0, 0, 0, 0);
    });
    await key(trigger("exchange"), "ArrowRight");
    expect(submenu("exchange").dataset.layout).toBe("inline");
    window.innerWidth = 1200;
    await act(async () => { window.dispatchEvent(new Event("resize")); });
    expect(submenu("exchange").dataset.layout).toBeUndefined();
    window.innerWidth = 600;
    await act(async () => { window.dispatchEvent(new Event("resize")); });
    expect(submenu("exchange").dataset.layout).toBe("inline");
    await key(trigger("exchange"), "Escape");
    expect(submenu("exchange").dataset.layout).toBeUndefined();
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
      if (this.classList.contains("file-submenu")) return rect(100, 209, 334, 34);
      return rect(0, 0, 0, 0);
    });
    await key(trigger("exchange"), "ArrowRight");
    // 350 - (300 - 8) = 58 px too low.
    expect(panel("exchange").style.getPropertyValue("--file-submenu-shift")).toBe("-58px");
    spy.mockRestore();
    style.remove();
  });

  describe("under application zoom", () => {
    // The root zoom scales DOMRect readings and the window size (visual px) but not the stylesheet's
    // lengths, the 220 px minimum, the gap and the margin (CSS layout px). Each case below is one
    // layout geometry, a 200 px row at x = 100 with a 240 px panel at x = 306, given to the menu as
    // the visual numbers the browser would report at that scale.
    const zoomed = (scale: number, layoutViewportWidth: number, layoutViewportHeight = 800, panelTop = 100, panelHeight = 120) => {
      document.documentElement.style.setProperty("--app-ui-scale", String(scale));
      window.innerWidth = layoutViewportWidth * scale;
      window.innerHeight = layoutViewportHeight * scale;
      return vi.spyOn(HTMLElement.prototype, "getBoundingClientRect").mockImplementation(function (this: HTMLElement) {
        if (this.dataset.fileSubmenuPanel) {
          const left = this.dataset.side === "left" ? 100 - 6 - 240 : 306;
          return rect(left * scale, panelTop * scale, 240 * scale, panelHeight * scale);
        }
        if (this.classList.contains("file-submenu")) return rect(100 * scale, (panelTop + 9) * scale, 200 * scale, 34 * scale);
        return rect(0, 0, 0, 0);
      });
    };
    const px = (value: string) => Number.parseFloat(value);
    afterEach(() => { document.documentElement.style.removeProperty("--app-ui-scale"); });

    it("at 150% goes inline where the visual side room passes 220 px but the layout room does not", async () => {
      const style = stylePanelsAbsolute();
      await mount();
      // 520 layout px wide: 520 - 8 - 306 = 206 px on the right (309 visual), 86 px on the left.
      const spy = zoomed(1.5, 520);
      await key(trigger("exchange"), "ArrowRight");
      expect(submenu("exchange").dataset.layout).toBe("inline");
      expect(panel("exchange").style.getPropertyValue("--file-submenu-max-width")).toBe("");
      spy.mockRestore();
      style.remove();
    });

    it("at 150% writes the width cap in layout px: the room on the side, not its visual size", async () => {
      const style = stylePanelsAbsolute();
      await mount();
      // 550 layout px wide: 236 px on the right (354 visual) for a 240 px panel, 86 px on the left.
      const spy = zoomed(1.5, 550);
      await key(trigger("exchange"), "ArrowRight");
      expect(submenu("exchange").dataset.layout).toBeUndefined();
      expect(panel("exchange").dataset.side).toBeUndefined();
      expect(px(panel("exchange").style.getPropertyValue("--file-submenu-max-width"))).toBeCloseTo(236, 2);
      spy.mockRestore();
      style.remove();
    });

    it("at 80% keeps the flyout and caps it where the layout room still holds the minimum", async () => {
      const style = stylePanelsAbsolute();
      await mount();
      // 540 layout px wide: 226 px on the right (181 visual, below 220 visual px), 86 px on the left.
      const spy = zoomed(0.8, 540);
      await key(trigger("exchange"), "ArrowRight");
      expect(submenu("exchange").dataset.layout).toBeUndefined();
      expect(px(panel("exchange").style.getPropertyValue("--file-submenu-max-width"))).toBeCloseTo(226, 2);
      spy.mockRestore();
      style.remove();
    });

    for (const scale of [1.5, 0.8]) {
      it(`at ${scale * 100}% moves a panel up by its layout overflow`, async () => {
        const style = stylePanelsAbsolute();
        await mount();
        // 400 layout px high; the panel spans 200..450, 450 - (400 - 8) = 58 px too low.
        const spy = zoomed(scale, 1200, 400, 200, 250);
        await key(trigger("exchange"), "ArrowRight");
        expect(px(panel("exchange").style.getPropertyValue("--file-submenu-shift"))).toBeCloseTo(-58, 2);
        spy.mockRestore();
        style.remove();
      });
    }

    it("places an open panel again when the application scale changes", async () => {
      const style = stylePanelsAbsolute();
      await mount();
      let spy = zoomed(1, 520);
      await key(trigger("exchange"), "ArrowRight");
      // 520 px at 100%: 206 px on the right, inline.
      expect(submenu("exchange").dataset.layout).toBe("inline");
      spy.mockRestore();
      // The user zooms out to 80% with the submenu open: the same window is 650 layout px wide.
      spy = zoomed(0.8, 650);
      await act(async () => { document.documentElement.style.setProperty("--app-ui-scale", "0.8"); });
      expect(submenu("exchange").dataset.layout).toBeUndefined();
      expect(panel("exchange").style.getPropertyValue("--file-submenu-max-width")).toBe("");
      spy.mockRestore();
      style.remove();
    });
  });

  describe("first-level fit (UISET3-01)", () => {
    // The menu's own box, in visual px at `scale`: a 280 x `height` layout px menu at (120, 60), or
    // at `openHeight` while a submenu is unfolded.
    const menuAt = (scale: number, layoutViewport: { width: number; height: number }, height: number, openHeight = height) => {
      document.documentElement.style.setProperty("--app-ui-scale", String(scale));
      window.innerWidth = layoutViewport.width * scale;
      window.innerHeight = layoutViewport.height * scale;
      return vi.spyOn(HTMLElement.prototype, "getBoundingClientRect").mockImplementation(function (this: HTMLElement) {
        // An open inline submenu (happy-dom lays every panel out statically) makes the menu taller
        // by its own height; the fit is the first level's, without it.
        if (this.dataset.fileSubmenuPanel) return rect(0, 0, 0, this.hidden ? 0 : (openHeight - height) * scale);
        // File's `<details>`, whose left edge an anchored menu starts at.
        if (this.tagName === "DETAILS") return rect(120 * scale, 20 * scale, 80 * scale, 32 * scale);
        if (!this.classList.contains("file-menu-content")) return rect(0, 0, 0, 0);
        const unfolded = [...this.querySelectorAll<HTMLElement>("[data-file-submenu-panel]")].some((p) => !p.hidden);
        // The menu's own shift moves it, as `left: var(--file-menu-shift)` does.
        const shift = Number.parseFloat(this.style.getPropertyValue("--file-menu-shift")) || 0;
        return rect((120 + shift) * scale, 60 * scale, 280 * scale, (unfolded ? openHeight : height) * scale);
      });
    };
    afterEach(() => { document.documentElement.style.removeProperty("--app-ui-scale"); });

    it("at 150% scrolls the whole menu and folds its submenus inline when the first level runs past the bottom", async () => {
      // 1440 x 601 at 150%: 960 x 400.67 layout px; a 340 px first level at y = 60 ends at 400 > 392.67.
      const spy = menuAt(1.5, { width: 960, height: 601 / 1.5 }, 340);
      await mount();
      expect(content().dataset.layout).toBe("inline");
      spy.mockRestore();
    });

    it("keeps flyouts where the same first level fits: at 100% that window is 601 px high", async () => {
      const spy = menuAt(1, { width: 1440, height: 601 }, 340);
      await mount();
      expect(content().dataset.layout).toBeUndefined();
      spy.mockRestore();
    });

    it("decides again on a zoom step and on a window resize, and drops the mark on close", async () => {
      let spy = menuAt(1, { width: 1440, height: 601 }, 340);
      await mount();
      expect(content().dataset.layout).toBeUndefined();
      spy.mockRestore();
      // Ctrl+Plus to 150%: the same window is now 400.67 layout px high.
      spy = menuAt(1.5, { width: 960, height: 601 / 1.5 }, 340);
      await act(async () => { document.documentElement.style.setProperty("--app-ui-scale", "1.5"); });
      expect(content().dataset.layout).toBe("inline");
      spy.mockRestore();
      // The window grows to 1440 x 900 (600 layout px high): the first level fits again.
      spy = menuAt(1.5, { width: 960, height: 600 }, 340);
      await act(async () => { window.dispatchEvent(new Event("resize")); });
      expect(content().dataset.layout).toBeUndefined();
      spy.mockRestore();
      spy = menuAt(1.5, { width: 960, height: 601 / 1.5 }, 340);
      await act(async () => { window.dispatchEvent(new Event("resize")); });
      expect(content().dataset.layout).toBe("inline");
      await act(async () => { details().open = false; });
      expect(content().dataset.layout).toBeUndefined();
      spy.mockRestore();
    });

    it("at 150% moves a first level that runs past the right margin left by its layout overflow", async () => {
      // 570 x 800 visual px at 150% are 380 layout px wide: the menu ends at 400, 28 px past 372.
      const style = document.createElement("style");
      style.textContent = ".file-menu-content { position: absolute; }";
      document.head.append(style);
      const spy = menuAt(1.5, { width: 380, height: 800 }, 300);
      await mount();
      expect(content().style.getPropertyValue("--file-menu-shift")).toBe("-28px");
      expect(content().dataset.layout).toBeUndefined();
      // Measured from File's box, not from the shifted menu: a refit finds the same shift.
      await act(async () => { window.dispatchEvent(new Event("resize")); });
      expect(content().style.getPropertyValue("--file-menu-shift")).toBe("-28px");
      await act(async () => { details().open = false; });
      expect(content().style.getPropertyValue("--file-menu-shift")).toBe("");
      spy.mockRestore();
      style.remove();
    });

    it("measures the first level with its submenus folded, so an open inline submenu does not count", async () => {
      // The first level ends at 360 of 800 layout px; with a submenu open inline the menu would end at 960.
      const spy = menuAt(1.5, { width: 960, height: 800 }, 300, 900);
      await mount();
      await key(trigger("exchange"), "ArrowRight");
      expect(panel("exchange").hidden).toBe(false);
      await act(async () => { window.dispatchEvent(new Event("resize")); });
      expect(content().dataset.layout).toBeUndefined();
      expect(panel("exchange").hidden).toBe(false);
      spy.mockRestore();
    });
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
        if (this.classList.contains("file-submenu")) return rect(100, 209, 334, 34);
        return rect(0, 0, 0, 0);
      });
      await key(trigger("history"), "ArrowRight");
      expect(panel("history").style.getPropertyValue("--file-submenu-shift")).toBe("");
      const observer = FakeResizeObserver.instances.find((o) => o.observed.includes(panel("history")))!;
      expect(observer).toBeDefined();
      // The panel grows while open (late entries, a font swap) and now passes the bottom edge.
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

describe("choosePanelPlacement", () => {
  // The panel is 240 px wide, its row starts at x = 100, the panel at x = 396 (row 200 px + gap).
  const at = (viewportWidth: number, anchorLeft = 100, panelWidth = 240) =>
    choosePanelPlacement({ viewportWidth, panelLeft: anchorLeft + 296, panelWidth, anchorLeft });

  it("keeps the right side while the whole panel fits", () => {
    expect(at(1200)).toEqual({ kind: "flyout", side: "right", maxWidth: null });
    expect(at(644)).toEqual({ kind: "flyout", side: "right", maxWidth: null });
  });

  it("flips left when the whole panel fits there and not on the right", () => {
    expect(at(600, 300)).toEqual({ kind: "flyout", side: "left", maxWidth: null });
  });

  it("caps the width on the side with room for the minimum", () => {
    // 640 - 8 - 396 = 236 px right, 100 - 6 - 8 = 86 px left.
    expect(at(640)).toEqual({ kind: "flyout", side: "right", maxWidth: 236 });
    // Right has 120 px, left has 260 - 6 - 8 = 246 px but the whole panel (300 px) does not fit.
    expect(choosePanelPlacement({ viewportWidth: 524, panelLeft: 396, panelWidth: 300, anchorLeft: 260 }))
      .toEqual({ kind: "flyout", side: "left", maxWidth: 246 });
  });

  it("offers exactly the minimum width and not a pixel less", () => {
    expect(MIN_PANEL_WIDTH_PX).toBe(220);
    // 396 + 220 + 8 = 624 px viewport: exactly 220 px of room on the right.
    expect(at(624)).toEqual({ kind: "flyout", side: "right", maxWidth: 220 });
    expect(at(623)).toEqual({ kind: "inline" });
  });

  it("goes inline when neither side keeps the minimum", () => {
    expect(at(600)).toEqual({ kind: "inline" });
    expect(at(430)).toEqual({ kind: "inline" });
  });
});

describe("choosePanelShift", () => {
  it("moves a panel up by its overflow past the bottom margin, never above the top margin", () => {
    expect(choosePanelShift({ viewportHeight: 400, panelTop: 200, panelBottom: 300 })).toBe(0);
    expect(choosePanelShift({ viewportHeight: 400, panelTop: 200, panelBottom: 450 })).toBe(58);
    // Taller than the room above: it stops at the top margin and scrolls inside itself.
    expect(choosePanelShift({ viewportHeight: 400, panelTop: 50, panelBottom: 600 })).toBe(42);
  });
});

describe("layoutGeometry", () => {
  // One layout geometry read at 150%: a 200 px row at x = 100 (so its panel starts at 300 + 6) and
  // a 240 x 250 px panel at y = 200.
  const panel = { top: 300, width: 360, height: 375 };
  const row = { left: 150, right: 450 };

  it("divides every visual reading by the application scale", () => {
    expect(layoutGeometry(panel, row, { width: 780, height: 600 }, 1.5)).toEqual({
      viewportWidth: 520, viewportHeight: 400, panelLeft: 306, panelWidth: 240, anchorLeft: 100, panelTop: 200, panelBottom: 450,
    });
  });

  it("feeds placement in one unit: 309 visual px of room at 150% are 206 layout px, below the minimum", () => {
    const geometry = layoutGeometry(panel, row, { width: 780, height: 600 }, 1.5);
    expect(choosePanelPlacement(geometry)).toEqual({ kind: "inline" });
    // The same window read without the conversion would wrongly pass the 220 px minimum.
    expect(choosePanelPlacement({ viewportWidth: 780, panelLeft: 459, panelWidth: 360, anchorLeft: 150 }))
      .toEqual({ kind: "flyout", side: "right", maxWidth: 313 });
    expect(choosePanelShift(geometry)).toBe(58);
  });

  it("at 80% gives back the room the visual reading hides: 181 visual px are 226 layout px", () => {
    const geometry = layoutGeometry({ top: 80, width: 192, height: 96 }, { left: 80, right: 240 }, { width: 432, height: 640 }, 0.8);
    const placement = choosePanelPlacement(geometry);
    expect(placement.kind === "flyout" && placement.side).toBe("right");
    expect(placement.kind === "flyout" ? placement.maxWidth : null).toBeCloseTo(226, 6);
  });

  it("is the identity at 100%", () => {
    expect(layoutGeometry({ top: 100, width: 240, height: 120 }, { left: 100, right: 390 }, { width: 640, height: 800 }, 1)).toEqual({
      viewportWidth: 640, viewportHeight: 800, panelLeft: 396, panelWidth: 240, anchorLeft: 100, panelTop: 100, panelBottom: 220,
    });
  });
});

describe("chooseMenuFit", () => {
  const fit = (input: Partial<Parameters<typeof chooseMenuFit>[0]>) =>
    chooseMenuFit({ viewportWidth: 960, viewportHeight: 400, menuLeft: 120, menuRight: 400, menuBottom: 300, ...input });

  it("takes the inline layout past the bottom margin and not a pixel before", () => {
    expect(fit({ menuBottom: 392 })).toEqual({ inline: false, shift: 0 });
    expect(fit({ menuBottom: 392.5 })).toEqual({ inline: true, shift: 0 });
  });

  it("moves left by its overflow past the right margin, never past the left margin", () => {
    expect(fit({ viewportWidth: 440, menuRight: 432 })).toEqual({ inline: false, shift: 0 });
    expect(fit({ viewportWidth: 440, menuRight: 461 })).toEqual({ inline: false, shift: 29 });
    // Wider than the room: it stops at the left margin.
    expect(fit({ viewportWidth: 300, menuLeft: 40, menuRight: 400 })).toEqual({ inline: false, shift: 32 });
  });
});

describe("menuGeometry", () => {
  it("divides the menu's DOMRect and the window by the scale: 601 visual px at 150% are 400.67 layout px", () => {
    const geometry = menuGeometry({ left: 180, right: 600, bottom: 593 }, { width: 1440, height: 601 }, 1.5);
    expect(geometry.viewportWidth).toBe(960);
    expect(geometry.viewportHeight).toBeCloseTo(400.67, 2);
    expect(geometry).toMatchObject({ menuLeft: 120, menuRight: 400 });
    expect(geometry.menuBottom).toBeCloseTo(395.33, 2);
    // Read unconverted, the same menu would pass (593 <= 601 - 8); converted it does not (395.33 > 392.67).
    expect(chooseMenuFit({ viewportWidth: 1440, viewportHeight: 601, menuLeft: 180, menuRight: 600, menuBottom: 593 }).inline).toBe(false);
    expect(chooseMenuFit(geometry).inline).toBe(true);
  });

  it("writes the shift in layout px: at 150% a 660 px window is 440 layout px wide", () => {
    // A menu from 255 to 691.5 visual px is 170..461 layout px: 29 layout px past the right margin.
    expect(chooseMenuFit(menuGeometry({ left: 255, right: 691.5, bottom: 600 }, { width: 660, height: 800 }, 1.5)).shift).toBeCloseTo(29, 6);
  });

  it("at 80% gives back the room the visual reading hides", () => {
    // 640 visual px at 80% are 800 layout px; a menu ending at 620 visual px ends at 775.
    expect(chooseMenuFit(menuGeometry({ left: 100, right: 400, bottom: 620 }, { width: 900, height: 640 }, 0.8))).toEqual({ inline: false, shift: 0 });
  });
});
