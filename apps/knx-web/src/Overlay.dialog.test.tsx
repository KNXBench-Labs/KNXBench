/** Pins the real Overlay shell against the shared dialog checklist (WP-49.4c). */
// @vitest-environment happy-dom
import { act, useEffect, useRef, useState } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, describe, expect, it, vi } from "vitest";
import Overlay from "./Overlay";
import { press } from "./test/comboboxChecklist";
import { describeDialogChecklist, type DialogAdapter } from "./test/dialogChecklist";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

type Variant = "labelledby" | "label";
let host: HTMLDivElement | undefined;
let root: Root | undefined;

/** A small consumer: an opener, a dialog that can be busy, and a second dialog opened from inside it. */
function Consumer({ variant, withRestore }: { variant: Variant; withRestore?: boolean }) {
  const [open, setOpen] = useState(false);
  const [busy, setBusy] = useState(false);
  const [nested, setNested] = useState(false);
  const designated = useRef<HTMLButtonElement>(null);
  const restore = useRef<HTMLButtonElement>(null);
  return <>
    <main>
      <button data-opener onClick={() => setOpen(true)}>Open</button>
      <button data-fallback ref={restore}>Fallback</button>
    </main>
    {open && <Overlay
      {...(variant === "labelledby" ? { labelledBy: "dlg-title" } : { label: "Dialog name" })}
      initialFocusRef={variant === "label" ? designated : undefined}
      restoreFocusRef={withRestore ? restore : undefined}
      onClose={() => { if (!busy) setOpen(false); }}>
      {variant === "labelledby" && <h2 id="dlg-title">Title</h2>}
      <input aria-label="Field" />
      <button data-busy onClick={() => setBusy(true)}>Busy</button>
      <button data-nested-opener onClick={() => setNested(true)}>More</button>
      <button data-close ref={designated} onClick={() => setOpen(false)}>Close</button>
      {nested && <Overlay label="Second dialog" onClose={() => setNested(false)}>
        <input aria-label="Second field" />
        <button data-second-close onClick={() => setNested(false)}>Done</button>
      </Overlay>}
    </Overlay>}
  </>;
}

const q = <T extends HTMLElement>(selector: string) => host!.querySelector<T>(selector)!;
const dialogs = () => [...host!.querySelectorAll<HTMLElement>('[role="dialog"]')];

function adapter(name: string, variant: Variant, withRestore = false): DialogAdapter {
  return {
    name,
    async mount(parentKeyDown) {
      host = document.createElement("div");
      document.body.append(host);
      root = createRoot(host);
      await act(async () => root!.render(<div onKeyDown={parentKeyDown}><Consumer variant={variant} withRestore={withRestore} /></div>));
    },
    async unmount() {
      await act(async () => root!.unmount());
      host!.remove(); host = undefined; root = undefined;
    },
    async open() { await act(async () => q("[data-opener]").click()); },
    dialog: () => dialogs()[0],
    isOpen: () => dialogs().length >= 1,
    opener: () => q("[data-opener]"),
    background: () => [q("main")],
    initialFocus: () => (variant === "labelledby" ? dialogs()[0].querySelector<HTMLElement>("input")! : q("[data-close]")),
    tabStops: () => [...dialogs()[0].querySelectorAll<HTMLElement>("input, button")].filter(el => !el.closest("[role=dialog] [role=dialog]")),
    nameSource: variant === "labelledby" ? "aria-labelledby" : "aria-label",
    describedBy: "none",
    shell: "overlay",
    escape: { closes: true, prevented: false, stopped: true },
    busy: async () => { await act(async () => q("[data-busy]").click()); },
    openerGone: withRestore ? {
      remove: async () => { await act(async () => q("[data-opener]").remove()); },
      fallback: () => q("[data-fallback]"),
    } : undefined,
    stack: variant === "labelledby" ? {
      openSecond: async () => { q("[data-nested-opener]").focus(); await act(async () => q("[data-nested-opener]").click()); },
      second: () => dialogs()[1],
      secondIsOpen: () => dialogs().length === 2,
      secondOpener: () => q("[data-nested-opener]"),
    } : undefined,
  };
}

describeDialogChecklist(adapter("Overlay (aria-labelledby, first focusable, nested dialog)", "labelledby"));
describeDialogChecklist(adapter("Overlay (aria-label, initialFocusRef, restoreFocusRef)", "label", true));

/** A child that moves focus into the dialog while it mounts, before the shell's own effect runs. */
function SelfFocusing({ how }: { how: "autoFocus" | "effect" }) {
  const ref = useRef<HTMLInputElement>(null);
  useEffect(() => { if (how === "effect") ref.current?.focus(); }, [how]);
  return <input ref={ref} aria-label="Field" autoFocus={how === "autoFocus"} />;
}

describe("Overlay beyond the shared checklist", () => {
  afterEach(async () => {
    if (root) await act(async () => root!.unmount());
    host?.remove(); host = undefined; root = undefined;
  });
  const mountConsumer = async (variant: Variant, withRestore = false) => {
    const a = adapter("x", variant, withRestore);
    await a.mount(() => {});
    return a;
  };

  // Regression (found by the RenameWorkbench checklist run): the opener used to be read in the shell's
  // effect, after a child's autoFocus had already moved focus, so close restored to a vanished input.
  it.each(["autoFocus", "effect"] as const)("returns focus to the opener when a child focuses itself on mount (%s)", async (how) => {
    host = document.createElement("div"); document.body.append(host);
    root = createRoot(host);
    const Harness = () => {
      const [open, setOpen] = useState(false);
      return <><button data-opener onClick={() => setOpen(true)}>Open</button>
        {open && <Overlay label="Self focusing" onClose={() => setOpen(false)}><SelfFocusing how={how} /><button>Other</button></Overlay>}</>;
    };
    await act(async () => root!.render(<Harness />));
    q("[data-opener]").focus();
    await act(async () => q("[data-opener]").click());
    expect(document.activeElement).toBe(q("input"));
    await press(q("input"), "Escape");
    expect(dialogs()).toHaveLength(0);
    expect(document.activeElement).toBe(q("[data-opener]"));
  });

  it("restores nothing when the opener is gone and no restoreFocusRef is given: focus falls to the page", async () => {
    const a = await mountConsumer("labelledby");
    q("[data-opener]").focus();
    await a.open();
    await act(async () => q("[data-opener]").remove());
    await press(document.activeElement as HTMLElement, "Escape");
    expect(a.isOpen()).toBe(false);
    expect(document.activeElement).toBe(document.body);
  });

  it("keeps the panel itself focusable as the final focus fallback, but not a tab stop", async () => {
    const a = await mountConsumer("labelledby");
    await a.open();
    expect(dialogs()[0].tabIndex).toBe(-1);
    expect(a.tabStops()).not.toContain(dialogs()[0]);
  });

  it("closes on a backdrop click but not on a click inside the panel, and Escape reaches onClose exactly once", async () => {
    const onClose = vi.fn();
    host = document.createElement("div"); document.body.append(host);
    root = createRoot(host);
    await act(async () => root!.render(<Overlay label="Plain" onClose={onClose}><button>Inside</button></Overlay>));
    await act(async () => host!.querySelector<HTMLElement>(".search-panel")!.click());
    expect(onClose).not.toHaveBeenCalled();
    await act(async () => host!.querySelector<HTMLElement>(".search-overlay")!.click());
    expect(onClose).toHaveBeenCalledTimes(1);
    await press(host.querySelector("button")!, "Escape");
    expect(onClose).toHaveBeenCalledTimes(2);
  });

  it("keeps Tab and Shift+Tab inside a dialog with nothing focusable: they are prevented and focus stays on the panel", async () => {
    host = document.createElement("div"); document.body.append(host);
    root = createRoot(host);
    await act(async () => root!.render(<Overlay label="Empty" onClose={vi.fn()}><p>Nothing to focus</p></Overlay>));
    const panel = host.querySelector<HTMLElement>('[role="dialog"]')!;
    panel.focus();
    expect(document.activeElement).toBe(panel);
    for (const shiftKey of [false, true]) {
      const tab = await press(panel, "Tab", { shiftKey });
      expect(tab.defaultPrevented, `Tab (shift ${shiftKey}) is swallowed`).toBe(true);
      expect(document.activeElement).toBe(panel);
    }
  });

  it("names a dialog only through its props: neither prop renders an unnamed dialog (pinned gap, see WP-49)", async () => {
    host = document.createElement("div"); document.body.append(host);
    root = createRoot(host);
    await act(async () => root!.render(<Overlay onClose={vi.fn()}><button>Inside</button></Overlay>));
    const panel = host.querySelector('[role="dialog"]')!;
    expect(panel.hasAttribute("aria-label")).toBe(false);
    expect(panel.hasAttribute("aria-labelledby")).toBe(false);
  });
});
