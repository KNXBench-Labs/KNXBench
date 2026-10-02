/** Tests the help tip: reachable by keyboard, described permanently, and still when asked. */
// @vitest-environment happy-dom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, describe, expect, it, vi } from "vitest";
import HelpTip from "./HelpTip";
import { messages as enMessages } from "./messages/en";

let host: HTMLDivElement | undefined;
let currentRoot: Root | undefined;

afterEach(async () => {
  await act(async () => currentRoot?.unmount());
  currentRoot = undefined;
  host?.remove();
  host = undefined;
  document.documentElement.removeAttribute("data-motion-level");
});

// Rendered inside an ancestor that owns its own `onKeyDown`, because that
// is the shape the tip actually ships in: `Overlay` puts a React keydown
// handler on the dialog around it, and "Escape closes the tip, not the
// dialog" is a claim about React's synthetic propagation, not the DOM's.
async function render(onOuterKeyDown?: () => void) {
  host = document.createElement("div");
  document.body.appendChild(host);
  const root = createRoot(host);
  currentRoot = root;
  await act(async () => {
    root.render(
      <div onKeyDown={onOuterKeyDown}>
        <HelpTip labelKey="help.tip.comFlags.label" textKey="help.tip.comFlags.text" />
      </div>,
    );
  });
  return root;
}

const trigger = () => host!.querySelector("button")!;
const bubble = () => document.body.querySelector(".help-tip-bubble") as HTMLElement;
const description = () => host!.querySelector('[role="tooltip"]') as HTMLElement;

describe("HelpTip", () => {
  it("repositions on resize and captured scroll and releases those listeners on unmount", async () => {
    const root = await render();
    const anchor = vi.spyOn(trigger(), "getBoundingClientRect");
    anchor.mockReturnValue({ left: 40, top: 40, bottom: 60 } as DOMRect);
    vi.spyOn(bubble(), "getBoundingClientRect").mockReturnValue({ width: 200, height: 80 } as DOMRect);
    await act(async () => trigger().dispatchEvent(new FocusEvent("focusin", { bubbles: true })));
    expect(bubble().style.left).toBe("40px");
    anchor.mockReturnValue({ left: 80, top: 40, bottom: 60 } as DOMRect);
    await act(async () => window.dispatchEvent(new Event("resize")));
    expect(bubble().style.left).toBe("80px");
    anchor.mockReturnValue({ left: 120, top: 40, bottom: 60 } as DOMRect);
    await act(async () => trigger().dispatchEvent(new Event("scroll", { bubbles: false })));
    expect(bubble().style.left).toBe("120px");
    const remove = vi.spyOn(window, "removeEventListener");
    await act(async () => root.unmount());
    expect(remove).toHaveBeenCalledWith("resize", expect.any(Function));
    expect(remove).toHaveBeenCalledWith("scroll", expect.any(Function), true);
    expect(document.body.querySelector(".help-tip-bubble")).toBeNull();
  });

  it.each([1, 1.5])("places a focused edge tooltip inside the viewport at scale %s", async (scale) => {
    const previous = document.documentElement.style.getPropertyValue("--app-ui-scale");
    document.documentElement.style.setProperty("--app-ui-scale", String(scale));
    const root = await render();
    try {
      vi.spyOn(trigger(), "getBoundingClientRect").mockReturnValue({ left: window.innerWidth - 20, top: window.innerHeight - 30, bottom: window.innerHeight - 12 } as DOMRect);
      vi.spyOn(bubble(), "getBoundingClientRect").mockReturnValue({ width: 200, height: 80 } as DOMRect);
      await act(async () => trigger().dispatchEvent(new FocusEvent("focusin", { bubbles: true })));
      expect(Number.parseFloat(bubble().style.left)).toBeCloseTo((window.innerWidth - 208) / scale);
      expect(Number.parseFloat(bubble().style.top)).toBeCloseTo((window.innerHeight - 116) / scale);
      const described = trigger().getAttribute("aria-describedby");
      await act(async () => trigger().dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true })));
      expect(bubble().style.left).toBe("0px");
      expect(bubble().style.top).toBe("0px");
      expect(description().id).toBe(described);
    } finally {
      await act(async () => root.unmount());
      if (previous) document.documentElement.style.setProperty("--app-ui-scale", previous);
      else document.documentElement.style.removeProperty("--app-ui-scale");
    }
  });

  it("keeps the permanent description beside its trigger while painting the bubble outside clipped ancestors", async () => {
    const root = await render();
    try {
      const described = trigger().getAttribute("aria-describedby")!;
      const description = document.getElementById(described)!;
      expect(host!.contains(description)).toBe(true);
      expect(description.classList.contains("help-tip-description")).toBe(true);
      expect(description.textContent).toBe(enMessages["help.tip.comFlags.text"]);
      const painted = document.body.querySelector<HTMLElement>(".help-tip-bubble")!;
      expect(painted.parentElement).toBe(document.body);
      expect(painted.getAttribute("aria-hidden")).toBe("true");
    } finally { await act(async () => root.unmount()); }
  });

  it("registers its topic for F1 and dispatches the same topic on click", async () => {
    host = document.createElement("div");
    document.body.appendChild(host);
    const root = createRoot(host);
    currentRoot = root;
    const received = vi.fn();
    window.addEventListener("knxbench:open-help", received);
    try {
      await act(async () => root.render(<HelpTip labelKey="help.tip.comFlags.label" textKey="help.tip.comFlags.text" topicId="comObjectFlags" />));
      expect(trigger().getAttribute("data-help-topic")).toBe("comObjectFlags");
      await act(async () => trigger().click());
      expect((received.mock.calls[0][0] as CustomEvent).detail).toEqual({ topicId: "comObjectFlags" });
    } finally {
      window.removeEventListener("knxbench:open-help", received);
      await act(async () => root.unmount());
    }
  });

  it("names itself and describes itself with the catalogue text", async () => {
    await render();
    expect(trigger().getAttribute("aria-label")).toBe(enMessages["help.tip.comFlags.label"]);
    expect(bubble().textContent).toBe(enMessages["help.tip.comFlags.text"]);
  });

  // The whole reason this component exists instead of a `title` attribute.
  // The association is unconditional: a screen reader reading the trigger
  // gets the description whether or not the bubble is painted, so this
  // must hold in the closed state too.
  it("points aria-describedby at the tooltip while it is closed", async () => {
    await render();
    const described = trigger().getAttribute("aria-describedby");
    expect(described).toBeTruthy();
    expect(description().id).toBe(described);
    expect(bubble().className).not.toContain("is-open");
  });

  it("keeps the same aria-describedby target once the tooltip opens", async () => {
    await render();
    const described = trigger().getAttribute("aria-describedby");
    await act(async () => {
      trigger().dispatchEvent(new FocusEvent("focusin", { bubbles: true }));
    });
    expect(trigger().getAttribute("aria-describedby")).toBe(described);
    expect(description().id).toBe(described);
  });

  // A tooltip that appears only on hover is unreachable from a keyboard.
  // React's `onFocus` is the delegated `focusin` event, so this is the real
  // path a Tab press takes.
  it("shows the bubble on focus, not only on hover", async () => {
    await render();
    expect(bubble().className).not.toContain("is-open");
    await act(async () => {
      trigger().dispatchEvent(new FocusEvent("focusin", { bubbles: true }));
    });
    expect(bubble().className).toContain("is-open");
  });

  it("hides the bubble again on blur", async () => {
    await render();
    await act(async () => {
      trigger().dispatchEvent(new FocusEvent("focusin", { bubbles: true }));
    });
    await act(async () => {
      trigger().dispatchEvent(new FocusEvent("focusout", { bubbles: true }));
    });
    expect(bubble().className).not.toContain("is-open");
  });

  // React derives `onPointerEnter`/`onPointerLeave` from the delegated
  // `pointerover`/`pointerout` pair, so those are the events a real mouse
  // produces and the ones worth dispatching here.
  it("shows the bubble on pointer enter and hides it on pointer leave", async () => {
    await render();
    await act(async () => {
      trigger().dispatchEvent(new Event("pointerover", { bubbles: true }));
    });
    expect(bubble().className).toContain("is-open");
    await act(async () => {
      trigger().dispatchEvent(new Event("pointerout", { bubbles: true }));
    });
    expect(bubble().className).not.toContain("is-open");
  });

  it("dismisses an open bubble with Escape without closing anything around it", async () => {
    const outer = vi.fn();
    await render(outer);
    await act(async () => {
      trigger().dispatchEvent(new FocusEvent("focusin", { bubbles: true }));
    });
    await act(async () => {
      trigger().dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true }));
    });
    expect(bubble().className).not.toContain("is-open");
    expect(outer).not.toHaveBeenCalled();
  });

  it("lets Escape through to the dialog around it when no bubble is open", async () => {
    const outer = vi.fn();
    await render(outer);
    await act(async () => {
      trigger().dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true }));
    });
    expect(outer).toHaveBeenCalledTimes(1);
  });

  it("marks the bubble still when the application's motion level is off", async () => {
    document.documentElement.setAttribute("data-motion-level", "off");
    await render();
    expect(bubble().className).toContain("is-still");
  });

  it("does not mark the bubble still at a motion level that allows movement", async () => {
    document.documentElement.setAttribute("data-motion-level", "standard");
    await render();
    expect(bubble().className).not.toContain("is-still");
  });
});
