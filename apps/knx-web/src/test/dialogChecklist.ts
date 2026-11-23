/** Shared keyboard, focus and ARIA checklist (WP-49.4c) run by every modal dialog test. */
// SPDX-License-Identifier: AGPL-3.0-or-later
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { press } from "./comboboxChecklist";

/** How one dialog is mounted and observed; the checklist itself never touches component internals. */
export interface DialogAdapter {
  name: string;
  /** Mounts the REAL component inside a parent that calls `parentKeyDown` for every keydown reaching it. */
  mount(parentKeyDown: () => void): Promise<void>;
  unmount(): Promise<void>;
  /** Brings the dialog up. When there is an opener it is focused first, as a keyboard user would have it. */
  open(): Promise<void>;
  dialog(): HTMLElement;
  isOpen(): boolean;
  /** The control the dialog was opened from. Absent: nothing opens it (the login screen). */
  opener?(): HTMLElement;
  /** Page content outside the dialog (at least one element) that a modal must hide from the user. */
  background(): HTMLElement[];
  /** Where focus must land on open. */
  initialFocus(): HTMLElement;
  /** Every enabled, visible tab stop inside the dialog in document order. */
  tabStops(): HTMLElement[];
  /** `aria-labelledby` points at a visible title; `aria-label` carries the name itself. */
  nameSource: "aria-labelledby" | "aria-label";
  /** `wired`: `aria-describedby` points at an existing element. `none`: absent (recorded, not added). */
  describedBy: "wired" | "none";
  /**
   * `overlay`: the shared shell traps Tab, isolates the background and returns focus. `standalone`: the
   * component owns none of that (the login screen; its owner, `AuthGate`, makes the app inert).
   */
  shell: "overlay" | "standalone";
  /**
   * What Escape does with the focus on `escapeFrom()`: whether it closes, and whether the event is
   * default-prevented and stopped before ancestors and the window see it.
   */
  escape: { closes: boolean; prevented: boolean; stopped: boolean };
  /** Where Escape is pressed. Defaults to the focused element after open. */
  escapeFrom?(): HTMLElement;
  /** Moves the dialog into a busy state in which it refuses to close; Escape must stay inert. */
  busy?(): Promise<void>;
  /** Removes the opener from the DOM while the dialog is open; `fallback` is where focus must go on close. */
  openerGone?: { remove(): Promise<void>; fallback(): HTMLElement };
  /** Opens a second dialog from the first. The top one owns Escape, Tab and focus return. */
  stack?: { openSecond(): Promise<void>; second(): HTMLElement; secondIsOpen(): boolean; secondOpener(): HTMLElement };
}

const nameOf = (el: HTMLElement) => (el.getAttribute("aria-label") ?? el.textContent ?? "").trim();
const isolated = (el: HTMLElement) => el.hasAttribute("inert") && el.getAttribute("aria-hidden") === "true";

/** Registers the shared checklist cases for one dialog; component-specific cases stay in its own file. */
export function describeDialogChecklist(adapter: DialogAdapter) {
  describe(`${adapter.name}: dialog checklist`, () => {
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
    const openFromKeyboard = async () => {
      adapter.opener?.().focus();
      await adapter.open();
    };

    it("exposes a modal dialog with one accessible name and, where wired, a description", async () => {
      await openFromKeyboard();
      const d = adapter.dialog();
      expect(d.getAttribute("role")).toBe("dialog");
      expect(d.getAttribute("aria-modal")).toBe("true");
      const labelledBy = d.getAttribute("aria-labelledby");
      const label = d.getAttribute("aria-label");
      if (adapter.nameSource === "aria-labelledby") {
        expect(label, "one name source only").toBeNull();
        expect(labelledBy).toBeTruthy();
        const titles = labelledBy!.split(/\s+/).map(id => d.ownerDocument.getElementById(id));
        for (const title of titles) {
          expect(title, "the referenced title exists").not.toBeNull();
          expect(d.contains(title), "the title is inside the dialog").toBe(true);
          expect(nameOf(title!), "the title is visible text").not.toBe("");
        }
      } else {
        expect(labelledBy, "one name source only").toBeNull();
        expect(label?.trim()).toBeTruthy();
      }
      const describedBy = d.getAttribute("aria-describedby");
      if (adapter.describedBy === "wired") {
        expect(describedBy).toBeTruthy();
        expect(d.ownerDocument.getElementById(describedBy!), "the description exists").not.toBeNull();
      } else expect(describedBy, "no description is wired (recorded, not added)").toBeNull();
    });

    it("puts initial focus on its designated element inside the dialog", async () => {
      await openFromKeyboard();
      expect(adapter.dialog().contains(focused())).toBe(true);
      expect(focused()).toBe(adapter.initialFocus());
      expect(adapter.tabStops().includes(focused()) || focused() === adapter.dialog(), "a tab stop or the panel itself").toBe(true);
    });

    if (adapter.shell === "overlay") {
      it("wraps Tab and Shift+Tab inside the dialog, and leaves a mid-list Tab to the browser", async () => {
        await openFromKeyboard();
        const stops = adapter.tabStops();
        expect(stops.length).toBeGreaterThanOrEqual(2);
        const last = stops[stops.length - 1];
        last.focus();
        const forward = await press(focused(), "Tab");
        expect(forward.defaultPrevented, "Tab on the last stop is wrapped by the dialog").toBe(true);
        expect(focused()).toBe(stops[0]);
        const backward = await press(focused(), "Tab", { shiftKey: true });
        expect(backward.defaultPrevented, "Shift+Tab on the first stop is wrapped").toBe(true);
        expect(focused()).toBe(last);
        stops[0].focus();
        const middle = await press(focused(), "Tab");
        expect(middle.defaultPrevented, "Tab between stops belongs to the browser").toBe(false);
        expect(focused()).toBe(stops[0]);
        for (const stop of stops) expect(adapter.dialog().contains(stop)).toBe(true);
      });

      it("pulls Tab and Shift+Tab back into the dialog when focus has left it", async () => {
        await openFromKeyboard();
        const stops = adapter.tabStops();
        const outside = document.createElement("button");
        document.body.append(outside);
        try {
          // The shell isolates late body children too, which makes `outside` unfocusable after the first
          // press; lift that so focus really sits outside each time, and assert it.
          const focusOutside = () => {
            outside.removeAttribute("inert"); outside.removeAttribute("aria-hidden");
            outside.focus();
            expect(focused(), "precondition: focus sits outside the dialog").toBe(outside);
          };
          focusOutside();
          // The dialog's keydown handler only sees events that bubble through it, so aim at the dialog.
          await press(adapter.dialog(), "Tab");
          expect(focused()).toBe(stops[0]);
          focusOutside();
          await press(adapter.dialog(), "Tab", { shiftKey: true });
          expect(focused()).toBe(stops[stops.length - 1]);
        } finally { outside.remove(); }
      });

      it("hides the background from the user and from assistive technology, restoring it on close, and locks no scroll", async () => {
        const behind = adapter.background();
        expect(behind.length).toBeGreaterThanOrEqual(1);
        for (const el of behind) expect(isolated(el), "before open").toBe(false);
        await openFromKeyboard();
        for (const el of behind) expect(isolated(el), "while open").toBe(true);
        expect(adapter.dialog().closest("[inert], [aria-hidden='true']")).toBeNull();
        // Scroll lock is not implemented: record it so adding one is a deliberate change.
        expect(document.body.style.overflow).toBe("");
        expect(document.documentElement.style.overflow).toBe("");
        await press(focused(), "Escape");
        if (adapter.escape.closes) {
          expect(adapter.isOpen()).toBe(false);
          for (const el of behind) expect(isolated(el), "after close").toBe(false);
          for (const el of behind) expect(el.hasAttribute("inert") || el.hasAttribute("aria-hidden"), "after close").toBe(false);
        }
      });
    } else {
      it("leaves Tab to the browser and isolates nothing itself: the owner makes the page inert", async () => {
        await openFromKeyboard();
        for (const el of adapter.background()) expect(isolated(el), "the owner, not the component, isolates").toBe(false);
        for (const shiftKey of [false, true]) {
          const tab = await press(focused(), "Tab", { shiftKey });
          expect(tab.defaultPrevented).toBe(false);
        }
        expect(document.body.style.overflow).toBe("");
      });
    }

    it(`handles Escape as pinned: ${adapter.escape.closes ? "closes" : "stays open"}, ${adapter.escape.prevented ? "prevented" : "not prevented"}, ${adapter.escape.stopped ? "stopped" : "propagating"}`, async () => {
      await openFromKeyboard();
      const from = adapter.escapeFrom ? adapter.escapeFrom() : focused();
      if (adapter.escapeFrom) from.focus();
      const before = { parent: parentKeys, window: windowKeys };
      const escape = await press(from, "Escape");
      expect(adapter.isOpen()).toBe(!adapter.escape.closes);
      expect(escape.defaultPrevented).toBe(adapter.escape.prevented);
      expect(parentKeys - before.parent, "ancestors").toBe(adapter.escape.stopped ? 0 : 1);
      expect(windowKeys - before.window, "window").toBe(adapter.escape.stopped ? 0 : 1);
    });

    if (adapter.busy) it("keeps Escape inert while busy, with the key still stopped as pinned", async () => {
      await openFromKeyboard();
      await adapter.busy!();
      const before = { parent: parentKeys, window: windowKeys };
      await press(adapter.dialog(), "Escape");
      expect(adapter.isOpen(), "a busy dialog refuses to close").toBe(true);
      expect(parentKeys - before.parent).toBe(adapter.escape.stopped ? 0 : 1);
      expect(windowKeys - before.window).toBe(adapter.escape.stopped ? 0 : 1);
    });

    if (adapter.opener && adapter.escape.closes) it("returns focus to the opener after Escape", async () => {
      await openFromKeyboard();
      await press(focused(), "Escape");
      expect(adapter.isOpen()).toBe(false);
      expect(focused()).toBe(adapter.opener!());
    });

    if (adapter.openerGone) it("falls back to the documented element when the opener has disappeared", async () => {
      await openFromKeyboard();
      await adapter.openerGone!.remove();
      await press(focused(), "Escape");
      expect(adapter.isOpen()).toBe(false);
      expect(focused()).toBe(adapter.openerGone!.fallback());
    });

    if (adapter.stack) it("lets only the top of two stacked dialogs take Escape, and returns focus to the lower one's opener", async () => {
      await openFromKeyboard();
      const stack = adapter.stack!;
      await stack.openSecond();
      expect(adapter.dialog().contains(stack.second()) || stack.second().isConnected).toBe(true);
      expect(stack.second().contains(focused()), "the top dialog holds focus").toBe(true);
      const before = { parent: parentKeys, window: windowKeys };
      await press(focused(), "Escape");
      expect(stack.secondIsOpen(), "Escape closes the top dialog").toBe(false);
      expect(adapter.isOpen(), "and only the top one").toBe(true);
      expect(windowKeys - before.window, "the lower dialog never saw the key").toBe(0);
      expect(parentKeys - before.parent).toBe(0);
      expect(focused(), "focus returns to where the second dialog was opened").toBe(stack.secondOpener());
      expect(adapter.dialog().closest("[inert], [aria-hidden='true']")).toBeNull();
      await press(focused(), "Escape");
      expect(adapter.isOpen(), "the next Escape closes the lower dialog").toBe(false);
    });

    if (adapter.stack && adapter.shell === "overlay") it("keeps Tab inside the top of two stacked dialogs", async () => {
      await openFromKeyboard();
      await adapter.stack!.openSecond();
      const top = adapter.stack!.second();
      const stops = [...top.querySelectorAll<HTMLElement>("button, input")].filter(el => !(el as HTMLButtonElement).disabled);
      expect(stops.length).toBeGreaterThanOrEqual(2);
      stops[stops.length - 1].focus();
      await press(focused(), "Tab");
      expect(focused(), "forward wrap stays in the top dialog").toBe(stops[0]);
      await press(focused(), "Tab", { shiftKey: true });
      expect(focused(), "backward wrap stays in the top dialog").toBe(stops[stops.length - 1]);
    });
  });
}
