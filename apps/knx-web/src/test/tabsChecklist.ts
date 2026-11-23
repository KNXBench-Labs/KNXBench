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
  /** `all`: every tab names its panel. `selected-only`: only the selected tab does (its panel is not always mounted). */
  controls: "all" | "selected-only";
  /** `present`: Home/End select the first/last tab. `absent`: they are ignored (current policy, owner question). */
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

    it("links every tab and panel both ways through aria-controls and aria-labelledby", () => {
      const tabs = adapter.tabs();
      const selected = selectedOf(tabs)[0];
      for (const t of tabs) {
        const controls = t.getAttribute("aria-controls");
        if (adapter.controls === "selected-only" && t !== selected) {
          expect(controls, "an unselected tab must not point at a missing panel").toBeNull();
          continue;
        }
        expect(controls, t.id).toBeTruthy();
        const panel = document.getElementById(controls!);
        expect(panel, controls!).not.toBeNull();
        expect(panel!.getAttribute("role")).toBe("tabpanel");
        expect(panel!.getAttribute("aria-labelledby")).toBe(t.id);
        expect(document.getElementById(t.id)).toBe(t);
      }
      // Every panel is named by a tab of this list, and ids are unique.
      for (const panel of adapter.panels()) {
        const owner = document.getElementById(panel.getAttribute("aria-labelledby")!);
        expect(tabs, panel.id).toContain(owner);
      }
      const ids = [...tabs, ...adapter.panels()].map(el => el.id);
      expect(new Set(ids).size).toBe(ids.length);
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

    it("selects and focuses the next/previous tab on ArrowRight/ArrowLeft (automatic activation, wrapping)", async () => {
      const count = adapter.tabs().length;
      focusActive();
      // Current policy (owner question): arrows wrap and activate at once; the APG allows both.
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
        // Current policy (owner question): Home/End are not handled and are left to the browser.
        expect(end.defaultPrevented).toBe(false);
        expect(afterEnd).toBe(start);
        expect(home.defaultPrevented).toBe(false);
        expect(activeIndex()).toBe(start);
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

    it("leaves Tab alone and lets the arrow keys bubble to ancestors and the window (no listener acts on them)", async () => {
      focusActive();
      const tab = await press(focused(), "Tab");
      expect(tab.defaultPrevented, "Tab must continue to the browser, leaving the tablist").toBe(false);
      parentKeys = 0; windowKeys = 0;
      // Current policy (owner question): handled arrows are prevented but not stopped; multiSelection.ts
      // only acts on Escape, so the leak has no side effect today.
      await press(focused(), "ArrowRight");
      expect(parentKeys).toBe(1);
      expect(windowKeys).toBe(1);
    });
  });
}
