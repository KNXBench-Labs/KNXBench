export type Selection =
  | { kind: "device"; id: number }
  | { kind: "group_address"; id: number }
  | { kind: "group_range"; id: number }
  | { kind: "building_part"; id: number }
  | { kind: "area"; id: number }
  | { kind: "line"; id: number };

// Ctrl/Cmd-click and shift-click multi-select in the Project Explorer
// (T9, GAP_ANALYSIS_ETS.md B9) — additive, separate state from `Selection`
// above. Only devices and group addresses are multi-selectable: the bulk
// toolbar's actions (delete, move-to-line, move-to-building-part) are all
// kind-specific, so a mixed-kind batch would have no single coherent
// action. `ids` is a `Set` (not an array) since toggling membership on
// ctrl-click is the dominant operation.
export type MultiSelectionKind = "device" | "group_address";
export type MultiSelection = { kind: MultiSelectionKind; ids: Set<number> };
