/** Tests for CommandPalette's overlay behaviour and listbox keyboard navigation. */
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
import { resetUiLanguageForTests, saveUiLanguage } from "./uiLanguage";
import { resetSettingsForTests, settingsStorage } from "./settingsStore";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

let host: HTMLDivElement | undefined;

afterEach(() => {
  host?.remove();
  host = undefined;
  resetSettingsForTests();
  resetUiLanguageForTests();
});

function setQuery(value: string) {
  const input = host!.querySelector<HTMLInputElement>("input")!;
  // A plain `input.value = x` does not make React's controlled `<input>`
  // see a change — same reasoning `BusMonitorPanel.test.tsx`'s
  // `setInputValue` documents; going through the native setter keeps
  // React's own change-tracking from treating this as a no-op.
  const setter = Object.getOwnPropertyDescriptor(window.HTMLInputElement.prototype, "value")!.set!;
  setter.call(input, value);
  input.dispatchEvent(new Event("input", { bubbles: true }));
}

function noopCtx(overrides: Partial<CommandContext> = {}): CommandContext {
  return {
    tree: null,
    newProject: () => {},
    pickProject: () => {},
    openNativeProject: () => {},
    saveProject: () => {},
    saveProjectAs: () => {},
    undo: () => {},
    redo: () => {},
    openSearch: () => {},
    openLog: () => {},
    openBusMonitor: () => {},
    openSettings: () => {},
    openCompanion: () => {},
    openHelp: () => {},
    openCatalog: () => {},
    openIntroduction: () => {},
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
  it("scrolls the enabled active row after skipping disabled commands", async () => {
    const previous = Object.getOwnPropertyDescriptor(HTMLElement.prototype, "scrollIntoView");
    const scroll = vi.fn();
    Object.defineProperty(HTMLElement.prototype, "scrollIntoView", { configurable: true, value: scroll });
    const tree = { can_undo: false, can_redo: false, is_modified: false } as unknown as ProjectTree;
    const { root } = await renderPalette(noopCtx({ tree }));
    try {
      const input = host!.querySelector<HTMLInputElement>("input")!;
      for (let i = 0; i < 4; i++) await act(async () => input.dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowDown", bubbles: true })));
      scroll.mockClear();
      await act(async () => input.dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowDown", bubbles: true })));
      expect(scroll).toHaveBeenCalledOnce();
      expect(scroll.mock.contexts[0]).toBe(host!.querySelector("#palette-option-7"));
      expect(scroll).toHaveBeenCalledWith({ block: "nearest", inline: "nearest" });
      expect(document.activeElement).toBe(input);
    } finally {
      await act(async () => root.unmount());
      if (previous) Object.defineProperty(HTMLElement.prototype, "scrollIntoView", previous);
      else delete (HTMLElement.prototype as { scrollIntoView?: unknown }).scrollIntoView;
    }
  });

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

    // "new-project" is COMMANDS[0], unconditionally enabled (it is the one
    // File action that needs no existing file), so it is the highlight on
    // mount even with no project loaded.
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
    // tree !== null enables new-project, open-project, open-native, save,
    // save-as and search; can_undo/can_redo false disables undo (index 5)
    // and redo (index 6) — the two disabled rows an ArrowDown run must
    // skip on its way from save-as (index 4) to search (index 7). Every
    // index here moved by one when "new-project" took the head of
    // `COMMANDS`; the walk is the same walk.
    const tree = { can_undo: false, can_redo: false, is_modified: false } as unknown as ProjectTree;
    const { root } = await renderPalette(noopCtx({ tree }));

    const input = host!.querySelector("input")!;
    for (let i = 0; i < 5; i++) {
      await act(async () => {
        input.dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowDown", bubbles: true }));
      });
    }

    expect(input.getAttribute("aria-activedescendant")).toBe("palette-option-7");
    expect(host!.querySelector("#palette-option-7")!.getAttribute("aria-selected")).toBe("true");
    expect(host!.querySelector("#palette-option-5")!.getAttribute("aria-selected")).toBe("false");
    expect(host!.querySelector("#palette-option-6")!.getAttribute("aria-selected")).toBe("false");

    root.unmount();
  });

  // T25 task 3: `COMMANDS` now holds `labelKey`s (commandRegistry.ts),
  // resolved to display text here at render time — this pins that the
  // resolution and the filtering both use the *German* label once the UI
  // language is German, not the (English) catalogue key underneath it.
  it("finds a command by its German label when the UI language is German", async () => {
    saveUiLanguage(settingsStorage, "de");
    resetUiLanguageForTests();
    const { root } = await renderPalette(noopCtx());

    await act(async () => {
      setQuery("gängig"); // substring of "Rückgängig" (Undo's German label) only
    });

    // `.provenance-badge` is the separate `shortcutHint` span ("Ctrl+Z")
    // Undo's row also renders — excluded here since this test is about the
    // resolved *label*, not the shortcut hint (covered separately below).
    const labels = Array.from(host!.querySelectorAll(".search-result span:not(.provenance-badge)")).map(
      (el) => el.textContent,
    );
    expect(labels).toEqual(["Rückgängig"]);

    root.unmount();
  });

  // T25 task 5: the shortcut hint itself is now translated too (German ETS
  // convention: "Strg" for Ctrl), so the badge is no longer a verbatim
  // "Ctrl+Z" once the UI language is German.
  it("translates the shortcut hint badge when the UI language is German", async () => {
    saveUiLanguage(settingsStorage, "de");
    resetUiLanguageForTests();
    const { root } = await renderPalette(noopCtx());

    await act(async () => {
      setQuery("gängig"); // substring of "Rückgängig" (Undo's German label) only
    });

    const badge = host!.querySelector(".search-result .provenance-badge");
    expect(badge!.textContent).toBe("Strg+Z");

    root.unmount();
  });
});
