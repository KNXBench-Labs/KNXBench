// @vitest-environment happy-dom
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { ProjectTree } from "./bindings/ProjectTree";
import type { InstallationNode } from "./bindings/InstallationNode";
import type { DeviceNode } from "./bindings/DeviceNode";
import type { GroupAddressNode } from "./bindings/GroupAddressNode";
import type { Selection } from "./selection";

const apiMock = vi.hoisted(() => ({
  batchDeleteDevices: vi.fn(),
  batchDeleteGroupAddresses: vi.fn(),
  batchMoveDevicesToLine: vi.fn(),
  batchMoveDevicesToBuildingPart: vi.fn(),
}));

vi.mock("./api", () => ({
  ...apiMock,
  errorMessage: (error: unknown) => (error instanceof Error ? error.message : String(error)),
}));

import ProjectExplorer from "./ProjectExplorer";

let host: HTMLDivElement | undefined;

afterEach(() => {
  host?.remove();
  host = undefined;
  vi.clearAllMocks();
});

function device(id: number, name: string): DeviceNode {
  return { id, name, address: null, description: null, com_object_count: 0 };
}

function ga(id: number, name: string, address: string): GroupAddressNode {
  return { id, name, address };
}

function baseTree(): ProjectTree {
  const devices = [device(1, "Device A"), device(2, "Device B"), device(3, "Device C")];
  const unassigned = [device(4, "Device D")];
  const groupAddresses = [
    ga(101, "GA A", "1/1/1"),
    ga(102, "GA B", "1/1/2"),
    ga(103, "GA C", "1/1/3"),
  ];
  const installation: InstallationNode = {
    id: 0,
    name: "Installation",
    topology: [
      {
        id: 1,
        name: "Area 1",
        address: 1,
        lines: [{ id: 1, name: "Line 1", address: 1, devices }],
      },
    ],
    buildings: [],
    unassigned,
    group_addresses: groupAddresses,
    group_ranges: [],
  };
  return {
    schema_version: 11,
    errors: 0,
    warnings: 0,
    can_undo: false,
    can_redo: false,
    installations: [installation],
  };
}

async function renderExplorer(tree: ProjectTree, onSelect = vi.fn(), onTreeUpdate = vi.fn()) {
  host = document.createElement("div");
  document.body.appendChild(host);
  const root = createRoot(host);
  await act(async () => {
    root.render(
      <ProjectExplorer
        tree={tree}
        selection={null}
        onSelect={onSelect}
        onTreeUpdate={onTreeUpdate}
      />,
    );
  });
  return { root, onSelect, onTreeUpdate };
}

function labelFor(text: string): HTMLElement {
  const found = Array.from(host!.querySelectorAll<HTMLElement>(".tree-label")).find(
    (el) => el.textContent === text,
  );
  if (!found) throw new Error(`no .tree-label with text "${text}" found`);
  return found;
}

async function click(el: HTMLElement, opts: { ctrlKey?: boolean; shiftKey?: boolean } = {}) {
  await act(async () => {
    el.dispatchEvent(
      new MouseEvent("click", { bubbles: true, cancelable: true, ...opts }),
    );
  });
}

async function unmount(root: Root) {
  await act(async () => root.unmount());
}

describe("ProjectExplorer multi-select", () => {
  it("ctrl-click toggles a device id into a MultiSelection", async () => {
    const { root } = await renderExplorer(baseTree());

    await click(labelFor("Device A"), { ctrlKey: true });
    expect(host!.textContent).toContain("1 device selected");

    await click(labelFor("Device B"), { ctrlKey: true });
    expect(host!.textContent).toContain("2 devices selected");

    // ctrl-clicking an already-selected id toggles it back out.
    await click(labelFor("Device A"), { ctrlKey: true });
    expect(host!.textContent).toContain("1 device selected");

    await unmount(root);
  });

  it("ctrl-click on a different kind while one kind is active replaces it", async () => {
    const { root } = await renderExplorer(baseTree());

    await click(labelFor("Device A"), { ctrlKey: true });
    await click(labelFor("Device B"), { ctrlKey: true });
    expect(host!.textContent).toContain("2 devices selected");

    await click(labelFor("1/1/1 GA A"), { ctrlKey: true });
    expect(host!.textContent).toContain("1 group address selected");
    expect(host!.textContent).not.toContain("device selected");
    expect(host!.textContent).not.toContain("devices selected");

    await unmount(root);
  });

  it("shift-click extends a contiguous range within the same kind", async () => {
    const { root } = await renderExplorer(baseTree());

    // Plain click on Device A sets the anchor.
    await click(labelFor("Device A"));
    await click(labelFor("Device C"), { shiftKey: true });

    expect(host!.textContent).toContain("3 devices selected");

    await unmount(root);
  });

  it("plain click clears the MultiSelection and sets the single Selection", async () => {
    const onSelect = vi.fn();
    const { root } = await renderExplorer(baseTree(), onSelect);

    await click(labelFor("Device A"), { ctrlKey: true });
    await click(labelFor("Device B"), { ctrlKey: true });
    expect(host!.textContent).toContain("2 devices selected");

    await click(labelFor("Device C"));
    expect(host!.textContent).not.toContain("devices selected");
    expect(host!.textContent).not.toContain("device selected");
    expect(onSelect).toHaveBeenCalledWith({ kind: "device", id: 3 } satisfies Selection);

    await unmount(root);
  });

  it("Escape clears the MultiSelection", async () => {
    const { root } = await renderExplorer(baseTree());

    await click(labelFor("Device A"), { ctrlKey: true });
    expect(host!.textContent).toContain("1 device selected");

    await act(async () => {
      window.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true }));
    });
    expect(host!.textContent).not.toContain("device selected");

    await unmount(root);
  });

  it("bulk-deletes selected devices and clears the selection on success", async () => {
    const newTree = baseTree();
    apiMock.batchDeleteDevices.mockResolvedValueOnce(newTree);
    const { root, onTreeUpdate } = await renderExplorer(baseTree());

    await click(labelFor("Device A"), { ctrlKey: true });
    await click(labelFor("Device B"), { ctrlKey: true });

    const deleteButton = Array.from(
      host!.querySelectorAll<HTMLButtonElement>(".bulk-action-toolbar button"),
    ).find((b) => b.textContent === "Delete")!;
    await click(deleteButton);

    expect(apiMock.batchDeleteDevices).toHaveBeenCalledWith([1, 2]);
    expect(onTreeUpdate).toHaveBeenCalledWith(newTree);
    expect(host!.querySelector(".bulk-action-toolbar")).toBeNull();

    await unmount(root);
  });

  it("bulk-deletes selected group addresses via the group-address batch route", async () => {
    const newTree = baseTree();
    apiMock.batchDeleteGroupAddresses.mockResolvedValueOnce(newTree);
    const { root } = await renderExplorer(baseTree());

    await click(labelFor("1/1/1 GA A"), { ctrlKey: true });
    await click(labelFor("1/1/2 GA B"), { ctrlKey: true });

    const deleteButton = Array.from(
      host!.querySelectorAll<HTMLButtonElement>(".bulk-action-toolbar button"),
    ).find((b) => b.textContent === "Delete")!;
    await click(deleteButton);

    expect(apiMock.batchDeleteGroupAddresses).toHaveBeenCalledWith([101, 102]);
    expect(host!.querySelector(".bulk-action-toolbar")).toBeNull();

    await unmount(root);
  });

  it("moves selected devices to a line via the picker", async () => {
    const newTree = baseTree();
    apiMock.batchMoveDevicesToLine.mockResolvedValueOnce(newTree);
    const { root } = await renderExplorer(baseTree());

    await click(labelFor("Device A"), { ctrlKey: true });
    await click(labelFor("Device D"), { ctrlKey: true });

    const select = host!.querySelector<HTMLSelectElement>(
      ".bulk-action-toolbar select[data-role='move-line']",
    )!;
    await act(async () => {
      select.value = "1";
      select.dispatchEvent(new Event("change", { bubbles: true }));
    });

    expect(apiMock.batchMoveDevicesToLine).toHaveBeenCalledWith([1, 4], 1);

    await unmount(root);
  });

  it("moves selected devices to unassigned building part via the picker", async () => {
    const newTree = baseTree();
    apiMock.batchMoveDevicesToBuildingPart.mockResolvedValueOnce(newTree);
    const { root } = await renderExplorer(baseTree());

    await click(labelFor("Device A"), { ctrlKey: true });

    const select = host!.querySelector<HTMLSelectElement>(
      ".bulk-action-toolbar select[data-role='move-building-part']",
    )!;
    await act(async () => {
      select.value = "none";
      select.dispatchEvent(new Event("change", { bubbles: true }));
    });

    expect(apiMock.batchMoveDevicesToBuildingPart).toHaveBeenCalledWith([1], null);

    await unmount(root);
  });

  it("dismiss control clears the MultiSelection without calling any batch API", async () => {
    const { root } = await renderExplorer(baseTree());

    await click(labelFor("Device A"), { ctrlKey: true });
    expect(host!.textContent).toContain("1 device selected");

    const dismiss = host!.querySelector<HTMLButtonElement>(
      ".bulk-action-toolbar button[aria-label='Dismiss selection']",
    )!;
    await click(dismiss);

    expect(host!.querySelector(".bulk-action-toolbar")).toBeNull();
    expect(apiMock.batchDeleteDevices).not.toHaveBeenCalled();

    await unmount(root);
  });
});
