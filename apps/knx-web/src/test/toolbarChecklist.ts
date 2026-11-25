/** Shared keyboard, focus and ARIA checklist (WP-49.4b) run by every hand-written toolbar test. */
// SPDX-License-Identifier: AGPL-3.0-or-later
import { act } from "react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { press } from "./comboboxChecklist";

/** How one toolbar is mounted and observed; the checklist itself never touches widget internals. */
export interface ToolbarAdapter {
  name: string;
  /** Mounts the REAL component inside a parent that calls `parentKeyDown` for every keydown reaching it. */
  mount(parentKeyDown: () => void): Promise<void>;
  unmount(): Promise<void>;
  toolbar(): HTMLElement;
  /** Every control of the toolbar in document order, disabled ones included. */
  controls(): HTMLElement[];
  /** The controls that must expose `aria-pressed`; every other control must not. Re-read after each state change. */
  toggles(): HTMLElement[];
  /** Moves the toolbar into a state where at least one control is disabled; returns them. Absent: nothing here can be disabled. */
  disable?(): Promise<HTMLElement[]>;
}

const isDisabled = (el: HTMLElement) => el instanceof HTMLButtonElement && el.disabled;
const nameOf = (el: HTMLElement) => (el.getAttribute("aria-label") ?? el.textContent ?? "").trim();

/**
 * Registers the shared checklist cases for one toolbar. DECIDED rule for every toolbar
 * (owner Q12, 23 Nov 2026, WP-49.5): NO roving tabindex, so every enabled button is a tab
 * stop; the toolbar handles no keys itself, so arrows, Home and End are left to the
 * browser. This is a conscious deviation from the APG toolbar pattern, recorded in the
 * ADR-0191 dated amendment; converting would change the Tab order for every user.
 */
export function describeToolbarChecklist(adapter: ToolbarAdapter) {
  describe(`${adapter.name}: toolbar checklist`, () => {
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
    /** Name, type and pressed-state rules, re-run after every state change. */
    const invariants = (label: string) => {
      const bar = adapter.toolbar();
      expect(bar.getAttribute("role"), label).toBe("toolbar");
      expect(bar.getAttribute("aria-label")?.trim(), `${label}: the toolbar needs a name`).toBeTruthy();
      expect(bar.hasAttribute("aria-orientation"), `${label}: horizontal is the default`).toBe(false);
      const controls = adapter.controls();
      const toggles = adapter.toggles();
      for (const c of controls) {
        expect(bar.contains(c), label).toBe(true);
        expect(nameOf(c), `${label}: every control needs a name`).not.toBe("");
        expect((c as HTMLButtonElement).type, label).toBe("button");
        if (toggles.includes(c)) expect(["true", "false"], `${label}: ${nameOf(c)}`).toContain(c.getAttribute("aria-pressed"));
        else expect(c.hasAttribute("aria-pressed"), `${label}: ${nameOf(c)} is not a toggle`).toBe(false);
      }
      for (const t of toggles) expect(controls, label).toContain(t);
    };

    it("exposes a named toolbar whose controls are named buttons, toggles carrying aria-pressed", () => {
      expect(adapter.controls().length).toBeGreaterThanOrEqual(1);
      invariants("initial");
    });

    it("keeps names and aria-pressed correct after each toggle changes state", async () => {
      // A toggle may be re-rendered with another caption (Maximize/Restore): follow it by position.
      const positions = adapter.toggles().map(t => adapter.controls().indexOf(t));
      for (const position of positions) {
        const before = adapter.controls()[position];
        const was = before.getAttribute("aria-pressed");
        await act(async () => { before.click(); });
        const after = adapter.controls()[position];
        expect(adapter.toggles(), `control ${position} is still a toggle`).toContain(after);
        expect(after.getAttribute("aria-pressed"), `control ${position}`).toBe(was === "true" ? "false" : "true");
        invariants(`after toggle ${position}`);
      }
    });

    it("leaves every enabled control a tab stop: no roving tabindex (decided rule, owner Q12, 23 Nov 2026)", () => {
      for (const c of adapter.controls()) {
        expect(c.tabIndex, nameOf(c)).toBe(0);
        expect(c.hasAttribute("tabindex"), "native tab stop, no tabindex attribute").toBe(false);
      }
    });

    it("does not handle arrows, Home or End, modified or not, and keeps focus where it is (decided rule, owner Q12, 23 Nov 2026)", async () => {
      const first = adapter.controls()[0];
      first.focus();
      for (const key of ["ArrowRight", "ArrowLeft", "ArrowDown", "ArrowUp", "Home", "End"]) {
        for (const mods of [{}, { shiftKey: true }, { altKey: true }, { ctrlKey: true }, { metaKey: true }]) {
          const event = await press(focused(), key, mods);
          expect(event.defaultPrevented, `${key} ${JSON.stringify(mods)} is left to the browser`).toBe(false);
          expect(focused(), `${key} must not move focus`).toBe(first);
        }
      }
    });

    it("lets Tab and every unhandled key bubble to ancestors and the window", async () => {
      adapter.controls()[0].focus();
      const tab = await press(focused(), "Tab");
      expect(tab.defaultPrevented, "Tab continues to the browser").toBe(false);
      expect(parentKeys).toBe(1);
      expect(windowKeys).toBe(1);
      await press(focused(), "ArrowRight");
      expect(parentKeys).toBe(2);
      expect(windowKeys).toBe(2);
    });

    if (adapter.disable) it("disables controls natively: no aria-disabled stand-in, and a disabled control cannot take focus", async () => {
      const disabled = await adapter.disable!();
      expect(disabled.length).toBeGreaterThanOrEqual(1);
      for (const c of adapter.controls()) {
        if (disabled.includes(c)) {
          expect(isDisabled(c), nameOf(c)).toBe(true);
          expect(c.hasAttribute("aria-disabled"), nameOf(c)).toBe(false);
          c.focus();
          expect(document.activeElement, `${nameOf(c)}: a disabled control is skipped`).not.toBe(c);
        } else {
          expect(isDisabled(c), nameOf(c)).toBe(false);
        }
      }
      invariants("after disabling");
    });
  });
}
