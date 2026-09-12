// @vitest-environment happy-dom
import { act, useRef } from "react";
import type { ReactNode } from "react";
import { createRoot } from "react-dom/client";
import { afterEach, describe, expect, it, vi } from "vitest";
import Overlay from "./Overlay";

let host: HTMLDivElement | undefined;

afterEach(() => {
  host?.remove();
  host = undefined;
});

async function mount(children: ReactNode) {
  host = document.createElement("div");
  document.body.appendChild(host);
  const root = createRoot(host);
  await act(async () => {
    root.render(children);
  });
  return root;
}

describe("Overlay", () => {
  it("renders .search-overlay > .search-panel, a dialog, with children inside", async () => {
    const root = await mount(
      <Overlay onClose={vi.fn()}>
        <p>payload</p>
      </Overlay>,
    );

    const overlay = host!.querySelector(".search-overlay");
    expect(overlay).not.toBeNull();
    const panel = overlay!.querySelector(".search-panel");
    expect(panel).not.toBeNull();
    expect(panel!.getAttribute("role")).toBe("dialog");
    expect(panel!.getAttribute("aria-modal")).toBe("true");
    expect(panel!.textContent).toContain("payload");

    root.unmount();
  });

  it("sets aria-labelledby when labelledBy is passed", async () => {
    const root = await mount(
      <Overlay onClose={vi.fn()} labelledBy="dialog-title">
        <h2 id="dialog-title">Title</h2>
      </Overlay>,
    );

    const panel = host!.querySelector(".search-panel")!;
    expect(panel.getAttribute("aria-labelledby")).toBe("dialog-title");
    expect(panel.hasAttribute("aria-label")).toBe(false);

    root.unmount();
  });

  it("sets aria-label when label is passed", async () => {
    const root = await mount(
      <Overlay onClose={vi.fn()} label="Settings">
        <p>content</p>
      </Overlay>,
    );

    const panel = host!.querySelector(".search-panel")!;
    expect(panel.getAttribute("aria-label")).toBe("Settings");
    expect(panel.hasAttribute("aria-labelledby")).toBe(false);

    root.unmount();
  });

  it("calls onClose on a backdrop click but not on a click inside the panel", async () => {
    const onClose = vi.fn();
    const root = await mount(
      <Overlay onClose={onClose}>
        <button type="button">inside</button>
      </Overlay>,
    );

    const panel = host!.querySelector(".search-panel")!;
    await act(async () => {
      panel.dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });
    expect(onClose).not.toHaveBeenCalled();

    const overlay = host!.querySelector(".search-overlay")!;
    await act(async () => {
      overlay.dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });
    expect(onClose).toHaveBeenCalledTimes(1);

    root.unmount();
  });

  it("calls onClose on Escape fired from an element inside the panel", async () => {
    const onClose = vi.fn();
    const root = await mount(
      <Overlay onClose={onClose}>
        <input aria-label="field" />
      </Overlay>,
    );

    const input = host!.querySelector("input")!;
    await act(async () => {
      input.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true }));
    });
    expect(onClose).toHaveBeenCalledTimes(1);

    root.unmount();
  });

  it("focuses the element pointed to by initialFocusRef on mount", async () => {
    function Harness() {
      const secondRef = useRef<HTMLInputElement | null>(null);
      return (
        <Overlay onClose={vi.fn()} initialFocusRef={secondRef}>
          <input aria-label="first" />
          <input aria-label="second" ref={secondRef} />
        </Overlay>
      );
    }

    const root = await mount(<Harness />);
    const second = host!.querySelector<HTMLInputElement>('input[aria-label="second"]')!;
    expect(document.activeElement).toBe(second);

    root.unmount();
  });

  it("focuses the first focusable element when no initialFocusRef is given", async () => {
    const root = await mount(
      <Overlay onClose={vi.fn()}>
        <input aria-label="first" />
        <input aria-label="second" />
      </Overlay>,
    );

    const first = host!.querySelector<HTMLInputElement>('input[aria-label="first"]')!;
    expect(document.activeElement).toBe(first);

    root.unmount();
  });

  it("traps focus: Tab on the last focusable wraps to the first, Shift+Tab on the first wraps to the last", async () => {
    const root = await mount(
      <Overlay onClose={vi.fn()}>
        <button type="button">first</button>
        <button type="button">middle</button>
        <button type="button">last</button>
      </Overlay>,
    );

    const buttons = host!.querySelectorAll<HTMLButtonElement>("button");
    const first = buttons[0];
    const last = buttons[2];

    // Initial focus landed on `first`; move it to `last` before exercising Tab.
    await act(async () => {
      last.focus();
    });
    expect(document.activeElement).toBe(last);

    await act(async () => {
      last.dispatchEvent(new KeyboardEvent("keydown", { key: "Tab", bubbles: true }));
    });
    expect(document.activeElement).toBe(first);

    await act(async () => {
      first.dispatchEvent(
        new KeyboardEvent("keydown", { key: "Tab", bubbles: true, shiftKey: true }),
      );
    });
    expect(document.activeElement).toBe(last);

    root.unmount();
  });

  it("restores focus to the previously focused element after unmount", async () => {
    const outside = document.createElement("button");
    outside.textContent = "outside";
    document.body.appendChild(outside);
    outside.focus();
    expect(document.activeElement).toBe(outside);

    const root = await mount(
      <Overlay onClose={vi.fn()}>
        <button type="button">inside</button>
      </Overlay>,
    );
    expect(document.activeElement).toBe(host!.querySelector("button"));

    await act(async () => {
      root.unmount();
    });
    expect(document.activeElement).toBe(outside);

    outside.remove();
  });

  it("does not call onClose for Escape fired after unmount", async () => {
    const onClose = vi.fn();
    const root = await mount(
      <Overlay onClose={onClose}>
        <button type="button">inside</button>
      </Overlay>,
    );

    await act(async () => {
      root.unmount();
    });

    await act(async () => {
      document.body.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true }));
    });
    expect(onClose).not.toHaveBeenCalled();
  });
});
