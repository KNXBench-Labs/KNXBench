export type Selection =
  | { kind: "device"; id: number }
  | { kind: "group_address"; id: number }
  | { kind: "group_range"; id: number }
  | { kind: "building_part"; id: number }
  | { kind: "area"; id: number }
  | { kind: "line"; id: number };
