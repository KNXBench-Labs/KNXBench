/** Shared keyboard, focus and ARIA checklist (WP-49.2) run by every hand-written combobox test. */
// SPDX-License-Identifier: AGPL-3.0-or-later
import { act } from "react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";

/** How one widget is mounted and observed; the checklist itself never touches widget internals. */
export interface ComboboxAdapter {
  name: string;
  /** Mounts the widget inside a parent that calls `parentKeyDown` for every keydown reaching it. */
  mount(parentKeyDown: () => void): Promise<void>;
  unmount(): Promise<void>;
  input(): HTMLInputElement;
  listbox(): HTMLElement;
  /** Opens the list with at least two options, the input focused and the first option highlighted. */
  open(): Promise<void>;
  /** Leaves the input focused with a list that has no options. */
  noOptions(): Promise<void>;
  /** What the widget chose so far, as comparable strings. */
  chosen(): string[];
  /** The choice that Enter on the option at `index` must produce. */
  expectedChoice(index: number): string;
  /**
   * `list`: Escape closes the popup and the widget stays. `dialog`: the popup is always shown and
   * Escape belongs to the surrounding overlay, which closes: the APG dialog pattern, decided by
   * the owner on 23 November 2026 (question 4).
   */
  escape: "list" | "dialog";
  /** `dialog` widgets only: whether the surrounding overlay has been asked to close. */
  dialogClosed(): boolean;
  /** `closes-list`: Tab collapses the popup. `stays-open`: the always-visible list is untouched. */
  tab: "closes-list" | "stays-open";
  /** `aria-expanded` while the list holds no options. */
  expandedWhenEmpty: "true" | "false";
}

export async function press(input: HTMLElement, key: string, init: KeyboardEventInit = {}): Promise<KeyboardEvent> {
  const event = new KeyboardEvent("keydown", { key, bubbles: true, cancelable: true, ...init });
  await act(async () => { input.dispatchEvent(event); });
  return event;
}

export function setNativeValue(input: HTMLInputElement, text: string) {
  Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")!.set!.call(input, text);
  input.dispatchEvent(new Event("input", { bubbles: true }));
}

const optionsOf = (a: ComboboxAdapter) => [...a.listbox().querySelectorAll<HTMLElement>("[role=option]")];
// Decided rule (owner, 23 November 2026, question 5): `aria-selected` marks the highlighted option, the
// APG single-select combobox semantics; there is no separate "chosen" mark, unlike ui-sdk's split.
const highlighted = (a: ComboboxAdapter) => optionsOf(a).filter(o => o.getAttribute("aria-selected") === "true");
const activeId = (a: ComboboxAdapter) => a.input().getAttribute("aria-activedescendant");

/** Registers the shared checklist cases for one widget; widget-specific cases stay in its own file. */
export function describeComboboxChecklist(adapter: ComboboxAdapter) {
  describe(`${adapter.name}: combobox checklist`, () => {
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

    it("keeps focus in the input while the highlight moves", async () => {
      await adapter.open();
      const input = adapter.input();
      expect(document.activeElement).toBe(input);
      for (const key of ["ArrowDown", "ArrowDown", "ArrowUp"]) {
        await press(input, key);
        expect(document.activeElement).toBe(input);
      }
    });

    it("points aria-activedescendant at the single highlighted option as it moves", async () => {
      await adapter.open();
      const options = optionsOf(adapter);
      expect(options.length).toBeGreaterThanOrEqual(2);
      expect(activeId(adapter)).toBe(options[0].id);
      expect(highlighted(adapter)).toEqual([options[0]]);
      await press(adapter.input(), "ArrowDown");
      expect(activeId(adapter)).toBe(options[1].id);
      expect(highlighted(adapter)).toEqual([options[1]]);
      // The id must resolve to a real option in the document, not just be a string.
      expect(document.getElementById(activeId(adapter)!)?.getAttribute("role")).toBe("option");
    });

    it("drops aria-activedescendant when no option is highlighted", async () => {
      await adapter.noOptions();
      expect(optionsOf(adapter)).toEqual([]);
      expect(adapter.input().hasAttribute("aria-activedescendant")).toBe(false);
      expect(adapter.input().getAttribute("aria-expanded")).toBe(adapter.expandedWhenEmpty);
    });

    it("stops at both ends instead of wrapping", async () => {
      await adapter.open();
      const options = optionsOf(adapter);
      await press(adapter.input(), "ArrowUp");
      expect(activeId(adapter)).toBe(options[0].id);
      for (let i = 0; i < options.length + 2; i++) await press(adapter.input(), "ArrowDown");
      // Disabled rows (palette) are skipped, so only the *last reachable* option is asserted.
      const last = highlighted(adapter);
      expect(last).toHaveLength(1);
      expect(activeId(adapter)).toBe(last[0].id);
      expect(options.indexOf(last[0])).toBeGreaterThan(0);
      await press(adapter.input(), "ArrowDown");
      expect(activeId(adapter)).toBe(last[0].id);
      for (let i = 0; i < options.length + 2; i++) await press(adapter.input(), "ArrowUp");
      expect(activeId(adapter)).toBe(options[0].id);
    });

    it("chooses the highlighted option on Enter", async () => {
      await adapter.open();
      await press(adapter.input(), "ArrowDown");
      await press(adapter.input(), "Enter");
      expect(adapter.chosen()).toEqual([adapter.expectedChoice(1)]);
    });

    if (adapter.escape === "list") {
      it("closes only the list on the first Escape and stops it; the second Escape reaches the page", async () => {
        await adapter.open();
        const first = await press(adapter.input(), "Escape");
        expect(first.defaultPrevented).toBe(true);
        expect(parentKeys).toBe(0);
        expect(windowKeys).toBe(0);
        expect(adapter.input().getAttribute("aria-expanded")).toBe("false");
        expect(optionsOf(adapter)).toEqual([]);
        expect(adapter.input().hasAttribute("aria-activedescendant")).toBe(false);
        expect(document.activeElement).toBe(adapter.input());
        const second = await press(adapter.input(), "Escape");
        expect(second.defaultPrevented).toBe(false);
        expect(parentKeys).toBe(1);
        expect(windowKeys).toBe(1);
      });
    } else {
      // Decided rule (owner, 23 November 2026, question 4): the list is never closable on its own, so
      // the first Escape dismisses the whole overlay (APG dialog pattern). The overlay stops it, so the
      // page does not see it.
      it("decided rule: the first Escape closes the surrounding overlay and stops there", async () => {
        await adapter.open();
        expect(adapter.dialogClosed()).toBe(false);
        await press(adapter.input(), "Escape");
        expect(adapter.dialogClosed()).toBe(true);
        expect(parentKeys).toBe(0);
        expect(windowKeys).toBe(0);
      });
    }

    if (adapter.tab === "closes-list") {
      it("closes the list on Tab and leaves the key to the browser", async () => {
        await adapter.open();
        const tab = await press(adapter.input(), "Tab");
        expect(tab.defaultPrevented).toBe(false);
        expect(adapter.input().getAttribute("aria-expanded")).toBe("false");
        expect(optionsOf(adapter)).toEqual([]);
      });
    } else {
      // Decided rule (owner, 23 November 2026, question 4): the list is always shown, so Tab has
      // nothing to close; the overlay's focus trap keeps focus inside the dialog.
      it("decided rule: Tab leaves the always-visible list as it is", async () => {
        await adapter.open();
        const before = optionsOf(adapter).length;
        await press(adapter.input(), "Tab");
        expect(optionsOf(adapter)).toHaveLength(before);
        expect(adapter.input().getAttribute("aria-expanded")).toBe("true");
        expect(adapter.dialogClosed()).toBe(false);
      });
    }

    it("keeps the input focused on a pointer press on an option, so the choice lands first", async () => {
      await adapter.open();
      const option = optionsOf(adapter)[1];
      const down = new MouseEvent("mousedown", { bubbles: true, cancelable: true, button: 0 });
      await act(async () => { option.dispatchEvent(down); });
      // The default action of mousedown is what moves focus to the nearest focusable ancestor.
      expect(down.defaultPrevented).toBe(true);
      // happy-dom never moves focus on mousedown, so this only guards handler-side blurs;
      // the defaultPrevented check above is the real proof of the browser behaviour.
      expect(document.activeElement).toBe(adapter.input());
      expect(adapter.input().getAttribute("aria-expanded")).toBe("true");
      await act(async () => { option.click(); });
      expect(adapter.chosen()).toEqual([adapter.expectedChoice(1)]);
    });

    it("exposes aria-haspopup, aria-expanded and aria-controls for the rendered listbox", async () => {
      await adapter.open();
      const input = adapter.input();
      expect(input.getAttribute("role")).toBe("combobox");
      expect(input.getAttribute("aria-haspopup")).toBe("listbox");
      expect(input.getAttribute("aria-expanded")).toBe("true");
      const listbox = document.getElementById(input.getAttribute("aria-controls")!);
      expect(listbox).toBe(adapter.listbox());
      expect(listbox!.getAttribute("role")).toBe("listbox");
      expect(listbox!.contains(document.getElementById(activeId(adapter)!))).toBe(true);
    });
  });
}
