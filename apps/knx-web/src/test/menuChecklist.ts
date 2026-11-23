/** Shared keyboard, focus and ARIA checklist (WP-49.3) run by every hand-written menu test. */
// SPDX-License-Identifier: AGPL-3.0-or-later
import { act } from "react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { press } from "./comboboxChecklist";

/** A submenu of the menu under test: its trigger entry in the top level and its panel. */
export interface SubmenuAdapter {
  trigger(): HTMLElement;
  panel(): HTMLElement;
  /** Panel entries in document order, disabled ones included; the first one is disabled. */
  items(): HTMLElement[];
}

/** How one menu is mounted and observed; the checklist itself never touches widget internals. */
export interface MenuAdapter {
  name: string;
  /** Mounts the menu inside a parent that calls `parentKeyDown` for every keydown reaching it. */
  mount(parentKeyDown: () => void): Promise<void>;
  unmount(): Promise<void>;
  /** The element that opened the menu: focus returns to it when the menu closes. */
  trigger(): HTMLElement;
  /** Opens the menu by keyboard from the focused trigger. */
  open(): Promise<void>;
  /** The top-level `role=menu` element while open, otherwise null. */
  menu(): HTMLElement | null;
  /** Top-level entries in document order, disabled ones included (the first one may be disabled). */
  items(): HTMLElement[];
  /** How many times any entry's action has run since mounting. */
  activations(): number;
  /** A pointer press that lands inside the open menu / outside of it. */
  pressInside(): Promise<void>;
  pressOutside(): Promise<void>;
  /** `menu-button`: the trigger announces the popup and its state. `none`: a context menu has no persistent trigger. */
  popupTrigger: "menu-button" | "none";
  /**
   * `continues`: Tab closes the menu and is left to the browser (menu button). `swallows`: Tab closes it
   * and is prevented, focus returning to the opener (a context menu has no persistent trigger). Both are
   * decided rules, not an inconsistency (owner, 23 November 2026, question 6).
   */
  tab: "continues" | "swallows";
  submenu?: SubmenuAdapter;
}

const isDisabled = (el: HTMLElement) => el instanceof HTMLButtonElement && el.disabled;

/** Registers the shared checklist cases for one menu; widget-specific cases stay in its own file. */
export function describeMenuChecklist(adapter: MenuAdapter) {
  describe(`${adapter.name}: menu checklist`, () => {
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
    const enabled = () => adapter.items().filter(el => !isDisabled(el));
    const stops = (items: HTMLElement[]) => items.filter(el => el.tabIndex === 0);

    it("opens onto the first enabled entry and leaves it as the only tab stop", async () => {
      expect(adapter.menu()).toBeNull();
      await adapter.open();
      expect(adapter.menu()).not.toBeNull();
      expect(document.activeElement).toBe(enabled()[0]);
      expect(stops(adapter.items())).toEqual([enabled()[0]]);
    });

    it("moves focus with ArrowDown, ArrowUp, Home and End over enabled entries only, one tab stop following it", async () => {
      await adapter.open();
      const live = enabled();
      const focused = () => document.activeElement;
      for (const key of ["ArrowDown", "ArrowUp", "Home", "End", "ArrowDown", "ArrowUp"]) {
        const event = await press(focused() as HTMLElement, key);
        // The page behind a menu must not scroll while its keys are used.
        expect(event.defaultPrevented, key).toBe(true);
        expect(live).toContain(focused());
        expect(stops(adapter.items()), key).toEqual([focused()]);
      }
      await press(focused() as HTMLElement, "Home");
      expect(focused()).toBe(live[0]);
      await press(focused() as HTMLElement, "End");
      expect(focused()).toBe(live[live.length - 1]);
      if (live.length >= 2) {
        // Decided rule (owner, 23 November 2026, question 6): the arrows wrap around, the APG menu pattern allows both.
        await press(focused() as HTMLElement, "ArrowDown");
        expect(focused()).toBe(live[0]);
        await press(focused() as HTMLElement, "ArrowUp");
        expect(focused()).toBe(live[live.length - 1]);
        await press(focused() as HTMLElement, "Home");
        await press(focused() as HTMLElement, "ArrowDown");
        expect(focused()).toBe(live[1]);
      } else {
        await press(focused() as HTMLElement, "ArrowDown");
        expect(focused()).toBe(live[0]);
      }
    });

    it("activates a focused entry through a native button, so Enter and Space click it", async () => {
      await adapter.open();
      const entry = enabled()[0];
      // happy-dom does not turn Enter/Space into a click; a native enabled button does so in a browser.
      expect(entry).toBeInstanceOf(HTMLButtonElement);
      expect((entry as HTMLButtonElement).type).toBe("button");
      expect(adapter.activations()).toBe(0);
      await act(async () => { entry.click(); });
      expect(adapter.activations()).toBe(1);
    });

    it("keeps disabled entries inert (disabled attribute, skipped by the arrows)", async () => {
      await adapter.open();
      const disabled = adapter.items().filter(isDisabled);
      if (disabled.length === 0) return;
      for (const entry of disabled) {
        expect(entry.hasAttribute("disabled")).toBe(true);
        await act(async () => { entry.click(); });
      }
      expect(adapter.activations()).toBe(0);
      await press(document.activeElement as HTMLElement, "Home");
      for (let i = 0; i < adapter.items().length + 1; i++) {
        expect(disabled).not.toContain(document.activeElement);
        await press(document.activeElement as HTMLElement, "ArrowDown");
      }
    });

    it("closes on Escape, returns focus to the trigger, and stops the key so one Escape closes one level", async () => {
      await adapter.open();
      parentKeys = 0; windowKeys = 0;
      const event = await press(document.activeElement as HTMLElement, "Escape");
      expect(adapter.menu()).toBeNull();
      expect(document.activeElement).toBe(adapter.trigger());
      // Decided rule (owner, 23 November 2026, question 8): the first-level Escape is prevented and
      // stopped, so no page-level Escape handler (the multi-selection clear, say) sees it.
      expect(event.defaultPrevented).toBe(true);
      expect(parentKeys).toBe(0);
      expect(windowKeys).toBe(0);
    });

    it("closes on Tab with focus on the trigger and leaves the key as the menu's decided rule says", async () => {
      await adapter.open();
      const event = await press(document.activeElement as HTMLElement, "Tab");
      expect(adapter.menu()).toBeNull();
      expect(document.activeElement).toBe(adapter.trigger());
      expect(event.defaultPrevented).toBe(adapter.tab === "swallows");
    });

    it("closes on a pointer press outside and stays open on one inside", async () => {
      await adapter.open();
      await adapter.pressInside();
      expect(adapter.menu()).not.toBeNull();
      await adapter.pressOutside();
      expect(adapter.menu()).toBeNull();
    });

    it("exposes menu and menuitem roles, the trigger's popup state, the menu's name and its link to the trigger", async () => {
      const trigger = adapter.trigger();
      if (adapter.popupTrigger === "menu-button") {
        expect(trigger.getAttribute("aria-haspopup")).toBe("menu");
        expect(trigger.getAttribute("aria-expanded")).toBe("false");
      } else {
        expect(trigger.hasAttribute("aria-haspopup")).toBe(false);
        expect(trigger.hasAttribute("aria-expanded")).toBe(false);
      }
      await adapter.open();
      const menu = adapter.menu()!;
      expect(menu.getAttribute("role")).toBe("menu");
      expect(adapter.items().length).toBeGreaterThan(0);
      for (const entry of adapter.items()) expect(entry.getAttribute("role")).toBe("menuitem");
      if (adapter.popupTrigger === "menu-button") expect(trigger.getAttribute("aria-expanded")).toBe("true");
      // Decided rule (owner, 23 November 2026, question 7): every menu is named, and a menu button
      // and its menu point at each other (aria-controls / aria-labelledby).
      const labelledBy = menu.getAttribute("aria-labelledby");
      const name = menu.getAttribute("aria-label")
        ?? (labelledBy ? document.getElementById(labelledBy)?.textContent : undefined);
      expect(name?.trim()).toBeTruthy();
      if (adapter.popupTrigger === "menu-button") {
        expect(menu.id).not.toBe("");
        expect(trigger.getAttribute("aria-controls")).toBe(menu.id);
        expect(document.getElementById(labelledBy!)).toBe(trigger);
      }
    });

    const sub = adapter.submenu;
    if (!sub) return;

    describe("submenu", () => {
      const openSub = async () => {
        await adapter.open();
        sub.trigger().focus();
        await press(sub.trigger(), "ArrowRight");
      };
      const liveSub = () => sub.items().filter(el => !isDisabled(el));

      it("opens on ArrowRight onto its first enabled entry and links trigger and panel", async () => {
        expect(sub.trigger().getAttribute("aria-haspopup")).toBe("menu");
        expect(sub.trigger().getAttribute("aria-expanded")).toBe("false");
        expect(sub.panel().hidden).toBe(true);
        await openSub();
        expect(sub.trigger().getAttribute("aria-expanded")).toBe("true");
        expect(sub.panel().hidden).toBe(false);
        expect(sub.panel().getAttribute("role")).toBe("menu");
        expect(document.getElementById(sub.trigger().getAttribute("aria-controls")!)).toBe(sub.panel());
        expect(document.getElementById(sub.panel().getAttribute("aria-labelledby")!)).toBe(sub.trigger());
        expect(sub.items()[0]).toSatisfy(isDisabled);
        expect(document.activeElement).toBe(liveSub()[0]);
        for (const entry of sub.items()) expect(entry.getAttribute("role")).toBe("menuitem");
        expect(stops(sub.items())).toEqual([liveSub()[0]]);
      });

      it("closes on ArrowLeft with focus on its trigger and the menu still open", async () => {
        await openSub();
        const event = await press(document.activeElement as HTMLElement, "ArrowLeft");
        expect(event.defaultPrevented).toBe(true);
        expect(sub.trigger().getAttribute("aria-expanded")).toBe("false");
        expect(sub.panel().hidden).toBe(true);
        expect(document.activeElement).toBe(sub.trigger());
        expect(adapter.menu()).not.toBeNull();
      });

      it("closes only itself on the first Escape and stops it; the second Escape follows the first-level policy", async () => {
        await openSub();
        parentKeys = 0; windowKeys = 0;
        const first = await press(document.activeElement as HTMLElement, "Escape");
        expect(first.defaultPrevented).toBe(true);
        expect(parentKeys).toBe(0);
        expect(windowKeys).toBe(0);
        expect(sub.trigger().getAttribute("aria-expanded")).toBe("false");
        expect(document.activeElement).toBe(sub.trigger());
        expect(adapter.menu()).not.toBeNull();
        await press(document.activeElement as HTMLElement, "Escape");
        expect(adapter.menu()).toBeNull();
        expect(document.activeElement).toBe(adapter.trigger());
      });

      it("closes the whole menu on Tab from inside the submenu", async () => {
        await openSub();
        const event = await press(document.activeElement as HTMLElement, "Tab");
        expect(event.defaultPrevented).toBe(false);
        expect(adapter.menu()).toBeNull();
        expect(sub.trigger().getAttribute("aria-expanded")).toBe("false");
        expect(document.activeElement).toBe(adapter.trigger());
      });

      it("keeps ArrowDown/ArrowUp inside the open submenu, skipping its disabled entry", async () => {
        await openSub();
        const live = liveSub();
        expect(live.length).toBeGreaterThanOrEqual(2);
        await press(document.activeElement as HTMLElement, "ArrowDown");
        expect(document.activeElement).toBe(live[1]);
        await press(document.activeElement as HTMLElement, "ArrowUp");
        expect(document.activeElement).toBe(live[0]);
        await press(document.activeElement as HTMLElement, "End");
        expect(document.activeElement).toBe(live[live.length - 1]);
        await press(document.activeElement as HTMLElement, "Home");
        expect(document.activeElement).toBe(live[0]);
      });
    });
  });
}
