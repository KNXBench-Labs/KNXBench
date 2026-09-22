/** Tests for the tree's shared multi-select behaviour and building-part label translation. */
// @vitest-environment happy-dom
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { ProjectTree } from "./bindings/ProjectTree";
import type { InstallationNode } from "./bindings/InstallationNode";
import type { DeviceNode } from "./bindings/DeviceNode";
import type { GroupAddressNode } from "./bindings/GroupAddressNode";
import type { BuildingNode } from "./bindings/BuildingNode";
import type { Selection } from "./selection";
import { UI_LANGUAGE_STORAGE_KEY, resetUiLanguageForTests } from "./uiLanguage";

const apiMock = vi.hoisted(() => ({
  batchDeleteDevices: vi.fn(),
  batchDeleteGroupAddresses: vi.fn(),
  batchMoveDevicesToLine: vi.fn(),
  batchMoveDevicesToBuildingPart: vi.fn(),
  moveDeviceToLine: vi.fn(),
  moveDeviceToBuildingPart: vi.fn(),
}));

vi.mock("./api", () => ({
  ...apiMock,
  errorMessage: (error: unknown) => (error instanceof Error ? error.message : String(error)),
}));

import ProjectExplorer from "./ProjectExplorer";
import BulkActionToolbar from "./BulkActionToolbar";
import { useMultiSelection } from "./multiSelection";
import { resetSettingsForTests, setSetting } from "./settingsStore";

let host: HTMLDivElement | undefined;

afterEach(() => {
  host?.remove();
  host = undefined;
  vi.clearAllMocks();
  resetSettingsForTests();
  resetUiLanguageForTests();
});

function device(id: number, name: string): DeviceNode {
  return { id, name, address: null, description: null, com_object_count: 0 };
}

function ga(id: number, name: string, address: string): GroupAddressNode {
  return { id, name, address, range: null, dpts: [], links: [] };
}

function building(id: number, name: string, kind: string): BuildingNode {
  return { id, name, kind, children: [], devices: [] };
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
    group_address_style: "ThreeLevel",
    installations: [installation],
  };
}

function treeWithSecondInstallation(): ProjectTree {
  const tree = baseTree();
  tree.installations.push({
    id: 2,
    name: "Second installation",
    topology: [{
      id: 2,
      name: "Area 2",
      address: 2,
      lines: [{ id: 22, name: "Line 2", address: 2, devices: [device(9, "Second device")] }],
    }],
    buildings: [],
    unassigned: [],
    group_addresses: [],
    group_ranges: [],
  });
  return tree;
}

class TestDataTransfer {
  private readonly values = new Map<string, string>();
  dropEffect: DataTransfer["dropEffect"] = "none";
  effectAllowed: DataTransfer["effectAllowed"] = "uninitialized";

  get types(): string[] {
    return [...this.values.keys()];
  }

  setData(format: string, data: string): void {
    this.values.set(format, data);
  }

  getData(format: string): string {
    return this.values.get(format) ?? "";
  }
}

// The multi-selection state machine lives in `multiSelection.ts` and the
// single `BulkActionToolbar` is rendered by `App` (stage 4: one owner, so
// the tree and the group-address table cannot disagree about what is
// selected). This harness is the smallest stand-in for that wiring, so
// these tests keep asserting the tree's click behaviour and the toolbar it
// drives rather than where either now happens to be declared.
function ExplorerHarness(props: {
  tree: ProjectTree;
  onSelect: (sel: Selection) => void;
  onTreeUpdate: (tree: ProjectTree) => void;
  onSummary: (message: string) => void;
  onError: (error: unknown) => void;
}) {
  const { multiSelection, onItemClick, clear } = useMultiSelection(props.tree, props.onSelect);
  return (
    <>
      {multiSelection && multiSelection.ids.size > 0 && (
        <BulkActionToolbar
          multiSelection={multiSelection}
          tree={props.tree}
          onTreeUpdate={props.onTreeUpdate}
          onDone={clear}
        />
      )}
      <ProjectExplorer
        tree={props.tree}
        selection={null}
        onSelect={props.onSelect}
        onTreeUpdate={props.onTreeUpdate}
        multiSelection={multiSelection}
        onItemClick={onItemClick}
        onSummary={props.onSummary}
        onError={props.onError}
      />
    </>
  );
}

async function renderExplorer(
  tree: ProjectTree,
  onSelect = vi.fn(),
  onTreeUpdate = vi.fn(),
  onSummary = vi.fn(),
  onError = vi.fn(),
) {
  host = document.createElement("div");
  document.body.appendChild(host);
  const root = createRoot(host);
  await act(async () => {
    root.render(
      <ExplorerHarness
        tree={tree}
        onSelect={onSelect}
        onTreeUpdate={onTreeUpdate}
        onSummary={onSummary}
        onError={onError}
      />,
    );
  });
  return { root, onSelect, onTreeUpdate, onSummary, onError };
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

async function dispatchDrag(
  target: HTMLElement,
  type: "dragstart" | "dragover" | "drop" | "dragend",
  transfer: TestDataTransfer,
): Promise<Event> {
  const event = new Event(type, { bubbles: true, cancelable: true });
  Object.defineProperty(event, "dataTransfer", { value: transfer });
  await act(async () => target.dispatchEvent(event));
  return event;
}

async function dragAndDrop(source: HTMLElement, target: HTMLElement): Promise<TestDataTransfer> {
  const transfer = new TestDataTransfer();
  await dispatchDrag(source, "dragstart", transfer);
  await dispatchDrag(target, "dragover", transfer);
  await dispatchDrag(target, "drop", transfer);
  return transfer;
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

describe("ProjectExplorer — building-part kind: translated label vs. untouched discriminant", () => {
  it("renders a translated kind word in the building's label while the same node's raw kind stays the wire value", async () => {
    setSetting(UI_LANGUAGE_STORAGE_KEY, "de");
    const tree = baseTree();
    tree.installations[0].buildings = [building(501, "Erdgeschoss", "Room")];
    const { root } = await renderExplorer(tree);

    // German UI language: the rendered label carries the translated word
    // ("Raum"), never the raw "Room" discriminant.
    expect(host!.textContent).toContain("Erdgeschoss (Raum)");
    expect(host!.textContent).not.toContain("(Room)");

    // The create-row's kind <option>s prove the split directly: the
    // displayed text is translated, but the `value` attribute — what
    // actually reaches `api.createBuildingPart` — is still the untouched
    // domain discriminant.
    const roomOption = Array.from(
      host!.querySelectorAll<HTMLOptionElement>("select option"),
    ).find((o) => o.textContent === "Raum")!;
    expect(roomOption).toBeTruthy();
    expect(roomOption.value).toBe("Room");

    await unmount(root);
  });
});

describe("ProjectExplorer — Project node", () => {
  // The Project node is the only way a user reaches `ProjectInspector`
  // (T4, KNOWN_LIMITATIONS.md §84) — `Inspector.test.tsx` builds the
  // `{ kind: "project" }` selection directly, so it never exercises
  // whether a click on the tree actually produces that selection. Delete
  // the node and this is the test that turns red; without it, review's
  // own mutation (deleting the node) left the suite green.
  it('renders and, on click, selects kind "project"', async () => {
    const onSelect = vi.fn();
    const { root } = await renderExplorer(baseTree(), onSelect);

    const projectLabel = labelFor("Project");
    expect(projectLabel).toBeTruthy();

    await click(projectLabel);
    expect(onSelect).toHaveBeenCalledWith({ kind: "project", id: 0 } satisfies Selection);

    await unmount(root);
  });
});

describe("ProjectExplorer structural drag source", () => {
  it("exposes only first-installation topology devices as drag sources", async () => {
    const { root } = await renderExplorer(treeWithSecondInstallation());

    expect(labelFor("Device A").getAttribute("draggable")).toBe("true");
    expect(labelFor("Device D").getAttribute("draggable")).toBe("true");
    expect(labelFor("Second device").getAttribute("draggable")).not.toBe("true");

    await unmount(root);
  });

  it("drag source writes only the typed decimal device id", async () => {
    const { root } = await renderExplorer(baseTree());
    const transfer = new TestDataTransfer();

    await dispatchDrag(labelFor("Device A"), "dragstart", transfer);

    expect(transfer.types).toEqual(["application/x-knxbench-device-id"]);
    expect(transfer.getData("application/x-knxbench-device-id")).toBe("1");
    expect(transfer.effectAllowed).toBe("move");
    await unmount(root);
  });

  it.each(["1x", "0", "-1", "9007199254740992"])(
    "foreign drag payload %s never calls the line move API",
    async (payload) => {
      const { root } = await renderExplorer(baseTree());
      const transfer = new TestDataTransfer();
      transfer.setData("application/x-knxbench-device-id", payload);

      await dispatchDrag(labelFor("Line 1: Line 1"), "drop", transfer);

      expect(apiMock.moveDeviceToLine).not.toHaveBeenCalled();
      expect(labelFor("Line 1: Line 1").getAttribute("data-drop-ready")).toBeNull();
      await unmount(root);
    },
  );

  it("foreign drag MIME never advertises line acceptance", async () => {
    const { root } = await renderExplorer(baseTree());
    const transfer = new TestDataTransfer();
    transfer.setData("text/plain", "1");

    const event = await dispatchDrag(labelFor("Line 1: Line 1"), "dragover", transfer);

    expect(event.defaultPrevented).toBe(false);
    expect(apiMock.moveDeviceToLine).not.toHaveBeenCalled();
    await unmount(root);
  });
});

describe("ProjectExplorer line drop", () => {
  it("moves a current first-installation device through the line command", async () => {
    const nextTree = baseTree();
    nextTree.installations[0].topology[0].lines[0].name = "Updated line";
    apiMock.moveDeviceToLine.mockResolvedValueOnce(nextTree);
    const { root, onTreeUpdate, onSummary } = await renderExplorer(baseTree());

    await dragAndDrop(labelFor("Device A"), labelFor("Line 1: Line 1"));

    expect(apiMock.moveDeviceToLine).toHaveBeenCalledWith(1, 1);
    expect(onTreeUpdate).toHaveBeenCalledWith(nextTree);
    expect(onSummary).toHaveBeenCalledWith("Device A moved to line Line 1: Line 1.");
    await unmount(root);
  });

  it("reports a rejected line drop without mutating the projection", async () => {
    const error = new Error("move refused");
    apiMock.moveDeviceToLine.mockRejectedValueOnce(error);
    const { root, onTreeUpdate, onSummary, onError } = await renderExplorer(baseTree());

    await dragAndDrop(labelFor("Device A"), labelFor("Line 1: Line 1"));

    expect(onTreeUpdate).not.toHaveBeenCalled();
    expect(onSummary).not.toHaveBeenCalled();
    expect(onError).toHaveBeenCalledWith(error);
    await unmount(root);
  });

  it("never accepts a line in a second installation", async () => {
    const { root } = await renderExplorer(treeWithSecondInstallation());
    const transfer = new TestDataTransfer();
    await dispatchDrag(labelFor("Device A"), "dragstart", transfer);

    const target = labelFor("Line 2: Line 2");
    const event = await dispatchDrag(target, "dragover", transfer);
    await dispatchDrag(target, "drop", transfer);

    expect(event.defaultPrevented).toBe(false);
    expect(target.getAttribute("data-drop-ready")).toBeNull();
    expect(apiMock.moveDeviceToLine).not.toHaveBeenCalled();
    await unmount(root);
  });

  it("ignores a stale drag source removed before the line drop", async () => {
    const tree = baseTree();
    const { root, onSelect, onTreeUpdate, onSummary, onError } = await renderExplorer(tree);
    const transfer = new TestDataTransfer();
    await dispatchDrag(labelFor("Device A"), "dragstart", transfer);

    const withoutSource = baseTree();
    withoutSource.installations[0].topology[0].lines[0].devices =
      withoutSource.installations[0].topology[0].lines[0].devices.filter(({ id }) => id !== 1);
    await act(async () => {
      root.render(
        <ExplorerHarness
          tree={withoutSource}
          onSelect={onSelect}
          onTreeUpdate={onTreeUpdate}
          onSummary={onSummary}
          onError={onError}
        />,
      );
    });

    await dispatchDrag(labelFor("Line 1: Line 1"), "drop", transfer);

    expect(apiMock.moveDeviceToLine).not.toHaveBeenCalled();
    expect(onTreeUpdate).not.toHaveBeenCalled();
    expect(onSummary).not.toHaveBeenCalled();
    expect(onError).not.toHaveBeenCalled();
    await unmount(root);
  });
});
