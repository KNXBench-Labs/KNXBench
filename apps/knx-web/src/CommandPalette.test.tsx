// @vitest-environment happy-dom
//
// commandRegistry.test.ts already covers filtering and enablement logic in
// isolation; this file only has to cover what T31 changed: the Overlay
// migration (Escape closes) and the new listbox ARIA wiring, including
// arrow-key traversal skipping a disabled row.
import { act } from "react";
import { createRoot } from "react-dom/client";
import { afterEach, describe, expect, it, vi } from "vitest";
import CommandPalette from "./CommandPalette";
import type { CommandContext } from "./commandRegistry";
import type { ProjectTree } from "./bindings/ProjectTree";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

let host: HTMLDivElement | undefined;

afterEach(() => {
  host?.remove();
  host = undefined;
});

function noopCtx(overrides: Partial<CommandContext> = {}): CommandContext {
  return {
    tree: null,
    pickProject: () => {},
    openNativeProject: () => {},
    saveProject: () => {},
    saveProjectAs: () => {},
    undo: () => {},
    redo: () => {},
    openSearch: () => {},
    ...overrides,
  };
}

async function renderPalette(ctx: CommandContext, onClose = vi.fn()) {
  host = document.createElement("div");
  document.body.appendChild(host);
  const root = createRoot(host);
  await act(async () => {
    root.render(<CommandPalette ctx={ctx} onClose={onClose} />);
  });
  return { root, onClose };
}

describe("CommandPalette", () => {
  it("closes on Escape (the Overlay shell's handler, not a local one)", async () => {
    const { root, onClose } = await renderPalette(noopCtx());

    const input = host!.querySelector("input")!;
    await act(async () => {
      input.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true }));
    });
    expect(onClose).toHaveBeenCalledTimes(1);

    root.unmount();
  });

  it("marks the highlighted row aria-selected and names it via aria-activedescendant", async () => {
    const { root } = await renderPalette(noopCtx());

    // "open-project" is COMMANDS[0], unconditionally enabled, so it is the
    // highlight on mount even with no project loaded.
    const input = host!.querySelector("input")!;
    expect(input.getAttribute("role")).toBe("combobox");
    expect(input.getAttribute("aria-haspopup")).toBe("listbox");
    expect(input.getAttribute("aria-activedescendant")).toBe("palette-option-0");

    const first = host!.querySelector("#palette-option-0")!;
    expect(first.getAttribute("role")).toBe("option");
    expect(first.getAttribute("aria-selected")).toBe("true");

    root.unmount();
  });

  it("ArrowDown skips a disabled row", async () => {
    // tree !== null enables open-project, open-native, save, save-as and
    // search; can_undo/can_redo false disables undo (index 4) and redo
    // (index 5) — the two disabled rows an ArrowDown run must skip on its
    // way from save-as (index 3) to search (index 6).
    const tree = { can_undo: false, can_redo: false } as unknown as ProjectTree;
    const { root } = await renderPalette(noopCtx({ tree }));

    const input = host!.querySelector("input")!;
    for (let i = 0; i < 4; i++) {
      await act(async () => {
        input.dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowDown", bubbles: true }));
      });
    }

    expect(input.getAttribute("aria-activedescendant")).toBe("palette-option-6");
    expect(host!.querySelector("#palette-option-6")!.getAttribute("aria-selected")).toBe("true");
    expect(host!.querySelector("#palette-option-4")!.getAttribute("aria-selected")).toBe("false");
    expect(host!.querySelector("#palette-option-5")!.getAttribute("aria-selected")).toBe("false");

    root.unmount();
  });
});
