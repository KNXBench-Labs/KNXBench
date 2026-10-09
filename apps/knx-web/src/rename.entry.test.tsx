/** Shared rename keyboard and context-menu entry points never write on open. */
// @vitest-environment happy-dom
import { act } from "react";
import { createRoot } from "react-dom/client";
import { expect, it, vi } from "vitest";
import RenameWorkbench from "./RenameWorkbench";
import * as api from "./api";
import type { ProjectTree } from "./bindings/ProjectTree";
vi.mock("./api", () => ({ renameEntity: vi.fn(), errorMessage: (e: unknown) => String(e) }));
(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;
export const tree: ProjectTree = { schema_version: 11, errors: 0, warnings: 0,
  can_undo: false, can_redo: false, is_modified: false, group_address_style: "ThreeLevel",
  server_incarnation: "rename", snapshot_revision: 1, project_incarnation: 0,
  installations: [{ id: 1, name: "I", topology: [], buildings: [], unassigned: [], group_ranges: [],
    group_addresses: [{ id: 2, name: "Original", address: "1/1/1", range: null, dpts: [], links: [] }] }] };
it("Escape cannot dismiss a rename whose commit is still pending", async () => {
  let resolve!: (value: ProjectTree) => void;
  vi.mocked(api.renameEntity).mockImplementation(() => new Promise((done) => { resolve = done; }));
  const applied = vi.fn(); const host = document.createElement("div"); document.body.append(host);
  const root = createRoot(host);
  try {
    await act(async () => root.render(<RenameWorkbench tree={tree} scope={0} onApplied={applied}>
      <button data-rename-kind="group_address" data-rename-id="2">Original</button>
    </RenameWorkbench>));
    await act(async () => host.querySelector("button")!.dispatchEvent(new KeyboardEvent("keydown", { key: "F2", bubbles: true })));
    const input = host.querySelector<HTMLInputElement>("input")!;
    await act(async () => {
      input.focus(); Object.getOwnPropertyDescriptor(window.HTMLInputElement.prototype, "value")!.set!.call(input, "Pending");
      input.dispatchEvent(new Event("input", { bubbles: true }));
    });
    await act(async () => input.dispatchEvent(new KeyboardEvent("keydown", { key: "Enter", bubbles: true })));
    await act(async () => host.querySelector('[role="dialog"]')!.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true })));
    expect(host.querySelector('[role="dialog"]')).not.toBeNull();
    await act(async () => resolve({ ...tree, snapshot_revision: 2 }));
    expect(applied).toHaveBeenCalledTimes(1); expect(host.querySelector('[role="dialog"]')).toBeNull();
  } finally { await act(async () => root.unmount()); host.remove(); vi.clearAllMocks(); }
});
it("opens a canonical list device absent from placement branches without guessing its name", async () => {
  const host = document.createElement("div"); document.body.append(host); const root = createRoot(host);
  try {
    await act(async () => root.render(<RenameWorkbench tree={tree} scope={0} onApplied={vi.fn()}>
      <button data-rename-kind="device" data-rename-id="99" data-rename-value="Unplaced canonical device">Unplaced canonical device</button>
    </RenameWorkbench>));
    await act(async () => host.querySelector("button")!.dispatchEvent(new KeyboardEvent("keydown", { key: "F2", bubbles: true })));
    expect(host.querySelector<HTMLInputElement>('input[data-rename-name]')?.value).toBe("Unplaced canonical device");
  } finally { await act(async () => root.unmount()); host.remove(); }
});
it.each(["F2", "ContextMenu"])("opens one guarded editor via %s and Escape leaves the model alone", async (key) => {
  const applied = vi.fn(); const host = document.createElement("div"); document.body.append(host);
  const root = createRoot(host);
  try {
    await act(async () => root.render(<RenameWorkbench tree={tree} scope={0} onApplied={applied}>
      <button data-rename-kind="group_address" data-rename-id="2">Original</button>
    </RenameWorkbench>));
    await act(async () => host.querySelector("button")!.dispatchEvent(new KeyboardEvent("keydown", { key, bubbles: true })));
    if (key === "ContextMenu") {
      const menu = host.querySelector('[role="menuitem"]'); expect(menu).not.toBeNull();
      await act(async () => (menu as HTMLElement).click());
    }
    expect(host.querySelector<HTMLInputElement>('input[data-rename-name]')?.value).toBe("Original");
    await act(async () => host.querySelector('input')!.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true })));
    expect(host.querySelector('[role="dialog"]')).toBeNull(); expect(applied).not.toHaveBeenCalled();
  } finally { await act(async () => root.unmount()); host.remove(); }
});
