/** Tests for Inspector's collapsed delete-restriction message on non-empty group ranges. */
// @vitest-environment happy-dom
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { ProjectTree } from "./bindings/ProjectTree";
import type { InstallationNode } from "./bindings/InstallationNode";
import type { GroupAddressNode } from "./bindings/GroupAddressNode";
import type { GroupRangeNode } from "./bindings/GroupRangeNode";
import type { Selection } from "./selection";

const apiMock = vi.hoisted(() => ({
  deviceParameters: vi.fn(),
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
    installations: [first, second],
  };
}

async function renderInspector(selection: Selection, tree: ProjectTree) {
  host = document.createElement("div");
  document.body.appendChild(host);
  root = createRoot(host);
  await act(async () => {
    root!.render(
      <Inspector
        selection={selection}
        tree={tree}
        deviceDetail={null}
        onApplied={vi.fn()}
        onDeleted={vi.fn()}
      />,
    );
  });
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
