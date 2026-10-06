/** Tests for Inspector's collapsed delete-restriction message on non-empty group ranges. */
// @vitest-environment happy-dom
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { ProjectTree } from "./bindings/ProjectTree";
import type { InstallationNode } from "./bindings/InstallationNode";
import type { GroupAddressNode } from "./bindings/GroupAddressNode";
import type { GroupRangeNode } from "./bindings/GroupRangeNode";
import type { DeviceDetail } from "./bindings/DeviceDetail";
import type { Selection } from "./selection";

const apiMock = vi.hoisted(() => ({
  deviceParameters: vi.fn().mockResolvedValue({
    programId: null,
    sections: [],
    stale: [],
    diagnostics: [],
  }),
  moveDeviceToLine: vi.fn(),
  moveLineToArea: vi.fn(),
  moveDeviceToBuildingPart: vi.fn(),
  moveBuildingPart: vi.fn(),
  moveGroupRange: vi.fn(),
  setComObjectFlag: vi.fn(),
  linkComObject: vi.fn(),
  unlinkComObject: vi.fn(),
  setIndividualAddress: vi.fn(),
  renameArea: vi.fn(),
  renameLine: vi.fn(),
  setGroupAddressStyle: vi.fn(),
  renameInstallation: vi.fn(),
  deleteGroupAddress: vi.fn(),
  repairDevicePlacement: vi.fn(),
  repairLineOwner: vi.fn(),
}));

vi.mock("./api", () => ({
  ...apiMock,
  errorMessage: (error: unknown) => (error instanceof Error ? error.message : String(error)),
}));

import Inspector from "./Inspector";
import { UI_LANGUAGE_STORAGE_KEY, resetUiLanguageForTests } from "./uiLanguage";
import { resetSettingsForTests, setSetting } from "./settingsStore";
import { PRODUCT_LANGUAGE_STORAGE_KEY, resetProductLanguageForTests } from "./productLanguage";

let host: HTMLDivElement | undefined;
let root: Root | undefined;

afterEach(async () => {
  if (root) {
    await act(async () => root!.unmount());
    root = undefined;
  }
  host?.remove();
  host = undefined;
  vi.clearAllMocks();
  resetSettingsForTests();
  resetUiLanguageForTests();
});

it.each([
  ["Stairway", "Treppenhaus"], ["RoomPart", "Raumteil"], ["Area", "Bereich"],
  ["Ground", "Grundstück"], ["Segment", "Segment"],
])("localizes the %s building kind in the Inspector", async (kind, label) => {
  setSetting(UI_LANGUAGE_STORAGE_KEY, "de");
  const tree = twoInstallationTree();
  tree.installations[0].buildings = [{ id: 501, name: "Test", kind, children: [], devices: [] }];
  await renderInspector({ kind: "building_part", id: 501 }, tree);
  expect(host!.querySelector(".inspector-description")?.textContent).toBe(label);
});

function ga(id: number, name: string, address: string): GroupAddressNode {
  return { id, name, address, range: null, dpts: [], links: [] };
}

function groupRange(id: number, name: string, start: string, end: string): GroupRangeNode {
  return { id, name, start, end, parent: null };
}

// Two installations: the second one carries the group address / group
// range under test, so `canDelete`/`canEdit` (Inspector's own
// `installations[0]`-only gate) come back false — the same shape
// `restrictedToFirstInstallationMessage`'s six original call sites all
// existed to report.
function twoInstallationTree(): ProjectTree {
  const first: InstallationNode = {
    id: 0,
    name: "Installation 1",
    topology: [],
    buildings: [],
    unassigned: [],
    group_addresses: [],
    group_ranges: [],
  };
  const second: InstallationNode = {
    id: 1,
    name: "Installation 2",
    topology: [],
    buildings: [],
    unassigned: [],
    group_addresses: [ga(201, "GA in second", "2/1/1")],
    group_ranges: [groupRange(301, "Range in second", "2/0/0", "2/7/255")],
  };
  return {
    schema_version: 11,
    errors: 0,
    warnings: 0,
    can_undo: false,
    can_redo: false,
    is_modified: false,
    group_address_style: "ThreeLevel",
    installations: [first, second],
  };
}

async function renderInspector(
  selection: Selection,
  tree: ProjectTree,
  deviceDetail: DeviceDetail | null = null,
  onApplied = vi.fn(),
) {
  host = document.createElement("div");
  document.body.appendChild(host);
  root = createRoot(host);
  await act(async () => {
    root!.render(
      <Inspector
        selection={selection}
        tree={tree}
        deviceDetail={deviceDetail}
        onApplied={onApplied}
        onDeleted={vi.fn()}
      />,
    );
  });
  return onApplied;
}

function setTextInputValue(input: HTMLInputElement, value: string) {
  const setter = Object.getOwnPropertyDescriptor(window.HTMLInputElement.prototype, "value")!.set!;
  setter.call(input, value);
  input.dispatchEvent(new Event("input", { bubbles: true }));
}

function movableBuildingTree(): ProjectTree {
  const tree = twoInstallationTree();
  tree.installations[0].buildings = [
    {
      id: 500, name: "Main", kind: "Building", devices: [],
      children: [{
        id: 501, name: "Floor", kind: "Floor", devices: [],
        children: [{ id: 502, name: "Room", kind: "Room", devices: [], children: [] }],
      }],
    },
    { id: 503, name: "Annex", kind: "Building", devices: [], children: [] },
  ];
  return tree;
}

it("moves a building part from Properties without offering itself or descendants as targets", async () => {
  const tree = movableBuildingTree();
  const onApplied = vi.fn();
  apiMock.moveBuildingPart.mockResolvedValue(tree);
  await renderInspector({ kind: "building_part", id: 501 }, tree, null, onApplied);
  const select = host!.querySelector<HTMLSelectElement>(".inspector-field select");
  expect(select?.value).toBe("500");
  expect(Array.from(select!.options, (option) => option.value)).toEqual(["", "500", "503"]);
  await act(async () => {
    select!.value = "503";
    select!.dispatchEvent(new Event("change", { bubbles: true }));
  });
  expect(apiMock.moveBuildingPart).toHaveBeenCalledExactlyOnceWith(501, 503);
  expect(onApplied).toHaveBeenCalledExactlyOnceWith(tree);
});

it("moves a building part to the building root with an explicit null parent", async () => {
  const tree = movableBuildingTree();
  apiMock.moveBuildingPart.mockResolvedValue(tree);
  await renderInspector({ kind: "building_part", id: 501 }, tree);
  const select = host!.querySelector<HTMLSelectElement>(".inspector-field select")!;
  await act(async () => {
    select.value = "";
    select.dispatchEvent(new Event("change", { bubbles: true }));
  });
  expect(apiMock.moveBuildingPart).toHaveBeenCalledExactlyOnceWith(501, null);
});

it("keeps the old building parent and shows a failed move instead of hiding it", async () => {
  const tree = movableBuildingTree();
  apiMock.moveBuildingPart.mockRejectedValue(new Error("parent is not available"));
  await renderInspector({ kind: "building_part", id: 501 }, tree);
  const select = host!.querySelector<HTMLSelectElement>(".inspector-field select")!;
  await act(async () => {
    select.value = "503";
    select.dispatchEvent(new Event("change", { bubbles: true }));
  });
  expect(apiMock.moveBuildingPart).toHaveBeenCalledExactlyOnceWith(501, 503);
  expect(select.value).toBe("500");
  expect(host!.querySelector(".field-error")?.textContent).toContain("parent is not available");
});

// MODEL-01 / ADR-0070: a uniquely owned later-installation entity is edited
// in its own installation (these four tests used to assert the old
// first-installation-only rule).
it("offers a building move for a later installation within that installation", async () => {
  const tree = movableBuildingTree();
  tree.installations[1].buildings = [
    { id: 601, name: "Later building", kind: "Building", devices: [], children: [] },
    { id: 602, name: "Later annex", kind: "Building", devices: [], children: [] },
  ];
  await renderInspector({ kind: "building_part", id: 601 }, tree);
  const values = Array.from(host!.querySelectorAll<HTMLOptionElement>(".inspector-field select option"), (o) => o.value);
  expect(values).toContain("602");
  expect(values).not.toContain("500");
  expect(host!.textContent).not.toContain("only available");
});

it("does not guess a parent when imported building placements are ambiguous", async () => {
  const tree = movableBuildingTree();
  tree.installations[0].buildings[1].children.push({
    id: 501, name: "Duplicate", kind: "Floor", devices: [], children: [],
  });
  await renderInspector({ kind: "building_part", id: 501 }, tree);
  expect(host!.querySelector(".inspector-field select")).toBeNull();
  expect(host!.textContent).toContain("This structure ID occurs more than once");
  expect(host!.querySelector("label.inspector-field input")).toBeNull();
  expect(apiMock.moveBuildingPart).not.toHaveBeenCalled();
});

function movableRangeTree(): ProjectTree {
  const tree = twoInstallationTree();
  tree.installations[0].group_ranges = [
    groupRange(400, "Main", "0/0/0", "0/7/255"),
    { ...groupRange(401, "Misplaced", "1/1/0", "1/1/255"), parent: 400 },
    { ...groupRange(403, "Nested", "1/1/0", "1/1/127"), parent: 401 },
    groupRange(402, "Target", "1/0/0", "1/7/255"),
  ];
  return tree;
}

it("moves a group range from Properties without offering itself or descendants", async () => {
  const tree = movableRangeTree();
  const onApplied = vi.fn();
  apiMock.moveGroupRange.mockResolvedValue(tree);
  await renderInspector({ kind: "group_range", id: 401 }, tree, null, onApplied);
  const select = host!.querySelector<HTMLSelectElement>(".inspector-field select");
  expect(select?.value).toBe("400");
  expect(Array.from(select!.options, (option) => option.value)).toEqual(["", "400", "402"]);
  await act(async () => {
    select!.value = "402";
    select!.dispatchEvent(new Event("change", { bubbles: true }));
  });
  expect(apiMock.moveGroupRange).toHaveBeenCalledExactlyOnceWith(401, 402);
  expect(onApplied).toHaveBeenCalledExactlyOnceWith(tree);
});

it("sends an explicit null when moving a group range to the root", async () => {
  const tree = movableRangeTree();
  apiMock.moveGroupRange.mockResolvedValue(tree);
  await renderInspector({ kind: "group_range", id: 401 }, tree);
  const select = host!.querySelector<HTMLSelectElement>(".inspector-field select")!;
  await act(async () => {
    select.value = "";
    select.dispatchEvent(new Event("change", { bubbles: true }));
  });
  expect(apiMock.moveGroupRange).toHaveBeenCalledExactlyOnceWith(401, null);
});

it("keeps a group range's old parent and shows a rejected move", async () => {
  const tree = movableRangeTree();
  apiMock.moveGroupRange.mockRejectedValue(new Error("range outside parent"));
  await renderInspector({ kind: "group_range", id: 401 }, tree);
  const select = host!.querySelector<HTMLSelectElement>(".inspector-field select")!;
  await act(async () => {
    select.value = "402";
    select.dispatchEvent(new Event("change", { bubbles: true }));
  });
  expect(apiMock.moveGroupRange).toHaveBeenCalledExactlyOnceWith(401, 402);
  expect(select.value).toBe("400");
  expect(host!.querySelector(".field-error")?.textContent).toContain("range outside parent");
});

it("offers group range moves for a later installation", async () => {
  await renderInspector({ kind: "group_range", id: 301 }, twoInstallationTree());
  expect(host!.querySelector(".inspector-field select")).not.toBeNull();
  expect(host!.textContent).not.toContain("only available");
});

it("does not guess a group range parent when ids or parent references are inconsistent", async () => {
  const tree = movableRangeTree();
  tree.installations[0].group_ranges.push({ ...groupRange(401, "Duplicate", "0/0/0", "0/0/1") });
  await renderInspector({ kind: "group_range", id: 401 }, tree);
  expect(host!.querySelector(".inspector-field select")).toBeNull();
  expect(host!.textContent).toContain("This structure ID occurs more than once");
  expect(host!.querySelector("label.inspector-field input")).toBeNull();
});

it.each([
  ["area", 10, "Area A"],
  ["line", 11, "Line A"],
] as const)("renames a first-installation %s from Properties through one API request", async (kind, id, original) => {
  const tree = deviceMoveTree();
  const onApplied = vi.fn();
  const rename = kind === "area" ? apiMock.renameArea : apiMock.renameLine;
  rename.mockResolvedValue(tree);
  await renderInspector({ kind, id }, tree, null, onApplied);
  const input = host!.querySelector<HTMLInputElement>("label.inspector-field input");
  expect(input?.value).toBe(original);
  await act(async () => {
    setTextInputValue(input!, "Renamed");
    input!.dispatchEvent(new Event("focusout", { bubbles: true }));
  });
  expect(rename).toHaveBeenCalledExactlyOnceWith(id, "Renamed");
  expect(onApplied).toHaveBeenCalledExactlyOnceWith(tree);
});

it.each(["area", "line"] as const)("renames a later-installation %s in its owning installation", async (kind) => {
  const tree = twoInstallationTree();
  tree.installations[1].topology = [{ id: 71, name: "Later area", address: 2,
    lines: [{ id: 72, name: "Later line", address: 1, devices: [] }] }];
  // Queue only the answer this case consumes: a leftover `…Once` would leak
  // into the next test.
  (kind === "area" ? apiMock.renameArea : apiMock.renameLine).mockResolvedValueOnce(tree);
  await renderInspector({ kind, id: kind === "area" ? 71 : 72 }, tree);
  const input = host!.querySelector<HTMLInputElement>("label.inspector-field input")!;
  await act(async () => {
    setTextInputValue(input, "Renamed");
    input.dispatchEvent(new Event("focusout", { bubbles: true }));
  });
  if (kind === "area") expect(apiMock.renameArea).toHaveBeenCalledExactlyOnceWith(71, "Renamed");
  else expect(apiMock.renameLine).toHaveBeenCalledExactlyOnceWith(72, "Renamed");
});

it.each([
  ["area", 10], ["line", 11], ["building_part", 501], ["group_range", 401],
] as const)("refuses ambiguous %s IDs shared with a later installation", async (kind, id) => {
  const tree = deviceMoveTree();
  tree.installations[0].group_ranges = [groupRange(401, "First range", "0/0/0", "0/0/255")];
  const later = twoInstallationTree().installations[1];
  later.topology = [{ id: 10, name: "Later area", address: 2,
    lines: [{ id: 11, name: "Later line", address: 2, devices: [] }] }];
  later.buildings = [{ id: 501, name: "Later building", kind: "Room", children: [], devices: [] }];
  later.group_ranges = [groupRange(401, "Later range", "1/0/0", "1/0/255")];
  tree.installations.push(later);
  await renderInspector({ kind, id }, tree);
  expect(host!.textContent).toContain("This structure ID occurs more than once");
  expect(host!.querySelector("label.inspector-field input")).toBeNull();
  expect(host!.querySelector(".inspector-field select")).toBeNull();
  expect(host!.querySelector("button")).toBeNull();
  expect(apiMock.renameArea).not.toHaveBeenCalled();
  expect(apiMock.renameLine).not.toHaveBeenCalled();
  expect(apiMock.moveBuildingPart).not.toHaveBeenCalled();
  expect(apiMock.moveGroupRange).not.toHaveBeenCalled();
});

it.each([
  ["area", 10], ["line", 11], ["building_part", 501], ["group_range", 401],
] as const)("refuses ambiguous duplicate %s IDs within one installation", async (kind, id) => {
  const tree = deviceMoveTree();
  tree.installations[0].group_ranges = [groupRange(401, "First range", "0/0/0", "0/0/255")];
  if (kind === "area") tree.installations[0].topology.push({ id, name: "Duplicate", address: 2, lines: [] });
  if (kind === "line") tree.installations[0].topology[0].lines.push({ id, name: "Duplicate", address: 3, devices: [] });
  if (kind === "building_part") tree.installations[0].buildings.push({ id, name: "Duplicate", kind: "Room", children: [], devices: [] });
  if (kind === "group_range") tree.installations[0].group_ranges.push(groupRange(id, "Duplicate", "1/0/0", "1/0/255"));
  await renderInspector({ kind, id }, tree);
  expect(host!.textContent).toContain("This structure ID occurs more than once");
  expect(host!.querySelector("label.inspector-field input")).toBeNull();
  expect(host!.querySelector(".inspector-field select")).toBeNull();
  expect(host!.querySelector("button")).toBeNull();
});

it("retains the original area name and shows the server error when rename fails", async () => {
  apiMock.renameArea.mockRejectedValue(new Error("unknown area"));
  const onApplied = await renderInspector({ kind: "area", id: 10 }, deviceMoveTree());
  const input = host!.querySelector<HTMLInputElement>("label.inspector-field input");
  expect(input?.value).toBe("Area A");
  await act(async () => {
    setTextInputValue(input!, "Other");
    input!.dispatchEvent(new Event("focusout", { bubbles: true }));
  });
  expect(input?.value).toBe("Area A");
  expect(host!.querySelector(".field-error")?.textContent).toBe("unknown area");
  expect(onApplied).not.toHaveBeenCalled();
});

function lineMoveTree(): ProjectTree {
  const tree = deviceMoveTree();
  tree.installations[0].topology.push({
    id: 20, name: "Area B", address: 2,
    lines: [{ id: 21, name: "Line C", address: 1, devices: [] }],
  });
  return tree;
}

it("moves a selected line to an explicitly labelled area from Properties", async () => {
  const tree = lineMoveTree();
  const onApplied = vi.fn();
  apiMock.moveLineToArea.mockResolvedValue(tree);
  await renderInspector({ kind: "line", id: 12 }, tree, null, onApplied);
  const select = host!.querySelector<HTMLSelectElement>(".inspector-field select");
  expect(select?.value).toBe("10");
  expect(Array.from(select!.options, (option) => option.value)).toEqual(["10", "20"]);
  expect(select!.closest("label")?.textContent).toContain("Assigned area");
  await act(async () => {
    select!.value = "20";
    select!.dispatchEvent(new Event("change", { bubbles: true }));
  });
  expect(apiMock.moveLineToArea).toHaveBeenCalledExactlyOnceWith(12, 20);
  expect(onApplied).toHaveBeenCalledExactlyOnceWith(tree);
});

it("shows why a line move failed and leaves its original area selected", async () => {
  const tree = lineMoveTree();
  apiMock.moveLineToArea.mockRejectedValue(new Error("address differs from assigned line"));
  await renderInspector({ kind: "line", id: 12 }, tree);
  const select = host!.querySelector<HTMLSelectElement>(".inspector-field select")!;
  await act(async () => {
    select.value = "20";
    select.dispatchEvent(new Event("change", { bubbles: true }));
  });
  expect(apiMock.moveLineToArea).toHaveBeenCalledExactlyOnceWith(12, 20);
  expect(select.value).toBe("10");
  expect(host!.querySelector(".field-error")?.textContent).toContain("address differs from assigned line");
});

it("offers a line move for a later installation among its own areas", async () => {
  const tree = deviceMoveTree();
  const later = twoInstallationTree().installations[1];
  later.topology = [{ id: 71, name: "Later", address: 2, lines: [{ id: 72, name: "Later line", address: 1, devices: [] }] },
    { id: 73, name: "Later two", address: 3, lines: [] }];
  tree.installations.push(later);
  await renderInspector({ kind: "line", id: 72 }, tree);
  const values = Array.from(host!.querySelectorAll<HTMLOptionElement>(".inspector-field select option"), (o) => o.value);
  expect(values).toContain("73");
  expect(values).not.toContain("10");
});

it("does not guess a source area when a line is projected under multiple areas", async () => {
  const tree = lineMoveTree();
  tree.installations[0].topology[1].lines.push({
    id: 12, name: "Duplicate", address: 2, devices: [],
  });
  await renderInspector({ kind: "line", id: 12 }, tree);
  expect(host!.querySelector(".inspector-field select")).toBeNull();
  expect(host!.textContent).toContain("This structure ID occurs more than once");
  expect(host!.querySelector("label.inspector-field input")).toBeNull();
  expect(apiMock.moveLineToArea).not.toHaveBeenCalled();
});

function deviceMoveTree(): ProjectTree {
  const device = {
    id: 42,
    name: "Device D",
    address: null,
    description: null,
    com_object_count: 0,
  };
  return {
    schema_version: 11,
    errors: 0,
    warnings: 0,
    can_undo: false,
    can_redo: false,
    is_modified: false,
    group_address_style: "ThreeLevel",
    installations: [{
      id: 1,
      name: "Installation",
      topology: [{
        id: 10,
        name: "Area A",
        address: 1,
        lines: [
          { id: 11, name: "Line A", address: 1, devices: [device] },
          { id: 12, name: "Line B", address: 2, devices: [] },
        ],
      }],
      buildings: [{ id: 501, name: "Room A", kind: "Room", children: [], devices: [] }],
      unassigned: [],
      group_addresses: [],
      group_ranges: [],
    }],
  };
}

function deviceDetail(): DeviceDetail {
  return {
    id: 42,
    name: "Device D",
    description: null,
    address: null,
    com_objects: [],
    product: { product_ref: null, program_ref: null, catalog: null, resolution: "NoReference" },
  };
}

function detailWithComObject(links: DeviceDetail["com_objects"][number]["links"] = []): DeviceDetail {
  return { ...deviceDetail(), com_objects: [{
    id: 7, number: 1, name: "Switch actuator output", dpt: "DPST-1-1", dpt_layer: null,
    description: null, description_layer: null, is_active: true,
    activation: "NotEvaluated", channel: null, program_dpt: null, dpt_text: null, function_text: null,
    read: false, write: true, transmit: false, update: false,
    communication: true, read_on_init: false, links,
  }] };
}

// MODEL-01 / ADR-0070: an entity is edited in the installation that owns it;
// every move or link target list comes from that installation only. Only an
// entity that does not belong to exactly one installation stays read-only.
function laterInstallationTree(): ProjectTree {
  const tree = twoInstallationTree();
  const device = { id: 42, name: "Device D", address: null, description: null, com_object_count: 0 };
  tree.installations[0].topology = [{ id: 10, name: "Area A", address: 1, lines: [
    { id: 11, name: "Line A", address: 1, devices: [] }, { id: 12, name: "Line B", address: 2, devices: [] }] }];
  tree.installations[0].buildings = [{ id: 500, name: "Main", kind: "Building", children: [], devices: [] }];
  tree.installations[0].group_ranges = [groupRange(300, "First range", "1/0/0", "1/7/255")];
  tree.installations[1].topology = [
    { id: 20, name: "Area B", address: 2, lines: [
      { id: 21, name: "Line C", address: 1, devices: [device] }, { id: 22, name: "Line D", address: 2, devices: [] }] },
    { id: 23, name: "Area C", address: 3, lines: [] },
  ];
  tree.installations[1].buildings = [{ id: 600, name: "Annex", kind: "Building", devices: [], children: [
    { id: 601, name: "Hall", kind: "Room", children: [], devices: [] }] },
    { id: 602, name: "Shed", kind: "Building", children: [], devices: [] }];
  tree.installations[1].group_ranges.push(groupRange(302, "Second main", "3/0/0", "3/7/255"));
  return tree;
}

function optionValues(): string[] {
  return Array.from(host!.querySelectorAll<HTMLOptionElement>("select option"), (option) => option.value);
}

describe("Inspector — entities of a later installation", () => {
  it("moves a later-installation device only among that installation's lines and parts, and deletes it", async () => {
    apiMock.moveDeviceToLine.mockResolvedValueOnce(laterInstallationTree());
    await renderInspector({ kind: "device", id: 42 }, laterInstallationTree(), deviceDetail());
    const values = optionValues();
    expect(values).toEqual(expect.arrayContaining(["21", "22", "600", "601", "602"]));
    for (const foreign of ["11", "12", "500"]) expect(values).not.toContain(foreign);
    const lineSelect = Array.from(host!.querySelectorAll<HTMLLabelElement>("label.inspector-field"))
      .find((field) => field.textContent?.startsWith("Line"))!.querySelector("select")!;
    expect(lineSelect.value).toBe("21");
    await act(async () => {
      lineSelect.value = "22";
      lineSelect.dispatchEvent(new Event("change", { bubbles: true }));
    });
    expect(apiMock.moveDeviceToLine).toHaveBeenCalledExactlyOnceWith(42, 22);
    expect(Array.from(host!.querySelectorAll("button")).some((b) => b.textContent === "Delete")).toBe(true);
    expect(host!.textContent).not.toContain("only available");
  });

  it("offers a later-installation line only that installation's areas", async () => {
    await renderInspector({ kind: "line", id: 21 }, laterInstallationTree());
    expect(host!.querySelector("label.inspector-field input")).not.toBeNull();
    const values = optionValues();
    expect(values).toContain("23");
    expect(values).not.toContain("10");
    expect(host!.textContent).not.toContain("only available");
  });

  it("offers a later-installation building part only that installation's parents", async () => {
    await renderInspector({ kind: "building_part", id: 601 }, laterInstallationTree());
    const values = optionValues();
    expect(values).toContain("602");
    expect(values).not.toContain("500");
    expect(host!.textContent).not.toContain("only available");
  });

  it("renames and moves a later-installation group range among that installation's ranges", async () => {
    await renderInspector({ kind: "group_range", id: 301 }, laterInstallationTree());
    expect(host!.querySelector("label.inspector-field input")).not.toBeNull();
    const values = optionValues();
    expect(values).toContain("302");
    expect(values).not.toContain("300");
    expect(host!.textContent).not.toContain("only available");
  });

  it("deletes a later-installation group address", async () => {
    apiMock.deleteGroupAddress.mockResolvedValueOnce(twoInstallationTree());
    await renderInspector({ kind: "group_address", id: 201 }, twoInstallationTree());
    const remove = Array.from(host!.querySelectorAll("button")).find((b) => b.textContent === "Delete");
    expect(remove).toBeTruthy();
    await act(async () => remove!.click());
    expect(apiMock.deleteGroupAddress).toHaveBeenCalledExactlyOnceWith(201);
  });

  it.each(["en", "de"])("keeps a group address read-only when its id belongs to two installations (%s)", async (language) => {
    setSetting(UI_LANGUAGE_STORAGE_KEY, language);
    const tree = twoInstallationTree();
    tree.installations[0].group_addresses = [ga(201, "Twin", "1/1/1")];
    await renderInspector({ kind: "group_address", id: 201 }, tree);
    expect(host!.textContent).toContain(language === "de"
      ? "Löschen ist nur für Gruppenadressen verfügbar, die genau einer Installation angehören."
      : "Delete is only available for group addresses that belong to exactly one installation.");
    expect(host!.textContent).not.toContain("{action}");
    expect(Array.from(host!.querySelectorAll("button")).some((b) => b.textContent === "Delete")).toBe(false);
  });
});

// KL-61 / ADR-0078: the effective type alone hides whether the address's own
// declaration and its linked objects agree.
describe("Inspector — group address type detail", () => {
  function withDetail(detail: GroupAddressNode["dpt_detail"], dpts: string[]): ProjectTree {
    const tree = twoInstallationTree();
    tree.installations[0].group_addresses = [{ ...ga(301, "Dimmer value", "1/0/2"), dpts, dpt_detail: detail }];
    return tree;
  }
  const facts = () =>
    Array.from(host!.querySelectorAll(".inspector-facts dt"), (dt) => [
      dt.textContent,
      dt.nextElementSibling?.textContent ?? "",
    ]);

  it("shows the declaration, the linked types and that they differ in subtype", async () => {
    await renderInspector({ kind: "group_address", id: 301 }, withDetail({
      declared: { state: "Value", text: "DPST-9-1" },
      linked: ["DPST-9-4"],
      outcome: "DeclaredDiffersFromLinked",
    }, ["DPST-9-1"]));
    expect(facts()).toEqual(expect.arrayContaining([
      ["Declared on the address", "DPST-9-1"],
      ["Linked objects state", "DPST-9-4"],
    ]));
    expect(host!.querySelector(".dpt-outcome")?.textContent).toBe(
      "The declaration applies. A linked object states another type of the same size.");
  });

  it("names a size conflict and applies no type", async () => {
    await renderInspector({ kind: "group_address", id: 301 }, withDetail({
      declared: { state: "Value", text: "DPST-1-1" },
      linked: ["DPST-5-1"],
      outcome: "SizeConflict",
    }, ["DPST-1-1", "DPST-5-1"]));
    expect(host!.querySelector(".dpt-outcome")?.textContent).toBe(
      "The declaration and a linked object differ in size, which the project format forbids. No type applies.");
    expect(host!.querySelector(".dpt-outcome")?.classList.contains("dpt-conflict")).toBe(true);
  });

  it.each([
    [{ state: "Absent", text: null }, "none", "No declaration on the address; the linked objects' type applies."],
    [{ state: "Empty", text: null }, "empty", "No declaration on the address; the linked objects' type applies."],
    [{ state: "Malformed", text: "DPT-x" }, "unreadable: DPT-x", "No declaration on the address; the linked objects' type applies."],
  ] as const)("shows a %o declaration as it is stored", async (declared, shown, outcome) => {
    await renderInspector({ kind: "group_address", id: 301 }, withDetail({
      declared: { ...declared },
      linked: [],
      outcome: "Inferred",
    }, []));
    expect(facts()).toEqual(expect.arrayContaining([
      ["Declared on the address", shown],
      ["Linked objects state", "none stated"],
    ]));
    expect(host!.querySelector(".dpt-outcome")?.textContent).toBe(outcome);
  });

  it("explains a store whose older declarations could not be attributed", async () => {
    setSetting(UI_LANGUAGE_STORAGE_KEY, "de");
    await renderInspector({ kind: "group_address", id: 301 }, withDetail({
      declared: { state: "Absent", text: null },
      linked: ["DPST-1-1"],
      outcome: "DeclarationNotLifted",
    }, ["DPST-1-1"]));
    expect(host!.querySelector(".dpt-outcome")?.textContent).toBe(
      "Dieses Projekt enthält Typangaben, die keiner Adresse mehr zugeordnet werden konnten. Es gilt der Typ der verknüpften Objekte; er kann unvollständig sein.");
  });

  it("shows no detail rows when the tree carries none", async () => {
    const tree = twoInstallationTree();
    tree.installations[0].group_addresses = [{ ...ga(301, "Plain", "1/0/2"), dpts: ["DPST-1-1"] }];
    await renderInspector({ kind: "group_address", id: 301 }, tree);
    expect(host!.textContent).not.toContain("Declared on the address");
    expect(host!.querySelector(".dpt-outcome")).toBeNull();
  });
});

// Restyling uses the existing command-backed route, not a display preference.
describe("Inspector — project node", () => {
  it.each(["en", "de"])("restyles an existing project through one authoritative request (%s)", async (language) => {
    setSetting(UI_LANGUAGE_STORAGE_KEY, language);
    const tree = twoInstallationTree();
    const changed = { ...tree, group_address_style: "TwoLevel", can_undo: true, is_modified: true };
    apiMock.setGroupAddressStyle.mockResolvedValueOnce(changed);
    const onApplied = await renderInspector({ kind: "project", id: 0 }, tree);
    const select = host!.querySelector<HTMLSelectElement>("select");
    expect(select).not.toBeNull();
    expect(Array.from(select!.options, ({ value }) => value)).toEqual(["ThreeLevel", "TwoLevel", "Free"]);
    expect(select!.getAttribute("aria-label")).toBe(language === "de" ? "Gruppenadress-Stil" : "Group address style");
    await act(async () => {
      select!.value = "TwoLevel";
      select!.dispatchEvent(new Event("change", { bubbles: true }));
    });
    expect(apiMock.setGroupAddressStyle).toHaveBeenCalledExactlyOnceWith("TwoLevel");
    expect(onApplied).toHaveBeenCalledExactlyOnceWith(changed);
    expect(tree.installations[1].group_addresses[0].address).toBe("2/1/1");
  });

  it("keeps the authoritative style and displays a refused restyle without publishing success", async () => {
    apiMock.setGroupAddressStyle.mockRejectedValueOnce(new Error("style change refused"));
    const onApplied = await renderInspector({ kind: "project", id: 0 }, twoInstallationTree());
    const select = host!.querySelector<HTMLSelectElement>("select");
    expect(select).not.toBeNull();
    await act(async () => {
      select!.value = "Free";
      select!.dispatchEvent(new Event("change", { bubbles: true }));
    });
    expect(onApplied).not.toHaveBeenCalled();
    expect(host!.querySelector('[role="alert"]')?.textContent).toBe("style change refused");
    expect(select!.value).toBe("ThreeLevel");
  });

  it("does not overlap restyle requests while a response is pending", async () => {
    let resolve!: (tree: ProjectTree) => void;
    const pending = new Promise<ProjectTree>((done) => { resolve = done; });
    apiMock.setGroupAddressStyle.mockReturnValueOnce(pending);
    const tree = twoInstallationTree();
    await renderInspector({ kind: "project", id: 0 }, tree);
    const select = host!.querySelector<HTMLSelectElement>("select");
    expect(select).not.toBeNull();
    await act(async () => {
      select!.value = "Free";
      select!.dispatchEvent(new Event("change", { bubbles: true }));
      select!.value = "TwoLevel";
      select!.dispatchEvent(new Event("change", { bubbles: true }));
    });
    expect(apiMock.setGroupAddressStyle).toHaveBeenCalledExactlyOnceWith("Free");
    expect(select!.disabled).toBe(true);
    await act(async () => resolve({ ...tree, group_address_style: "Free" }));
    expect(select!.disabled).toBe(false);
  });

  it("refreshes the style selector from an authoritative Undo snapshot", async () => {
    const tree = { ...twoInstallationTree(), group_address_style: "Free" };
    const onApplied = await renderInspector({ kind: "project", id: 0 }, tree);
    expect(host!.querySelector<HTMLSelectElement>("select")!.value).toBe("Free");
    await act(async () => root!.render(<Inspector selection={{ kind: "project", id: 0 }}
      tree={{ ...tree, group_address_style: "TwoLevel", can_redo: true }} deviceDetail={null}
      onApplied={onApplied} onDeleted={vi.fn()} />));
    expect(host!.querySelector<HTMLSelectElement>("select")!.value).toBe("TwoLevel");
    expect(apiMock.setGroupAddressStyle).not.toHaveBeenCalled();
  });

  it("preserves an unknown imported style without silently defaulting or posting it", async () => {
    await renderInspector({ kind: "project", id: 0 }, { ...twoInstallationTree(), group_address_style: "FutureStyle" });
    const select = host!.querySelector<HTMLSelectElement>("select")!;
    expect(select.value).toBe("FutureStyle");
    expect(select.selectedOptions[0].disabled).toBe(true);
    expect(host!.textContent).toContain("FutureStyle");
    expect(apiMock.setGroupAddressStyle).not.toHaveBeenCalled();
  });

  it("does not post an unchanged project style", async () => {
    await renderInspector({ kind: "project", id: 0 }, twoInstallationTree());
    await act(async () => host!.querySelector<HTMLSelectElement>("select")!
      .dispatchEvent(new Event("change", { bubbles: true })));
    expect(apiMock.setGroupAddressStyle).not.toHaveBeenCalled();
  });

  // MODEL-01: an installation is renamed from the project node, one field
  // per installation, through `PATCH /api/installations/{id}`.
  it.each(["en", "de"])("renames each installation through its own field (%s)", async (language) => {
    setSetting(UI_LANGUAGE_STORAGE_KEY, language);
    const tree = twoInstallationTree();
    const renamed = { ...tree, can_undo: true };
    apiMock.renameInstallation.mockResolvedValueOnce(renamed);
    const onApplied = await renderInspector({ kind: "project", id: 0 }, tree);
    const label = language === "de" ? "Name der Installation" : "Installation name";
    const fields = Array.from(host!.querySelectorAll<HTMLInputElement>(`input[aria-label^="${label}"]`));
    expect(fields.map((field) => field.value)).toEqual(["Installation 1", "Installation 2"]);
    await act(async () => {
      setTextInputValue(fields[1], "Annex");
      fields[1].dispatchEvent(new Event("focusout", { bubbles: true }));
    });
    expect(apiMock.renameInstallation).toHaveBeenCalledExactlyOnceWith(1, "Annex");
    expect(onApplied).toHaveBeenCalledExactlyOnceWith(renamed);
  });

  it("keeps an installation's name and shows the refusal when a rename fails", async () => {
    apiMock.renameInstallation.mockRejectedValueOnce(new Error("installation name must not be empty"));
    const onApplied = await renderInspector({ kind: "project", id: 0 }, twoInstallationTree());
    const field = host!.querySelectorAll<HTMLInputElement>('input[aria-label^="Installation name"]')[0];
    await act(async () => {
      setTextInputValue(field, "Renamed");
      field.dispatchEvent(new Event("focusout", { bubbles: true }));
    });
    expect(field.value).toBe("Installation 1");
    expect(host!.querySelector(".field-error")?.textContent).toBe("installation name must not be empty");
    expect(onApplied).not.toHaveBeenCalled();
  });

  it("shows the project's current group address style", async () => {
    const tree = twoInstallationTree();
    await renderInspector({ kind: "project", id: 0 }, tree);

    expect(host!.textContent).toContain("ThreeLevel");
  });

  it("reflects a non-default style the same way", async () => {
    const tree = { ...twoInstallationTree(), group_address_style: "Free" };
    await renderInspector({ kind: "project", id: 0 }, tree);

    expect(host!.textContent).toContain("Free");
    expect(host!.textContent).not.toContain("ThreeLevel");
  });
});

describe("Inspector — structural move keyboard equivalent", () => {
  it("keeps both native selects wired to the same validated move commands", async () => {
    const nextTree = deviceMoveTree();
    apiMock.moveDeviceToLine.mockResolvedValueOnce(nextTree);
    apiMock.moveDeviceToBuildingPart.mockResolvedValueOnce(nextTree);
    const onApplied = vi.fn();
    await renderInspector(
      { kind: "device", id: 42 },
      deviceMoveTree(),
      deviceDetail(),
      onApplied,
    );
    const fields = Array.from(host!.querySelectorAll<HTMLLabelElement>("label.inspector-field"));
    const lineSelect = fields.find((field) => field.textContent?.startsWith("Line"))
      ?.querySelector("select");
    const buildingSelect = fields.find((field) => field.textContent?.startsWith("Building part"))
      ?.querySelector("select");
    expect(lineSelect).toBeTruthy();
    expect(buildingSelect).toBeTruthy();

    await act(async () => {
      lineSelect!.value = "12";
      lineSelect!.dispatchEvent(new Event("change", { bubbles: true }));
    });
    await act(async () => {
      buildingSelect!.value = "501";
      buildingSelect!.dispatchEvent(new Event("change", { bubbles: true }));
    });

    expect(apiMock.moveDeviceToLine).toHaveBeenCalledWith(42, 12);
    expect(apiMock.moveDeviceToBuildingPart).toHaveBeenCalledWith(42, 501);
    expect(onApplied).toHaveBeenCalledTimes(2);
    expect(onApplied).toHaveBeenCalledWith(nextTree);
  });
});

describe("Inspector — readable KNX communication flags (ISSUE-09)", () => {
  it.each([
    ["en", ["Read", "Write", "Transmit", "Update", "Communication", "Read on init"]],
    ["de", ["Lesen", "Schreiben", "Übertragen", "Aktualisieren", "Kommunikation", "Lesen bei Initialisierung"]],
  ])("shows the standard letters and the %s flag names without hiding them in hover tips", async (language, names) => {
    setSetting(UI_LANGUAGE_STORAGE_KEY, language);
    const nextTree = deviceMoveTree();
    apiMock.setComObjectFlag.mockResolvedValue(nextTree);
    await renderInspector({ kind: "device", id: 42 }, nextTree, detailWithComObject());
    const labels = Array.from(host!.querySelectorAll<HTMLElement>(".com-object-flags label"));
    expect(labels.map((label) => label.querySelector(".flag-code")?.textContent)).toEqual(["R", "W", "T", "U", "C", "I"]);
    expect(labels.map((label) => label.querySelector(".flag-name")?.textContent)).toEqual(names);
    await act(async () => labels[0].querySelector("input")!.click());
    expect(apiMock.setComObjectFlag).toHaveBeenCalledWith(7, "Read", true);
  });
});

describe("Inspector — atomic send-and-receive link action (ISSUE-09)", () => {
  it("offers Both as one request while keeping Send and Receive choices", async () => {
    const tree = deviceMoveTree();
    tree.installations[0].group_addresses = [ga(9, "Kitchen lights", "1/2/3")];
    apiMock.linkComObject.mockResolvedValue(tree);
    const onApplied = await renderInspector({ kind: "device", id: 42 }, tree, detailWithComObject());
    const row = host!.querySelector<HTMLElement>(".group-link-list .tree-new-row")!;
    const selects = row.querySelectorAll<HTMLSelectElement>("select");
    expect(Array.from(selects[1].options).map((option) => option.value)).toEqual(["Send", "Receive", "Both"]);
    await act(async () => {
      selects[0].value = "9";
      selects[0].dispatchEvent(new Event("change", { bubbles: true }));
      selects[1].value = "Both";
      selects[1].dispatchEvent(new Event("change", { bubbles: true }));
    });
    await act(async () => row.querySelector("button")!.click());
    expect(apiMock.linkComObject).toHaveBeenCalledTimes(1);
    expect(apiMock.linkComObject).toHaveBeenCalledWith(7, 9, "Both");
    expect(onApplied).toHaveBeenCalledWith(tree);
  });

  it("reports a refused paired link without refreshing or retrying a partial result", async () => {
    const tree = deviceMoveTree();
    tree.installations[0].group_addresses = [ga(9, "Kitchen lights", "1/2/3")];
    apiMock.linkComObject.mockRejectedValueOnce(new Error("Receive is already linked"));
    const onApplied = await renderInspector(
      { kind: "device", id: 42 },
      tree,
      detailWithComObject([{ ga_id: 9, address: "1/2/3", name: "Kitchen lights", direction: "Receive" }]),
    );
    const row = host!.querySelector<HTMLElement>(".group-link-list .tree-new-row")!;
    const selects = row.querySelectorAll<HTMLSelectElement>("select");
    await act(async () => {
      selects[0].value = "9";
      selects[0].dispatchEvent(new Event("change", { bubbles: true }));
      selects[1].value = "Both";
      selects[1].dispatchEvent(new Event("change", { bubbles: true }));
    });
    await act(async () => row.querySelector("button")!.click());
    expect(apiMock.linkComObject).toHaveBeenCalledTimes(1);
    expect(apiMock.linkComObject).toHaveBeenCalledWith(7, 9, "Both");
    expect(row.querySelector(".field-error")?.textContent).toContain("Receive is already linked");
    expect(host!.querySelectorAll(".group-link-row")).toHaveLength(1);
    expect(onApplied).not.toHaveBeenCalled();
  });

  it("offers one atomic unlink-both action beside separate directional unlink buttons", async () => {
    const tree = deviceMoveTree();
    tree.installations[0].group_addresses = [ga(9, "Kitchen lights", "1/2/3")];
    apiMock.unlinkComObject.mockResolvedValue(tree);
    const links = [
      { ga_id: 9, address: "1/2/3", name: "Kitchen lights", direction: "Send" },
      { ga_id: 9, address: "1/2/3", name: "Kitchen lights", direction: "Receive" },
    ];
    const onApplied = await renderInspector({ kind: "device", id: 42 }, tree, detailWithComObject(links));
    expect(host!.querySelectorAll(".group-link-row")).toHaveLength(2);
    const unlinkBoth = host!.querySelector<HTMLButtonElement>(".group-link-row .unlink-both")!;
    expect(unlinkBoth?.textContent).toBe("Unlink both");
    await act(async () => unlinkBoth.click());
    expect(apiMock.unlinkComObject).toHaveBeenCalledTimes(1);
    expect(apiMock.unlinkComObject).toHaveBeenCalledWith(7, 9, "Both");
    expect(onApplied).toHaveBeenCalledWith(tree);
    const single = host!.querySelector<HTMLButtonElement>(".group-link-row button:not(.unlink-both)")!;
    await act(async () => single.click());
    expect(apiMock.unlinkComObject).toHaveBeenLastCalledWith(7, 9, "Send");
  });
});

describe("Inspector — line-bound physical address (ISSUE-09)", () => {
  it("keeps area and line read-only while saving only an edited device octet as a full address", async () => {
    const tree = deviceMoveTree();
    const detail = { ...deviceDetail(), address: "1.1.12" };
    apiMock.setIndividualAddress.mockResolvedValue(tree);
    const onApplied = await renderInspector({ kind: "device", id: 42 }, tree, detail);
    const field = host!.querySelector<HTMLElement>(".individual-address-field")!;
    const prefix = field.querySelector<HTMLElement>(".address-prefix")!;
    expect(prefix.textContent).toBe("1.1.");
    const input = field.querySelector<HTMLInputElement>("input")!;
    expect(prefix.getAttribute("aria-hidden")).toBeNull();
    expect(input.getAttribute("aria-describedby")).toBe(prefix.id);
    expect(prefix.id).toBe("device-address-prefix-42");
    expect(input.value).toBe("12");
    await act(async () => setTextInputValue(input, "17"));
    await act(async () => {
      input.dispatchEvent(new FocusEvent("focusout", { bubbles: true }));
      await Promise.resolve();
    });
    expect(apiMock.setIndividualAddress).toHaveBeenCalledWith(42, "1.1.17");
    expect(onApplied).toHaveBeenCalledWith(tree);
  });

  it("rejects non-device numbers before contacting the API", async () => {
    const tree = deviceMoveTree();
    await renderInspector({ kind: "device", id: 42 }, tree, deviceDetail());
    const field = host!.querySelector<HTMLElement>(".individual-address-field")!;
    const input = field.querySelector<HTMLInputElement>("input")!;
    for (const bad of ["256", "1.2", "-1"]) {
      await act(async () => setTextInputValue(input, bad));
      await act(async () => {
        input.dispatchEvent(new FocusEvent("focusout", { bubbles: true }));
        await Promise.resolve();
      });
      expect(field.querySelector(".field-error")?.textContent).toBeTruthy();
      expect(apiMock.setIndividualAddress).not.toHaveBeenCalled();
    }
  });

  // MODEL-03: only the server knows whether the product database marks this
  // product as a coupler (`Hardware/@IsCoupler`), so the editor must ask it.
  it("submits device number 0 so the server can accept an evidenced coupler", async () => {
    const tree = deviceMoveTree();
    apiMock.setIndividualAddress.mockResolvedValue(tree);
    const onApplied = await renderInspector({ kind: "device", id: 42 }, tree, deviceDetail());
    const field = host!.querySelector<HTMLElement>(".individual-address-field")!;
    const input = field.querySelector<HTMLInputElement>("input")!;
    await act(async () => setTextInputValue(input, "0"));
    await act(async () => {
      input.dispatchEvent(new FocusEvent("focusout", { bubbles: true }));
      await Promise.resolve();
    });
    expect(apiMock.setIndividualAddress).toHaveBeenCalledExactlyOnceWith(42, "1.1.0");
    expect(onApplied).toHaveBeenCalledWith(tree);
    expect(field.querySelector(".field-error")).toBeNull();
  });

  it("shows the server's refusal when device number 0 is not an evidenced coupler", async () => {
    const tree = deviceMoveTree();
    const refusal = "address 1.1.0 ends in 0, reserved for couplers; device 42 cannot be assigned "
      + "a new coupler address without device classification";
    apiMock.setIndividualAddress.mockRejectedValue(new Error(refusal));
    const onApplied = await renderInspector({ kind: "device", id: 42 }, tree, { ...deviceDetail(), address: "1.1.12" });
    const field = host!.querySelector<HTMLElement>(".individual-address-field")!;
    const input = field.querySelector<HTMLInputElement>("input")!;
    await act(async () => setTextInputValue(input, "0"));
    await act(async () => {
      input.dispatchEvent(new FocusEvent("focusout", { bubbles: true }));
      await Promise.resolve();
    });
    expect(apiMock.setIndividualAddress).toHaveBeenCalledExactlyOnceWith(42, "1.1.0");
    expect(field.querySelector(".field-error")?.textContent).toBe(refusal);
    expect(input.value).toBe("12");
    expect(onApplied).not.toHaveBeenCalled();
  });

  it("preserves the full-address editor for unassigned devices, including clearing", async () => {
    const tree = deviceMoveTree();
    tree.installations[0].unassigned = tree.installations[0].topology[0].lines[0].devices.splice(0);
    apiMock.setIndividualAddress.mockResolvedValue(tree);
    const onApplied = await renderInspector({ kind: "device", id: 42 }, tree, { ...deviceDetail(), address: "2.3.7" });
    const field = host!.querySelector<HTMLElement>(".individual-address-field")!;
    expect(field.querySelector(".address-prefix")).toBeNull();
    const input = field.querySelector<HTMLInputElement>("input")!;
    expect(input.value).toBe("2.3.7");
    await act(async () => setTextInputValue(input, ""));
    await act(async () => {
      input.dispatchEvent(new FocusEvent("focusout", { bubbles: true }));
      await Promise.resolve();
    });
    expect(apiMock.setIndividualAddress).toHaveBeenCalledWith(42, null);
    expect(onApplied).toHaveBeenCalledWith(tree);
  });

  it("shows an imported mismatch without silently rewriting it on blur", async () => {
    const tree = deviceMoveTree();
    apiMock.setIndividualAddress.mockResolvedValue(tree);
    await renderInspector({ kind: "device", id: 42 }, tree, { ...deviceDetail(), address: "2.3.9" });
    const field = host!.querySelector<HTMLElement>(".individual-address-field")!;
    expect(field.textContent).toContain("2.3.9");
    const input = field.querySelector<HTMLInputElement>("input")!;
    await act(async () => input.dispatchEvent(new FocusEvent("focusout", { bubbles: true })));
    expect(apiMock.setIndividualAddress).not.toHaveBeenCalled();
    await act(async () => setTextInputValue(input, "17"));
    await act(async () => {
      input.dispatchEvent(new FocusEvent("focusout", { bubbles: true }));
      await Promise.resolve();
    });
    expect(apiMock.setIndividualAddress).toHaveBeenCalledWith(42, "1.1.17");
  });

  it("reports a duplicate address without applying or forgetting the original", async () => {
    const tree = deviceMoveTree();
    const onApplied = await renderInspector({ kind: "device", id: 42 }, tree, { ...deviceDetail(), address: "1.1.12" });
    apiMock.setIndividualAddress.mockRejectedValueOnce(new Error("individual address 1.1.17 already used"));
    const field = host!.querySelector<HTMLElement>(".individual-address-field")!;
    const input = field.querySelector<HTMLInputElement>("input")!;
    await act(async () => setTextInputValue(input, "17"));
    await act(async () => {
      input.dispatchEvent(new FocusEvent("focusout", { bubbles: true }));
      await Promise.resolve();
    });
    expect(apiMock.setIndividualAddress).toHaveBeenCalledWith(42, "1.1.17");
    expect(field.querySelector(".field-error")?.textContent).toContain("already used");
    expect(input.value).toBe("12");
    expect(onApplied).not.toHaveBeenCalled();
  });

  it("uses the line in the owning installation, not the first installation", async () => {
    const tree = twoInstallationTree();
    const area = deviceMoveTree().installations[0].topology[0];
    area.address = 2;
    area.lines[0].address = 3;
    tree.installations[1].topology = [area];
    apiMock.setIndividualAddress.mockResolvedValue(tree);
    await renderInspector({ kind: "device", id: 42 }, tree, { ...deviceDetail(), address: "2.3.9" });
    const field = host!.querySelector<HTMLElement>(".individual-address-field")!;
    expect(field.querySelector(".address-prefix")?.textContent).toBe("2.3.");
    const input = field.querySelector<HTMLInputElement>("input")!;
    await act(async () => setTextInputValue(input, "18"));
    await act(async () => {
      input.dispatchEvent(new FocusEvent("focusout", { bubbles: true }));
      await Promise.resolve();
    });
    expect(apiMock.setIndividualAddress).toHaveBeenCalledWith(42, "2.3.18");
  });

  it("disables the address editor when a device is both on a line and unassigned", async () => {
    const tree = deviceMoveTree();
    tree.installations[0].unassigned.push(tree.installations[0].topology[0].lines[0].devices[0]);
    await renderInspector({ kind: "device", id: 42 }, tree, deviceDetail());
    const field = host!.querySelector<HTMLElement>(".individual-address-field")!;
    expect(field.querySelector<HTMLInputElement>("input")!.disabled).toBe(true);
    expect(field.querySelector(".field-error")?.textContent).toContain("no unambiguous owning area and line");
    expect(apiMock.setIndividualAddress).not.toHaveBeenCalled();
  });

  it("disables the address editor for a repeated device reference in one line", async () => {
    const tree = deviceMoveTree();
    tree.installations[0].topology[0].lines[0].devices.push(tree.installations[0].topology[0].lines[0].devices[0]);
    await renderInspector({ kind: "device", id: 42 }, tree, deviceDetail());
    const field = host!.querySelector<HTMLElement>(".individual-address-field")!;
    expect(field.querySelector<HTMLInputElement>("input")!.disabled).toBe(true);
    expect(field.querySelector(".field-error")?.textContent).toContain("no unambiguous owning area and line");
    expect(apiMock.setIndividualAddress).not.toHaveBeenCalled();
  });

  it("refuses to guess when the tree lists a device on two lines", async () => {
    const tree = deviceMoveTree();
    tree.installations[0].topology[0].lines[1].devices.push(tree.installations[0].topology[0].lines[0].devices[0]);
    await renderInspector({ kind: "device", id: 42 }, tree, deviceDetail());
    const field = host!.querySelector<HTMLElement>(".individual-address-field")!;
    expect(field.querySelector<HTMLInputElement>("input")!.disabled).toBe(true);
    expect(field.querySelector(".field-error")?.textContent).toContain("no unambiguous owning area and line");
    expect(apiMock.setIndividualAddress).not.toHaveBeenCalled();
  });
});

// MODEL-02 / ADR-0071: an ambiguous placement is repaired only by an explicit
// choice of the slot to keep; nothing is chosen for the user.
describe("Inspector — placement repair", () => {
  const keepButtons = (label: string) =>
    Array.from(host!.querySelectorAll("button")).filter((button) => button.textContent === label);

  it("offers to keep each current placement of a device placed twice", async () => {
    const tree = deviceMoveTree();
    tree.installations[0].unassigned.push(tree.installations[0].topology[0].lines[0].devices[0]);
    const repaired = deviceMoveTree();
    apiMock.repairDevicePlacement.mockResolvedValueOnce(repaired);
    const onApplied = await renderInspector({ kind: "device", id: 42 }, tree, deviceDetail());
    expect(host!.textContent).toContain("Placement conflict");
    const buttons = keepButtons("Keep this placement");
    expect(buttons).toHaveLength(2);
    await act(async () => buttons[0].click());
    expect(apiMock.repairDevicePlacement).toHaveBeenCalledExactlyOnceWith(42, { lineId: 11 });
    expect(onApplied).toHaveBeenCalledExactlyOnceWith(repaired);
  });

  it("keeps the unassigned slot of the named installation", async () => {
    const tree = deviceMoveTree();
    tree.installations[0].unassigned.push(tree.installations[0].topology[0].lines[0].devices[0]);
    apiMock.repairDevicePlacement.mockResolvedValueOnce(deviceMoveTree());
    await renderInspector({ kind: "device", id: 42 }, tree, deviceDetail());
    await act(async () => keepButtons("Keep this placement")[1].click());
    expect(apiMock.repairDevicePlacement).toHaveBeenCalledExactlyOnceWith(42, { unassignedInstallationId: 1 });
  });

  it("shows a refused repair and keeps the conflict on screen", async () => {
    const tree = deviceMoveTree();
    tree.installations[0].topology[0].lines[1].devices.push(tree.installations[0].topology[0].lines[0].devices[0]);
    apiMock.repairDevicePlacement.mockRejectedValueOnce(new Error("repair not needed"));
    const onApplied = await renderInspector({ kind: "device", id: 42 }, tree, deviceDetail());
    await act(async () => keepButtons("Keep this placement")[1].click());
    expect(apiMock.repairDevicePlacement).toHaveBeenCalledExactlyOnceWith(42, { lineId: 12 });
    expect(host!.querySelector(".placement-repair .field-error")?.textContent).toBe("repair not needed");
    expect(onApplied).not.toHaveBeenCalled();
    expect(keepButtons("Keep this placement")).toHaveLength(2);
  });

  it.each(["en", "de"])("offers no repair for a device placed once (%s)", async (language) => {
    setSetting(UI_LANGUAGE_STORAGE_KEY, language);
    await renderInspector({ kind: "device", id: 42 }, deviceMoveTree(), deviceDetail());
    expect(host!.querySelector(".placement-repair")).toBeNull();
  });

  it("names the conflict in German", async () => {
    setSetting(UI_LANGUAGE_STORAGE_KEY, "de");
    const tree = deviceMoveTree();
    tree.installations[0].unassigned.push(tree.installations[0].topology[0].lines[0].devices[0]);
    await renderInspector({ kind: "device", id: 42 }, tree, deviceDetail());
    expect(host!.textContent).toContain("Platzierungskonflikt");
    expect(keepButtons("Diese Platzierung behalten")).toHaveLength(2);
  });

  it("offers to keep a line listed by two areas under one of them", async () => {
    const tree = deviceMoveTree();
    const line = tree.installations[0].topology[0].lines[0];
    tree.installations[0].topology.push({ id: 20, name: "Area B", address: 2, lines: [line] });
    const repaired = deviceMoveTree();
    apiMock.repairLineOwner.mockResolvedValueOnce(repaired);
    const onApplied = await renderInspector({ kind: "line", id: 11 }, tree);
    expect(host!.textContent).toContain("This structure ID occurs more than once");
    const buttons = keepButtons("Keep under this area");
    expect(buttons).toHaveLength(2);
    await act(async () => buttons[1].click());
    expect(apiMock.repairLineOwner).toHaveBeenCalledExactlyOnceWith(11, 20);
    expect(onApplied).toHaveBeenCalledExactlyOnceWith(repaired);
  });

  it("offers no line-owner repair for two different lines that share an id", async () => {
    const tree = deviceMoveTree();
    tree.installations[0].topology.push({ id: 20, name: "Area B", address: 2,
      lines: [{ id: 11, name: "Other line", address: 5, devices: [] }] });
    await renderInspector({ kind: "line", id: 11 }, tree);
    expect(host!.textContent).toContain("This structure ID occurs more than once");
    expect(keepButtons("Keep under this area")).toHaveLength(0);
  });

  it("offers no line-owner repair when the shared line's id also occurs in another installation", async () => {
    const tree = deviceMoveTree();
    const line = tree.installations[0].topology[0].lines[0];
    tree.installations[0].topology.push({ id: 20, name: "Area B", address: 2, lines: [line] });
    const later = twoInstallationTree().installations[1];
    later.topology = [{ id: 30, name: "Later area", address: 3, lines: [{ ...line }] }];
    tree.installations.push(later);
    await renderInspector({ kind: "line", id: 11 }, tree);
    expect(keepButtons("Keep under this area")).toHaveLength(0);
  });

  it("offers no line-owner repair when the line id occurs in two installations", async () => {
    const tree = deviceMoveTree();
    const later = twoInstallationTree().installations[1];
    later.topology = [{ id: 30, name: "Later area", address: 3, lines: [{ id: 11, name: "Twin", address: 1, devices: [] }] }];
    tree.installations.push(later);
    await renderInspector({ kind: "line", id: 11 }, tree);
    expect(host!.textContent).toContain("This structure ID occurs more than once");
    expect(keepButtons("Keep under this area")).toHaveLength(0);
  });
});

// UX-01: dropping a group address on a communication object's link row links
// it once, in the direction chosen in that row (the keyboard path's choice).
describe("Inspector — group address drop", () => {
  class DropTransfer {
    private readonly values = new Map<string, string>();
    dropEffect = "none";
    effectAllowed = "link";
    get types(): string[] { return [...this.values.keys()]; }
    setData(format: string, data: string) { this.values.set(format, data); }
    getData(format: string) { return this.values.get(format) ?? ""; }
  }

  async function drag(target: HTMLElement, type: "dragover" | "drop" | "dragleave", transfer: DropTransfer) {
    const event = new Event(type, { bubbles: true, cancelable: true });
    Object.defineProperty(event, "dataTransfer", { value: transfer });
    await act(async () => target.dispatchEvent(event));
    return event;
  }

  async function linkRow(tree = linkTree()) {
    const onApplied = await renderInspector({ kind: "device", id: 42 }, tree, detailWithComObject());
    return { row: host!.querySelector<HTMLElement>(".group-link-list .tree-new-row")!, onApplied };
  }

  function linkTree(): ProjectTree {
    const tree = deviceMoveTree();
    tree.installations[0].group_addresses = [ga(9, "Kitchen lights", "1/2/3")];
    return tree;
  }

  function carrying(id: string): DropTransfer {
    const transfer = new DropTransfer();
    transfer.setData("application/x-knxbench-group-address-id", id);
    return transfer;
  }

  it("accepts a dragged group address and links it once with the row's direction", async () => {
    const tree = linkTree();
    apiMock.linkComObject.mockResolvedValueOnce(tree);
    const { row, onApplied } = await linkRow(tree);
    const over = await drag(row, "dragover", carrying("9"));
    expect(over.defaultPrevented).toBe(true);
    expect(row.getAttribute("data-drop-ready")).toBe("true");
    await drag(row, "drop", carrying("9"));
    expect(apiMock.linkComObject).toHaveBeenCalledExactlyOnceWith(7, 9, "Send");
    expect(onApplied).toHaveBeenCalledExactlyOnceWith(tree);
    expect(row.getAttribute("data-drop-ready")).toBeNull();
  });

  it("uses the direction already chosen in the row", async () => {
    apiMock.linkComObject.mockResolvedValueOnce(linkTree());
    const { row } = await linkRow();
    const direction = row.querySelectorAll<HTMLSelectElement>("select")[1];
    await act(async () => {
      direction.value = "Receive";
      direction.dispatchEvent(new Event("change", { bubbles: true }));
    });
    await drag(row, "drop", carrying("9"));
    expect(apiMock.linkComObject).toHaveBeenCalledExactlyOnceWith(7, 9, "Receive");
  });

  it("refuses a group address this device cannot link, without a request", async () => {
    const { row } = await linkRow();
    await drag(row, "drop", carrying("77"));
    expect(apiMock.linkComObject).not.toHaveBeenCalled();
    expect(row.querySelector(".field-error")?.textContent)
      .toBe("This group address cannot be linked to this device.");
  });

  it("shows the server's refusal of a dropped link", async () => {
    apiMock.linkComObject.mockRejectedValueOnce(new Error("link already exists"));
    const { row, onApplied } = await linkRow();
    await drag(row, "drop", carrying("9"));
    expect(row.querySelector(".field-error")?.textContent).toBe("link already exists");
    expect(onApplied).not.toHaveBeenCalled();
  });

  it("ignores foreign drags such as a device", async () => {
    const { row } = await linkRow();
    const device = new DropTransfer();
    device.setData("application/x-knxbench-device-id", "9");
    const over = await drag(row, "dragover", device);
    expect(over.defaultPrevented).toBe(false);
    expect(row.getAttribute("data-drop-ready")).toBeNull();
    await drag(row, "drop", device);
    expect(apiMock.linkComObject).not.toHaveBeenCalled();
  });
});

// KL-37 (AR10 slice 2b): the device's product block and a communication
// object's DPT text say which stored language answered (`*_language`) or are
// the package's / master data's own text (field absent). Marked only with a
// product language selected; the declared source language is named when the
// package states one.
describe("Inspector — product texts in a selected product language (KL-37)", () => {
  afterEach(() => resetProductLanguageForTests());

  const catalogDetail = (): DeviceDetail => ({
    ...detailWithComObject(),
    product: {
      product_ref: "P-1", program_ref: "AP-1", resolution: "Resolved",
      catalog: {
        manufacturer_id: "M-1", manufacturer_name: "Maker", product_text: "Schaltaktor", order_number: "SA-1",
        hardware_name: null, hardware_version: null, hardware_serial_number: null,
        catalog_item_name: "Switch actuator", catalog_item_number: null, application_program_id: "AP-1",
        application_name: "Switching", application_number: null, application_version: null, mask_version: null,
        product_text_language: "de-DE", product_source_language: "en-US",
        catalog_item_source_language: "en-US",
      },
    },
    com_objects: [{ ...detailWithComObject().com_objects[0], dpt_text: "switch" }],
  });
  const badgeBeside = (value: string) => {
    const row = [...host!.querySelectorAll(".identity-row")].find((r) => r.querySelector("dd")!.textContent!.startsWith(value))!;
    return row.querySelector(".language-fallback-badge")?.textContent ?? null;
  };

  it("marks product texts that fell back, naming the declared source language when there is one", async () => {
    setSetting(PRODUCT_LANGUAGE_STORAGE_KEY, "de");
    await renderInspector({ kind: "device", id: 42 }, laterInstallationTree(), catalogDetail());
    expect(badgeBeside("Schaltaktor")).toBeNull();
    expect(badgeBeside("Switch actuator")).toBe("Untranslated (en-US)");
    expect(badgeBeside("Switching")).toBe("Untranslated");
    expect(badgeBeside("Maker")).toBeNull();
    const dpt = host!.querySelector(".com-object-effective-dpt")!;
    expect(dpt.querySelector(".language-fallback-badge")!.textContent).toBe("Untranslated");
  });

  it("does not mark a DPT text the master data translated", async () => {
    setSetting(PRODUCT_LANGUAGE_STORAGE_KEY, "de");
    const detail = catalogDetail();
    detail.com_objects = [{ ...detail.com_objects[0], dpt_text: "schalten", dpt_text_language: "de-DE" }];
    await renderInspector({ kind: "device", id: 42 }, laterInstallationTree(), detail);
    expect(host!.querySelector(".com-object-effective-dpt .language-fallback-badge")).toBeNull();
  });

  it("marks nothing without a selected product language", async () => {
    await renderInspector({ kind: "device", id: 42 }, laterInstallationTree(), catalogDetail());
    expect(host!.querySelectorAll(".language-fallback-badge")).toHaveLength(0);
  });
});
