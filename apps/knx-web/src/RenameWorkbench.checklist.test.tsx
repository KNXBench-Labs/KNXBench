/** Runs the shared menu checklist (WP-49.3) against the RenameWorkbench context menu. */
// @vitest-environment happy-dom
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { vi } from "vitest";
import RenameWorkbench from "./RenameWorkbench";
import type { ProjectTree } from "./bindings/ProjectTree";
import { describeMenuChecklist, type MenuAdapter } from "./test/menuChecklist";

vi.mock("./api", () => ({ renameEntity: vi.fn(), errorMessage: (e: unknown) => String(e) }));
(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const tree: ProjectTree = { schema_version: 11, errors: 0, warnings: 0,
  can_undo: false, can_redo: false, is_modified: false, group_address_style: "ThreeLevel",
  server_incarnation: "rename", snapshot_revision: 1, project_incarnation: 0,
  installations: [{ id: 1, name: "I", topology: [], buildings: [], unassigned: [], group_ranges: [],
    group_addresses: [{ id: 2, name: "Original", address: "1/1/1", range: null, dpts: [], links: [] }] }] };

let host: HTMLDivElement | undefined;
let root: Root | undefined;
const target = () => host!.querySelector<HTMLElement>("[data-rename-kind]")!;
const menu = () => host!.querySelector<HTMLElement>('[role="menu"]');
const pointerDown = (el: Element) => act(async () => { el.dispatchEvent(new PointerEvent("pointerdown", { bubbles: true })); });

const adapter: MenuAdapter = {
  name: "RenameWorkbench context menu",
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
    host!.remove();
    host = undefined;
    root = undefined;
  },
  trigger: target,
  async open() {
    target().focus();
    await act(async () => { target().dispatchEvent(new KeyboardEvent("keydown", { key: "ContextMenu", bubbles: true, cancelable: true })); });
  },
  menu,
  items: () => [...menu()!.querySelectorAll<HTMLElement>("[role=menuitem]")],
  activations: () => host!.querySelectorAll('[role="dialog"]').length,
  pressInside: () => pointerDown(menu()!.querySelector("button")!),
  pressOutside: () => pointerDown(document.body),
  popupTrigger: "none",
  menuName: "named",
  escape: "stops",
  tab: "swallows",
};

describeMenuChecklist(adapter);
