/** Shared keyboard, focus and ARIA checklist (WP-49.4a) run by every hand-written tablist test. */
// SPDX-License-Identifier: AGPL-3.0-or-later
import { act } from "react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { press } from "./comboboxChecklist";

/** How one tablist is mounted and observed; the checklist itself never touches widget internals. */
export interface TabsAdapter {
  name: string;
  /** Mounts the REAL component inside a parent that calls `parentKeyDown` for every keydown reaching it. */
  mount(parentKeyDown: () => void): Promise<void>;
  unmount(): Promise<void>;
  tablist(): HTMLElement;
  /** Tabs in document order. */
  tabs(): HTMLElement[];
  /** Every `role=tabpanel` element of the widget in the document, hidden ones included. */
  panels(): HTMLElement[];
  /** `all`: every tab names its panel. `selected-only`: only the selected tab does (its panel is not always mounted). Decided contract (owner Q10, 23 Nov 2026): the bus monitor keeps `selected-only`. */
  controls: "all" | "selected-only";
  /** `present`: Home/End select the first/last tab. `absent`: they are ignored. Decided (owner Q9, 23 Nov 2026): both tablists handle Home/End. */
  homeEnd: "present" | "absent";
  /** `tab-stop`: every panel is itself focusable (tabIndex 0). `none`: panels are not tab stops. */
  panelStop: "tab-stop" | "none";
}

const selectedOf = (tabs: HTMLElement[]) => tabs.filter(t => t.getAttribute("aria-selected") === "true");

/** Registers the shared checklist cases for one tablist; widget-specific cases stay in its own file. */
export function describeTabsChecklist(adapter: TabsAdapter) {
  describe(`${adapter.name}: tabs checklist`, () => {
    let parentKeys = 0;
    let windowKeys = 0;
    const onWindowKey = () => { windowKeys += 1; };
    beforeEach(async () => {
      parentKeys = 0; windowKeys = 0;
      window.addEventListener("keydown", onWindowKey);
      await adapter.mount(() => { parentKeys += 1; });
    });
    afterEach(async () => {
      window.removeEventListener("keydown", onWindowKey);
      await adapter.unmount();
    });
    const focused = () => document.activeElement as HTMLElement;
    const activeIndex = () => adapter.tabs().findIndex(t => t.getAttribute("aria-selected") === "true");
    /** Moves focus onto the selected tab the way a user arrives at the tablist. */
    const focusActive = () => { adapter.tabs()[activeIndex()].focus(); };
    const invariants = (label: string) => {
      const tabs = adapter.tabs();
      expect(selectedOf(tabs), label).toHaveLength(1);
      expect(tabs.filter(t => t.tabIndex === 0), label).toEqual(selectedOf(tabs));
      for (const t of tabs) expect(t.tabIndex, label).toBe(t === selectedOf(tabs)[0] ? 0 : -1);
    };

    it("exposes a named tablist, tab roles and exactly one selected tab", () => {
      const list = adapter.tablist();
      expect(list.getAttribute("role")).toBe("tablist");
      expect(list.getAttribute("aria-label")?.trim()).toBeTruthy();
      const tabs = adapter.tabs();
      expect(tabs.length).toBeGreaterThanOrEqual(2);
      for (const t of tabs) {
        expect(t.getAttribute("role")).toBe("tab");
        expect(list.contains(t)).toBe(true);
        expect(["true", "false"]).toContain(t.getAttribute("aria-selected"));
      }
      expect(selectedOf(tabs)).toHaveLength(1);
    });

    /** Both-way tab/panel links for the CURRENT selection; run again after every selection change. */
    const checkLinks = (label: string) => {
      const tabs = adapter.tabs();
      const selected = selectedOf(tabs)[0];
      for (const t of tabs) {
        const controls = t.getAttribute("aria-controls");
        if (adapter.controls === "selected-only" && t !== selected) {
          expect(controls, `${label}: an unselected tab must not point at a missing panel`).toBeNull();
          continue;
        }
        expect(controls, `${label}: ${t.id}`).toBeTruthy();
        const panel = document.getElementById(controls!);
        expect(panel, `${label}: ${controls}`).not.toBeNull();
        expect(panel!.getAttribute("role"), label).toBe("tabpanel");
        expect(panel!.getAttribute("aria-labelledby"), `${label}: panel of ${t.id}`).toBe(t.id);
        expect(document.getElementById(t.id), label).toBe(t);
      }
      // Every panel is named by a tab of this list, and ids are unique.
      for (const panel of adapter.panels()) {
        const owner = document.getElementById(panel.getAttribute("aria-labelledby")!);
        expect(tabs, `${label}: ${panel.id}`).toContain(owner);
      }
      const ids = [...tabs, ...adapter.panels()].map(el => el.id);
      expect(new Set(ids).size, label).toBe(ids.length);
    };

    it("links every tab and panel both ways through aria-controls and aria-labelledby", () => {
      checkLinks("initial");
    });

    it("keeps the tab/panel links correct after selecting each tab in turn", async () => {
      const count = adapter.tabs().length;
      for (let i = 0; i < count; i++) {
        const target = adapter.tabs()[i];
        await act(async () => { target.click(); });
        expect(activeIndex()).toBe(i);
        checkLinks(`tab ${i} selected`);
      }
    });

    it("shows only the selected tab's panel", () => {
      const selected = selectedOf(adapter.tabs())[0];
      const visible = adapter.panels().filter(p => !p.hidden);
      expect(visible).toHaveLength(1);
      expect(visible[0].getAttribute("aria-labelledby")).toBe(selected.id);
    });

    it("keeps one roving tab stop on the selected tab and panels per the panel policy", () => {
      invariants("initial");
      for (const panel of adapter.panels()) {
        expect(panel.tabIndex, panel.id).toBe(adapter.panelStop === "tab-stop" ? 0 : -1);
      }
    });

    it("selects and focuses the next/previous tab on ArrowRight/ArrowLeft (automatic activation, wrapping — decided, owner Q9, 23 Nov 2026)", async () => {
      const count = adapter.tabs().length;
      focusActive();
      // Decided (owner Q9): arrows wrap and activate at once; the APG allows both.
      for (let i = 1; i <= count; i++) {
        const before = activeIndex();
        const event = await press(focused(), "ArrowRight");
        expect(event.defaultPrevented, "the page behind a tablist must not scroll").toBe(true);
        expect(activeIndex()).toBe((before + 1) % count);
        expect(focused()).toBe(adapter.tabs()[activeIndex()]);
        invariants(`ArrowRight ${i}`);
      }
      for (let i = 1; i <= count; i++) {
        const before = activeIndex();
        const event = await press(focused(), "ArrowLeft");
        expect(event.defaultPrevented).toBe(true);
        expect(activeIndex()).toBe((before - 1 + count) % count);
        expect(focused()).toBe(adapter.tabs()[activeIndex()]);
        invariants(`ArrowLeft ${i}`);
      }
      // The visible panel follows the selection.
      const visible = adapter.panels().filter(p => !p.hidden);
      expect(visible).toHaveLength(1);
      expect(visible[0].getAttribute("aria-labelledby")).toBe(adapter.tabs()[activeIndex()].id);
    });

    it("handles Home and End per the Home/End policy", async () => {
      const tabs = adapter.tabs();
      focusActive();
      await press(focused(), "ArrowRight");
      const start = activeIndex();
      const end = await press(focused(), "End");
      const afterEnd = activeIndex();
      const home = await press(focused(), "Home");
      if (adapter.homeEnd === "present") {
        expect(end.defaultPrevented).toBe(true);
        expect(afterEnd).toBe(tabs.length - 1);
        expect(home.defaultPrevented).toBe(true);
        expect(activeIndex()).toBe(0);
        expect(focused()).toBe(adapter.tabs()[0]);
        invariants("Home");
      } else {
        // Decided absent is no longer used by any adapter (owner Q9 made Home/End present everywhere).
        expect(end.defaultPrevented).toBe(false);
        expect(afterEnd).toBe(start);
        expect(home.defaultPrevented).toBe(false);
        expect(activeIndex()).toBe(start);
      }
    });

    it("leaves Alt/Ctrl/Meta-modified navigation keys to the browser, unhandled and unprevented (decided, owner Q11, 23 Nov 2026)", async () => {
      focusActive();
      const start = activeIndex();
      for (const key of ["ArrowLeft", "ArrowRight", "Home", "End"]) {
        for (const mods of [{ altKey: true }, { ctrlKey: true }, { metaKey: true }]) {
          const event = await press(focused(), key, mods);
          expect(event.defaultPrevented, `${key} ${JSON.stringify(mods)} must not be prevented (browser shortcut)`).toBe(false);
          expect(activeIndex(), `${key} ${JSON.stringify(mods)} must not change the selection`).toBe(start);
          expect(focused(), `${key} ${JSON.stringify(mods)} must not move focus`).toBe(adapter.tabs()[start]);
        }
      }
      // Positive control: the plain and Shift-modified keys are still handled.
      for (const mods of [{}, { shiftKey: true }]) {
        const before = activeIndex();
        const event = await press(focused(), "ArrowRight", mods);
        expect(event.defaultPrevented, `plain/Shift ArrowRight ${JSON.stringify(mods)} stays handled`).toBe(true);
        expect(activeIndex()).toBe((before + 1) % adapter.tabs().length);
      }
    });

    it("selects a tab on click and keeps the roving tab stop on it", async () => {
      const target = adapter.tabs().find(t => t.getAttribute("aria-selected") === "false")!;
      await act(async () => { target.click(); });
      expect(adapter.tabs().find(t => t.getAttribute("aria-selected") === "true")).toBe(target);
      invariants("click");
    });

    it("has no disabled tabs, so arrows never need to skip one", () => {
      for (const t of adapter.tabs()) {
        expect(t.hasAttribute("disabled")).toBe(false);
        expect(t.hasAttribute("aria-disabled")).toBe(false);
      }
    });

    it("leaves Tab alone and lets the arrow keys bubble to ancestors and the window (no ARIA-relevant listener acts on them)", async () => {
      focusActive();
      const tab = await press(focused(), "Tab");
      expect(tab.defaultPrevented, "Tab must continue to the browser, leaving the tablist").toBe(false);
      parentKeys = 0; windowKeys = 0;
      // Current policy (owner question): handled arrows are prevented but not stopped; multiSelection.ts
      // only acts on Escape and the achievements Konami matcher (konami.ts) observes
      // the arrows harmlessly, so the leak has no side effect today.
      await press(focused(), "ArrowRight");
      expect(parentKeys).toBe(1);
      expect(windowKeys).toBe(1);
    });
  });
}
