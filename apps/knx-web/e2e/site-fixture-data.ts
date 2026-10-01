/** Synthetic single-installation site fixture; no production project or device data. */
import type { ProjectTree } from "../src/bindings/ProjectTree";

const northDevice = { id: 9, name: "North actuator", address: "1.2.9", description: null, com_object_count: 1 };
const southDevice = { id: 10, name: "South actuator", address: "1.2.10", description: null, com_object_count: 1 };

export const initialTree: ProjectTree = {
  schema_version: 11, errors: 0, warnings: 0, can_undo: false, can_redo: false,
  is_modified: false, group_address_style: "ThreeLevel",
  installations: [{
    id: 1, name: "Shared installation", unassigned: [], group_addresses: [], group_ranges: [],
    topology: [{ id: 2, name: "Area", address: 1, lines: [{
      id: 3, name: "Line", address: 2, devices: [northDevice, southDevice],
    }] }],
    buildings: [
      { id: 4, name: "North building", kind: "Building", devices: [northDevice], children: [] },
      { id: 6, name: "South building", kind: "Building", devices: [southDevice], children: [] },
    ],
  }],
};
