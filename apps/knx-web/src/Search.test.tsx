// @vitest-environment happy-dom
//
// Search.tsx's own logic (grouping, arrow-key clamping) is already exercised
// indirectly via searchMatch.test.ts and treeUtils.test.ts; this file mocks
// `buildSearchIndex` to a fixed index so it only has to cover what changed
// in T31: the Overlay migration (Escape closes) and the new listbox ARIA
// wiring.
import { act } from "react";
import { createRoot } from "react-dom/client";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { ProjectTree } from "./bindings/ProjectTree";
import type { SearchEntry } from "./treeUtils";

const INDEX: SearchEntry[] = [
  { kind: "device", id: 1, label: "Dimmer hallway", address: "1.1.2" },
  { kind: "device", id: 2, label: "Living room dimmer", address: "1.1.1" },
];

vi.mock("./treeUtils", () => ({
  buildSearchIndex: () => INDEX,
}));

import Search from "./Search";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

let host: HTMLDivElement | undefined;

afterEach(() => {
  host?.remove();
  host = undefined;
});

async function renderSearch(onClose = vi.fn(), onSelect = vi.fn()) {
  host = document.createElement("div");
  document.body.appendChild(host);
  const root = createRoot(host);
  await act(async () => {
    root.render(<Search tree={{} as ProjectTree} onSelect={onSelect} onClose={onClose} />);
  });
  return { root, onClose, onSelect };
}

function setQuery(value: string) {
  const input = host!.querySelector<HTMLInputElement>("input")!;
  const setter = Object.getOwnPropertyDescriptor(window.HTMLInputElement.prototype, "value")!.set!;
  setter.call(input, value);
  input.dispatchEvent(new Event("input", { bubbles: true }));
}

describe("Search", () => {
  it("closes on Escape (the Overlay shell's handler, not a local one)", async () => {
    const { root, onClose } = await renderSearch();

    const input = host!.querySelector("input")!;
    await act(async () => {
      input.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true }));
    });
    expect(onClose).toHaveBeenCalledTimes(1);

    root.unmount();
  });

  it("marks the highlighted option aria-selected and names it via aria-activedescendant", async () => {
    const { root } = await renderSearch();

    await act(async () => {
      setQuery("dimmer");
    });

    const input = host!.querySelector("input")!;
    expect(input.getAttribute("role")).toBe("combobox");
    expect(input.getAttribute("aria-haspopup")).toBe("listbox");
    // matchEntries ranks the starts-with match first: "Dimmer hallway".
    expect(input.getAttribute("aria-activedescendant")).toBe("search-option-0");

    const first = host!.querySelector("#search-option-0")!;
    const second = host!.querySelector("#search-option-1")!;
    expect(first.getAttribute("role")).toBe("option");
    expect(first.getAttribute("aria-selected")).toBe("true");
    expect(second.getAttribute("aria-selected")).toBe("false");

    root.unmount();
  });

  it("ArrowDown past the last result stays on the last", async () => {
    const { root } = await renderSearch();

    await act(async () => {
      setQuery("dimmer");
    });

    const input = host!.querySelector("input")!;
    for (let i = 0; i < 5; i++) {
      await act(async () => {
        input.dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowDown", bubbles: true }));
      });
    }

    expect(input.getAttribute("aria-activedescendant")).toBe("search-option-1");
    expect(host!.querySelector("#search-option-1")!.getAttribute("aria-selected")).toBe("true");

    root.unmount();
  });
});
