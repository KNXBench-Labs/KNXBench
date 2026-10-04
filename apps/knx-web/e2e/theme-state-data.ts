/** Synthetic projections and admitted palettes for the offline closing-state fixture. */
// SPDX-License-Identifier: AGPL-3.0-or-later
import type { ProjectTree } from "../src/bindings/ProjectTree";
import type { DeviceDetail } from "../src/bindings/DeviceDetail";
import { themePackFixture } from "../src/themePackFixtures";
import { validateThemePack } from "../src/themePack";

export const stateTree: ProjectTree = {
  schema_version: 11, errors: 0, warnings: 0, can_undo: false, can_redo: false,
  is_modified: false, group_address_style: "ThreeLevel",
  installations: [{ id: 1, name: "Synthetic installation", topology: [{ id: 10, name: "Area", address: 1,
    lines: [{ id: 11, name: "Line", address: 1, devices: [{ id: 42, name: "Synthetic actuator",
      address: "1.1.12", description: null, com_object_count: 0 }] }] }],
    buildings: [], unassigned: [], group_ranges: [], group_addresses: [
      { id: 9, name: "Lighting", address: "1/1/1", range: null, dpts: ["DPST-1-1"], links: [] },
      { id: 13, name: "Heating", address: "1/1/2", range: null, dpts: [], links: [] },
    ] }],
};
export const stateDevice: DeviceDetail = {
  id: 42, name: "Synthetic actuator", address: "1.1.12", description: null,
  product: { product_ref: null, program_ref: null, catalog: null, resolution: "NoReference" }, com_objects: [],
};
export function themeStatePacks() {
  const light = { ...themePackFixture(), id: "user-orchid", name: "Orchid", accents: {
    mint: { "--knx-accent": "#137551", "--knx-on-accent": "#ffffff" },
    blue: { "--knx-accent": "#7041dc", "--knx-on-accent": "#ffffff" },
  } };
  const dark = { ...light, id: "user-midnight", name: "Midnight", colorScheme: "dark", tokens: {
    ...light.tokens, "--knx-bg": "#161b22", "--knx-surface": "#222b36", "--knx-foreground": "#f5f7ff",
    "--knx-muted": "#adb7cd", "--knx-border": "#485065", "--knx-error-color": "#ff9b9b",
    "--knx-warning-color": "#eac45c", "--knx-success-color": "#80d5b5",
  } };
  return [light, dark].map((raw) => {
    const admitted = validateThemePack(raw);
    if (!admitted.ok) throw new Error("Synthetic state palette violates the actual admission contract");
    return admitted.pack;
  });
}
