/** Shared name-editor fixtures with explicit server/project/revision identity. */
import type { ProjectTree } from "./bindings/ProjectTree";
export function renameTree(): ProjectTree {
  return { schema_version: 11, errors: 0, warnings: 0, can_undo: false, can_redo: false,
    is_modified: false, group_address_style: "ThreeLevel", server_incarnation: "rename",
    snapshot_revision: 1, project_incarnation: 0,
    installations: [1, 2].map((id) => ({ id, name: `I${id}`, topology: [], buildings: [],
      unassigned: [{ id, name: "Original", address: null, description: null, com_object_count: 0 }],
      group_ranges: [], group_addresses: [{ id, name: "Original", address: "1/1/1", range: null, dpts: [], links: [] }] })) };
}
