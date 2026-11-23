/** Runs the shared menu checklist (WP-49.3) against the File menu with App's open/close wiring. */
// @vitest-environment happy-dom
import { act, useEffect, useRef } from "react";
import { createRoot, type Root } from "react-dom/client";
import { describe, expect, it } from "vitest";
import { FileMenuContent, FileMenuItem, FileMenuSummary, FileSubmenu } from "./FileMenu";
import { describeMenuChecklist, type MenuAdapter } from "./test/menuChecklist";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

let host: HTMLDivElement | undefined;
let root: Root | undefined;
let runs = 0;
const ran = () => { runs += 1; };

/**
 * The File control as `App` wires it: a closed `<details>` whose Escape closes it and returns focus
 * to the summary, and a document `pointerdown` outside it that closes it (App.tsx, F1). Those two
 * handlers live in `App`; `App.test.tsx` pins them against the real component.
 */
function AppFileMenu({ onParentKey }: { onParentKey: () => void }) {
  const ref = useRef<HTMLDetailsElement | null>(null);
  useEffect(() => {
    const down = (e: PointerEvent) => {
      const menu = ref.current;
      if (!menu?.open) return;
      if (e.target instanceof Node && menu.contains(e.target)) return;
      menu.open = false;
    };
    document.addEventListener("pointerdown", down);
    return () => document.removeEventListener("pointerdown", down);
  }, []);
  return (
    <div onKeyDown={onParentKey}>
      <details ref={ref} className="file-menu"
        onKeyDown={(e) => { if (e.key === "Escape") { e.currentTarget.open = false; e.currentTarget.querySelector("summary")?.focus(); } }}>
        <FileMenuSummary>File</FileMenuSummary>
        <FileMenuContent>
          <FileMenuItem disabled onClick={ran}>Disabled first</FileMenuItem>
          <FileMenuItem onClick={ran}>New project</FileMenuItem>
          <FileMenuItem onClick={ran}>Open project</FileMenuItem>
          <div role="separator" />
          <FileSubmenu id="exchange" label="Import & export">
            <FileMenuItem disabled onClick={ran}>Import disabled</FileMenuItem>
            <FileMenuItem onClick={ran}>Export project</FileMenuItem>
            <FileMenuItem onClick={ran}>Import CSV</FileMenuItem>
          </FileSubmenu>
          <FileMenuItem onClick={ran}>Quit</FileMenuItem>
        </FileMenuContent>
      </details>
    </div>
  );
}

const summary = () => host!.querySelector<HTMLElement>("summary")!;
const details = () => host!.querySelector<HTMLDetailsElement>("details")!;
const level = () => host!.querySelector<HTMLElement>(".file-menu-content")!;
const declared = (el: HTMLElement) => [...el.querySelectorAll<HTMLElement>("[data-file-menu-item]")]
  .filter(item => item.closest('[role="menu"]') === el);
const pointerDown = (target: Element) => act(async () => { target.dispatchEvent(new PointerEvent("pointerdown", { bubbles: true })); });

const adapter: MenuAdapter = {
  name: "FileMenu",
  async mount(parentKeyDown) {
    runs = 0;
    host = document.createElement("div");
    document.body.append(host);
    root = createRoot(host);
    await act(async () => root!.render(<AppFileMenu onParentKey={parentKeyDown} />));
  },
  async unmount() {
    await act(async () => root!.unmount());
    host!.remove();
    host = undefined;
    root = undefined;
  },
  trigger: summary,
  async open() {
    summary().focus();
    await act(async () => { summary().dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowDown", bubbles: true, cancelable: true })); });
  },
  menu: () => (details().open ? level() : null),
  items: () => declared(level()),
  activations: () => runs,
  pressInside: () => pointerDown(summary()),
  pressOutside: () => pointerDown(document.body),
  popupTrigger: "menu-button",
  menuName: "unnamed",
  escape: "propagates",
  tab: "continues",
  submenu: {
    trigger: () => host!.querySelector<HTMLElement>('[data-file-submenu-trigger="exchange"]')!,
    panel: () => host!.querySelector<HTMLElement>('[data-file-submenu-panel="exchange"]')!,
    items: () => declared(host!.querySelector<HTMLElement>('[data-file-submenu-panel="exchange"]')!),
  },
};

describeMenuChecklist(adapter);

describe("FileMenu: current menu policy pins", () => {
  it("opens from Enter or Space on the summary onto the first enabled entry, and ArrowUp onto the last", async () => {
    await adapter.mount(() => {});
    try {
      summary().focus();
      await act(async () => { summary().dispatchEvent(new MouseEvent("click", { bubbles: true, detail: 0 })); details().open = true; });
      expect(document.activeElement).toBe(declared(level())[1]);
      details().open = false;
      summary().focus();
      await act(async () => { summary().dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowUp", bubbles: true, cancelable: true })); });
      expect(document.activeElement).toBe(declared(level()).at(-1));
    } finally { await adapter.unmount(); }
  });
});
