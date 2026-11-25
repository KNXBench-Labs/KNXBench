/** Pins the RenameWorkbench rename dialog against the shared dialog checklist (WP-49.4c). */
// @vitest-environment happy-dom
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, describe, expect, it, vi } from "vitest";
import * as api from "./api";
import RenameWorkbench from "./RenameWorkbench";
import type { ProjectTree } from "./bindings/ProjectTree";
import { press, setNativeValue } from "./test/comboboxChecklist";
import { describeDialogChecklist, type DialogAdapter } from "./test/dialogChecklist";

vi.mock("./api", () => ({ renameEntity: vi.fn(), errorMessage: (e: unknown) => String(e) }));
(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const tree: ProjectTree = { schema_version: 11, errors: 0, warnings: 0,
  can_undo: false, can_redo: false, is_modified: false, group_address_style: "ThreeLevel",
  server_incarnation: "rename", snapshot_revision: 1, project_incarnation: 0,
  installations: [{ id: 1, name: "I", topology: [], buildings: [], unassigned: [], group_ranges: [],
    group_addresses: [{ id: 2, name: "Original", address: "1/1/1", range: null, dpts: [], links: [] }] }] };

let host: HTMLDivElement | undefined;
let root: Root | undefined;
let resolveRename: ((tree: ProjectTree) => void) | undefined;
const opener = () => host!.querySelector<HTMLElement>("[data-rename-kind]")!;
const dialog = () => host!.querySelector<HTMLElement>('[role="dialog"]');

const adapter: DialogAdapter = {
  name: "RenameWorkbench rename dialog",
  async mount(parentKeyDown) {
    host = document.createElement("div");
    document.body.append(host);
    root = createRoot(host);
    await act(async () => root!.render(<div onKeyDown={parentKeyDown}>
      <RenameWorkbench tree={tree} scope={0} onApplied={vi.fn()}>
        <button data-rename-kind="group_address" data-rename-id="2">Original</button>
      </RenameWorkbench></div>));
  },
  async unmount() {
    await act(async () => root!.unmount());
    host!.remove(); host = undefined; root = undefined; resolveRename = undefined;
    vi.clearAllMocks();
  },
  async open() { await act(async () => { opener().dispatchEvent(new KeyboardEvent("keydown", { key: "F2", bubbles: true, cancelable: true })); }); },
  dialog: () => dialog()!,
  isOpen: () => dialog() !== null,
  opener,
  background: () => [opener()],
  initialFocus: () => host!.querySelector<HTMLElement>("input[data-rename-name]")!,
  tabStops: () => [...dialog()!.querySelectorAll<HTMLElement>("input, button")].filter(el => !(el as HTMLInputElement).disabled),
  nameSource: "aria-label",
  describedBy: "none",
  shell: "overlay",
  // Escape in the name field belongs to the field: it cancels, and prevents and stops the key.
  escape: { closes: true, prevented: true, stopped: true },
  async busy() {
    vi.mocked(api.renameEntity).mockImplementation(() => new Promise(done => { resolveRename = done; }));
    const input = host!.querySelector<HTMLInputElement>("input[data-rename-name]")!;
    input.focus();
    await act(async () => setNativeValue(input, "Pending"));
    await act(async () => { input.dispatchEvent(new KeyboardEvent("keydown", { key: "Enter", bubbles: true, cancelable: true })); });
  },
};

describeDialogChecklist(adapter);

describe("RenameWorkbench dialog beyond the shared checklist", () => {
  afterEach(async () => { if (root) await adapter.unmount(); });

  it("lets the shell take Escape from a button: closes, stopped, and prevented (decided, owner Q14, 23 Nov 2026; the field handles its own Escape the same way)", async () => {
    const parent = vi.fn();
    await adapter.mount(parent);
    opener().focus();
    await adapter.open();
    const cancel = [...dialog()!.querySelectorAll<HTMLElement>("button")].find(b => b.textContent === "Cancel")!;
    cancel.focus();
    const before = parent.mock.calls.length;
    const escape = await press(cancel, "Escape");
    expect(dialog()).toBeNull();
    expect(escape.defaultPrevented).toBe(true);
    expect(parent.mock.calls.length - before).toBe(0);
    expect(document.activeElement).toBe(opener());
  });

  it("keeps the dialog open on Escape from the panel while the commit is pending, then closes it and returns focus", async () => {
    await adapter.mount(() => {});
    opener().focus();
    await adapter.open();
    await adapter.busy!();
    expect(api.renameEntity).toHaveBeenCalledTimes(1);
    await press(dialog()!, "Escape");
    expect(dialog()).not.toBeNull();
    await act(async () => resolveRename!({ ...tree, snapshot_revision: 2 }));
    expect(dialog()).toBeNull();
    expect(document.activeElement).toBe(opener());
  });
});

// Decided behaviour (owner Q17, 23 Nov 2026, WP-49.5): the renamed tree row is the
// Overlay's restoreFocusRef, so after closing a dialog opened from the context-menu
// item focus returns to the row (the menu item itself unmounts on open). F2 is the
// positive control: the row was the opener and gets focus back directly.
it("returns focus to the renamed tree row after closing a dialog opened from the context-menu item (owner Q17, 23 Nov 2026, decided)", async () => {
  await adapter.mount(() => {});
  opener().focus();
  await act(async () => { opener().dispatchEvent(new MouseEvent("contextmenu", { bubbles: true, cancelable: true, clientX: 10, clientY: 10 })); });
  const item = host!.querySelector<HTMLElement>('[role="menuitem"]')!;
  expect(document.activeElement).toBe(item);
  await act(async () => item.click());
  expect(dialog()).not.toBeNull();
  await press(document.activeElement as HTMLElement, "Escape");
  expect(dialog()).toBeNull();
  expect(document.activeElement, "owner Q17: the tree row replaces the vanished menu item").toBe(opener());
  await adapter.unmount();
});

it("returns focus to the tree row after a successful rename too (owner Q17; the row identity may change, the element is the same row)", async () => {
  await adapter.mount(() => {});
  opener().focus();
  await act(async () => { opener().dispatchEvent(new MouseEvent("contextmenu", { bubbles: true, cancelable: true, clientX: 10, clientY: 10 })); });
  await act(async () => host!.querySelector<HTMLElement>('[role="menuitem"]')!.click());
  expect(dialog()).not.toBeNull();
  vi.mocked(api.renameEntity).mockResolvedValue({ ...tree, snapshot_revision: 2 });
  const input = host!.querySelector<HTMLInputElement>("input[data-rename-name]")!;
  await act(async () => setNativeValue(input, "Renamed"));
  await act(async () => { input.dispatchEvent(new KeyboardEvent("keydown", { key: "Enter", bubbles: true, cancelable: true })); });
  // The rename promise resolves on a microtask; flush it without fake timers.
  await act(async () => { await Promise.resolve(); });
  expect(dialog()).toBeNull();
  expect(document.activeElement).toBe(opener());
  await adapter.unmount();
});

it("keeps focus on the page when the row is gone at close (the shell's existing fallback, owner Q17)", async () => {
  await adapter.mount(() => {});
  opener().focus();
  await act(async () => { opener().dispatchEvent(new MouseEvent("contextmenu", { bubbles: true, cancelable: true, clientX: 10, clientY: 10 })); });
  await act(async () => host!.querySelector<HTMLElement>('[role="menuitem"]')!.click());
  expect(dialog()).not.toBeNull();
  await act(async () => opener().remove());
  await press(document.activeElement as HTMLElement, "Escape");
  expect(dialog()).toBeNull();
  expect(document.activeElement).toBe(document.body);
  await adapter.unmount();
});
