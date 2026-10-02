/** Tests shared modal focus, background isolation and viewport resizing. */
// @vitest-environment happy-dom
import { readFileSync } from "node:fs";
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
  it("skips descendants of hidden, inert and aria-hidden ancestors for initial focus and Tab wrap", async () => {
    const root = await mount(<Overlay onClose={vi.fn()}>
      <div hidden><button>Hidden</button></div>
      <div inert><button>Inert</button></div>
      <div aria-hidden="true"><button>Excluded</button></div>
      <button data-visible="true">Visible</button>
    </Overlay>);
    try {
      const visible = host!.querySelector<HTMLElement>("[data-visible]")!;
      expect(document.activeElement).toBe(visible);
      await act(async () => visible.dispatchEvent(new KeyboardEvent("keydown", { key: "Tab", shiftKey: true, bubbles: true })));
      expect(document.activeElement).toBe(visible);
      await act(async () => visible.dispatchEvent(new KeyboardEvent("keydown", { key: "Tab", bubbles: true })));
      expect(document.activeElement).toBe(visible);
    } finally { await act(async () => root.unmount()); }
  });

  it("excludes asynchronously added background content and restores it after close", async () => {
    const root = await mount(<Overlay onClose={vi.fn()}><button>Dialog</button></Overlay>);
    const added = document.createElement("button");
    added.textContent = "Late background";
    try {
      await act(async () => { document.body.appendChild(added); await Promise.resolve(); });
      expect(added.hasAttribute("inert")).toBe(true);
      expect(added.getAttribute("aria-hidden")).toBe("true");
      await act(async () => root.unmount());
      expect(added.hasAttribute("inert")).toBe(false);
      expect(added.hasAttribute("aria-hidden")).toBe(false);
    } finally {
      await act(async () => root.unmount());
      added.remove();
    }
  });

  it("keeps background excluded when a lower dialog unmounts before the top dialog", async () => {
    const outside = document.createElement("button");
    document.body.appendChild(outside);
    outside.focus();
    const lower = await mount(<Overlay label="Lower" onClose={vi.fn()}><button>Lower control</button></Overlay>);
    const upperHost = document.createElement("div");
    document.body.appendChild(upperHost);
    const upper = createRoot(upperHost);
    try {
      await act(async () => upper.render(<Overlay label="Upper" onClose={vi.fn()}><button>Upper control</button></Overlay>));
      expect(upperHost.querySelector('[role="dialog"]')!.closest("[inert], [aria-hidden='true']")).toBeNull();
      await act(async () => {
        lower.unmount();
        // Assert before MutationObserver delivery: it must not repair an
        // already-exposed background after an out-of-order close.
        expect(outside.hasAttribute("inert")).toBe(true);
        expect(outside.getAttribute("aria-hidden")).toBe("true");
      });
      expect(outside.hasAttribute("inert")).toBe(true);
      expect(outside.getAttribute("aria-hidden")).toBe("true");
      await act(async () => upper.unmount());
      expect(outside.hasAttribute("inert")).toBe(false);
      expect(outside.hasAttribute("aria-hidden")).toBe(false);
      expect(document.activeElement).toBe(outside);
    } finally {
      await act(async () => { lower.unmount(); upper.unmount(); });
      upperHost.remove(); outside.remove();
    }
  });

  it("excludes background branches without hiding its ancestors and restores existing attributes on close", async () => {
    const outside = document.createElement("div");
    outside.setAttribute("inert", "preexisting");
    outside.setAttribute("aria-hidden", "false");
    document.body.appendChild(outside);
    const root = await mount(<><main data-background="true"><button>Background</button></main><Overlay onClose={vi.fn()}><button>Dialog</button></Overlay></>);
    try {
      const background = host!.querySelector("main")!;
      const panel = host!.querySelector<HTMLElement>('[role="dialog"]')!;
      expect(background.hasAttribute("inert")).toBe(true);
      expect(background.getAttribute("aria-hidden")).toBe("true");
      expect(outside.getAttribute("aria-hidden")).toBe("true");
      expect(panel.closest("[inert], [aria-hidden='true']")).toBeNull();
      expect(document.activeElement).toBe(panel.querySelector("button"));
      await act(async () => root.render(<main><button>Background</button></main>));
      expect(host!.querySelector("main")!.hasAttribute("inert")).toBe(false);
      expect(host!.querySelector("main")!.hasAttribute("aria-hidden")).toBe(false);
      expect(outside.getAttribute("inert")).toBe("preexisting");
      expect(outside.getAttribute("aria-hidden")).toBe("false");
    } finally {
      await act(async () => root.unmount());
      outside.remove();
    }
  });

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

  it("does not dismiss a resizable dialog when a pointer drag starts inside and ends on the backdrop", async () => {
    const onClose = vi.fn();
    const root = await mount(
      <Overlay onClose={onClose} resizable={{ width: 800, height: 640 }}>
        <p>Resize grip</p>
      </Overlay>,
    );
    const panel = host!.querySelector(".search-panel")!;
    const backdrop = host!.querySelector(".search-overlay")!;
    await act(async () => {
      panel.dispatchEvent(new Event("pointerdown", { bubbles: true }));
      backdrop.dispatchEvent(new Event("pointerup", { bubbles: true }));
      backdrop.dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });
    expect(onClose).not.toHaveBeenCalled();

    await act(async () => {
      backdrop.dispatchEvent(new Event("pointerdown", { bubbles: true }));
      backdrop.dispatchEvent(new MouseEvent("click", { bubbles: true }));
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

  it("lets an opted-in dialog resize from the keyboard without losing focus containment or Escape", async () => {
    const outside = document.createElement("button");
    document.body.appendChild(outside);
    outside.focus();
    const onClose = vi.fn();
    const root = await mount(
      <Overlay label="Resizable" onClose={onClose} resizable={{ width: 900, height: 620 }}>
        <input aria-label="first" />
      </Overlay>,
    );
    const panel = host!.querySelector<HTMLElement>(".search-panel")!;
    const handle = host!.querySelector<HTMLButtonElement>(".overlay-resize-key")!;
    expect(panel.classList.contains("search-panel-resizable")).toBe(true);
    expect(host!.querySelector(".search-overlay")?.classList.contains("search-overlay-resizable")).toBe(true);
    expect(handle.getAttribute("aria-label")).toMatch(/resize.*arrow/i);
    expect(document.activeElement).toBe(host!.querySelector("input"));

    vi.spyOn(panel, "getBoundingClientRect")
      .mockReturnValueOnce({ width: 500, height: 300 } as DOMRect)
      .mockReturnValue({ width: 524, height: 300 } as DOMRect);
    await act(async () => handle.dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowRight", bubbles: true })));
    await act(async () => handle.dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowDown", bubbles: true })));
    expect(panel.style.width).toBe("524px");
    expect(panel.style.height).toBe("324px");

    handle.focus();
    await act(async () => handle.dispatchEvent(new KeyboardEvent("keydown", { key: "Tab", bubbles: true })));
    expect(document.activeElement).toBe(host!.querySelector("input"));
    await act(async () => handle.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true })));
    expect(onClose).toHaveBeenCalledTimes(1);
    await act(async () => root.unmount());
    expect(document.activeElement).toBe(outside);
    outside.remove();
  });

  it("uses layout pixels when resizing under application zoom", async () => {
    const previous = document.documentElement.style.getPropertyValue("--app-ui-scale");
    document.documentElement.style.setProperty("--app-ui-scale", "1.5");
    const root = await mount(
      <Overlay onClose={vi.fn()} resizable={{ width: 900, height: 620 }}>
        <p>zoomed dialog</p>
      </Overlay>,
    );
    try {
      const panel = host!.querySelector<HTMLElement>(".search-panel")!;
      const handle = host!.querySelector<HTMLButtonElement>(".overlay-resize-key")!;
      vi.spyOn(panel, "getBoundingClientRect")
        .mockReturnValueOnce({ width: 600, height: 450 } as DOMRect)
        .mockReturnValue({ width: 564, height: 450 } as DOMRect);
      await act(async () => handle.dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowLeft", bubbles: true })));
      expect(panel.style.width).toBe("376px"); // 600 visual px / 1.5 - 24 layout px
      await act(async () => handle.dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowUp", bubbles: true })));
      expect(panel.style.height).toBe("276px"); // 450 / 1.5 - 24
    } finally {
      await act(async () => root.unmount());
      if (previous) document.documentElement.style.setProperty("--app-ui-scale", previous);
      else document.documentElement.style.removeProperty("--app-ui-scale");
    }
  });

  it("clamps keyboard resizing to the viewport and offers a native pointer grip with contained scrolling", async () => {
    const root = await mount(
      <Overlay label="Bounds" onClose={vi.fn()} resizable={{ width: 900, height: 620 }}>
        <p>long content</p>
      </Overlay>,
    );
    const panel = host!.querySelector<HTMLElement>(".search-panel")!;
    const handle = host!.querySelector<HTMLButtonElement>(".overlay-resize-key")!;
    vi.spyOn(panel, "getBoundingClientRect")
      .mockReturnValueOnce({ width: window.innerWidth - 35, height: window.innerHeight - 35 } as DOMRect)
      .mockReturnValue({ width: window.innerWidth - 32, height: window.innerHeight - 35 } as DOMRect);
    await act(async () => handle.dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowRight", bubbles: true })));
    expect(panel.style.width).toBe(`${window.innerWidth - 32}px`);
    await act(async () => handle.dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowDown", bubbles: true })));
    expect(panel.style.width).toBe(`${window.innerWidth - 32}px`);
    expect(panel.style.height).toBe(`${window.innerHeight - 32}px`);
    const css = readFileSync("src/styles.css", "utf8");
    const rule = css.match(/\.search-panel-resizable \{([^}]*)\}/)?.[1] ?? "";
    expect(rule).toMatch(/resize:\s*both/);
    expect(rule).toMatch(/overflow:\s*auto/);
    expect(rule).toMatch(/max-width:\s*100%/);
    expect(rule).toMatch(/max-height:\s*100%/);
    await act(async () => root.unmount());
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
