/** Direct group-address naming is available in every installation. */
// @vitest-environment happy-dom
import { act } from "react";
import { createRoot } from "react-dom/client";
import { expect, it, vi } from "vitest";
import Inspector from "./Inspector";
import type { ProjectTree } from "./bindings/ProjectTree";
vi.mock("./api", () => ({ errorMessage: (e: unknown) => String(e), renameEntity: vi.fn() }));
it("offers an editable Name field for a second-installation group address", async () => {
  const tree: ProjectTree = { schema_version: 11, errors: 0, warnings: 0,
    can_undo: false, can_redo: false, is_modified: false, group_address_style: "ThreeLevel",
    server_incarnation: "rename", snapshot_revision: 1, project_incarnation: 0,
    installations: [1, 2].map((id) => ({ id, name: `I${id}`, topology: [], buildings: [],
      unassigned: [], group_ranges: [], group_addresses: id === 1 ? [] : [
        { id: 2, name: "Original", address: "1/1/1", range: null, dpts: [], links: [] }] })) };
  const host = document.createElement("div"); document.body.append(host);
  const root = createRoot(host);
  try {
    await act(async () => root.render(<Inspector selection={{ kind: "group_address", id: 2 }}
      tree={tree} deviceDetail={null} onApplied={vi.fn()} onDeleted={vi.fn()} />));
    expect(host.querySelector<HTMLInputElement>('input[data-rename-name]')?.value).toBe("Original");
  } finally { await act(async () => root.unmount()); host.remove(); }
});
