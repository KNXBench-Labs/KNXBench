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
  moveDeviceToBuildingPart: vi.fn(),
}));

vi.mock("./api", () => ({
  ...apiMock,
  errorMessage: (error: unknown) => (error instanceof Error ? error.message : String(error)),
}));

import Inspector from "./Inspector";

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

describe("Inspector — collapsed delete-restriction message", () => {
  it("renders the 'Delete is only available for group addresses…' variant for a second-installation group address", async () => {
    const tree = twoInstallationTree();
    await renderInspector({ kind: "group_address", id: 201 }, tree);

    expect(host!.textContent).toContain(
      "Delete is only available for group addresses in the first installation.",
    );
    // The action clause is entity-specific text, not a raw discriminant —
    // no leftover template placeholder should ever reach the DOM.
    expect(host!.textContent).not.toContain("{action}");
    expect(host!.textContent).not.toContain("{entity}");
  });

  it("renders the 'Rename and Delete are only available for group ranges…' variant for a second-installation group range", async () => {
    const tree = twoInstallationTree();
    await renderInspector({ kind: "group_range", id: 301 }, tree);

    expect(host!.textContent).toContain(
      "Rename and Delete are only available for group ranges in the first installation.",
    );
  });
});

// KNOWN_LIMITATIONS.md §84 — a project's group address style, once chosen,
// used to be invisible again. This is display only: no button, dropdown, or
// route call lives in `ProjectInspector`, just `tree.group_address_style`
// read back onto the screen.
describe("Inspector — project node", () => {
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
