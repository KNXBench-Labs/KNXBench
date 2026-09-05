export type Selection =
  | { kind: "device"; id: number }
  | { kind: "group_address"; id: number }
  | { kind: "building_part"; id: number };
