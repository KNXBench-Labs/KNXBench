/** Tests the help tip: reachable by keyboard, described permanently, and still when asked. */
// @vitest-environment happy-dom
import { act } from "react";
import { createRoot } from "react-dom/client";
import { afterEach, describe, expect, it, vi } from "vitest";
import HelpTip from "./HelpTip";
import { messages as enMessages } from "./messages/en";

let host: HTMLDivElement | undefined;

afterEach(() => {
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
const bubble = () => host!.querySelector('[role="tooltip"]') as HTMLElement;

describe("HelpTip", () => {
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
    expect(bubble().id).toBe(described);
    expect(bubble().className).not.toContain("is-open");
  });

  it("keeps the same aria-describedby target once the tooltip opens", async () => {
    await render();
    const described = trigger().getAttribute("aria-describedby");
    await act(async () => {
      trigger().dispatchEvent(new FocusEvent("focusin", { bubbles: true }));
    });
    expect(trigger().getAttribute("aria-describedby")).toBe(described);
    expect(bubble().id).toBe(described);
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
