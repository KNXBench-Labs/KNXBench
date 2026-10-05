/** U20: validated AR20 flow wire types; unsafe numbers and unknown names are refused. */
// Contract: docs/TELEGRAM_FLOW_VISUALIZATION.md §10 (`apps/knx-server/src/flow.rs`,
// `bus_routes.rs`). Hand-written like the other `/api/bus/*` DTOs. Nothing
// here is coerced: a value that is not exactly what the server states is a
// refusal, because a rounded id or a guessed generation would attach traffic
// to the wrong participant.

export type FlowSnapshotStatus = "current" | "historical" | "unavailable";
export type FlowAddressStyle = "Free" | "TwoLevel" | "ThreeLevel";
export type FlowDirection = "Send" | "Receive";

/** Each flag as stated by the project; `null` = no layer states it. */
export interface FlowFlags {
  communication: boolean | null;
  read: boolean | null;
  write: boolean | null;
  transmit: boolean | null;
  update: boolean | null;
  readOnInit: boolean | null;
}

export interface FlowDevice {
  deviceId: number;
  installationId: number | null;
  name: string;
  individualAddressRaw: number | null;
}

/** Configuration evidence: a linked object, not proof of delivery. */
export interface FlowMember {
  deviceId: number;
  comObjectId: number;
  direction: FlowDirection;
  active: boolean;
  flags: FlowFlags;
}

export interface FlowGroup {
  gaRaw: number;
  gaId: number;
  installationId: number;
  name: string;
  dpt: string | null;
  members: FlowMember[];
}

export interface FlowDiagnostics {
  duplicateIndividualAddresses: { individualAddressRaw: number; deviceIds: number[] }[];
  ambiguousGroupAddresses: { gaRaw: number; gaIds: number[] }[];
  ambiguousDevices: number[];
  danglingLinks: { comObjectId: number; gaId: number }[];
  unknownDevices: { comObjectId: number; deviceId: number }[];
  objectsWithoutFlags: number[];
}

export interface FlowTruncated {
  devices: number;
  groups: number;
  members: number;
  diagnostics: number;
}

export interface FlowSnapshot {
  serverIncarnation: string;
  sessionId: number;
  /** Decimal string (u64 on the server); compared as text, never as a number. */
  generation: string;
  status: FlowSnapshotStatus;
  groupAddressStyle: FlowAddressStyle | null;
  devices: FlowDevice[];
  groups: FlowGroup[];
  diagnostics: FlowDiagnostics;
  truncated: FlowTruncated;
}

const U8 = 0xff;
const U16 = 0xffff;
const U32 = 0xffff_ffff;
const GENERATION = /^(0|[1-9][0-9]*)$/;

class FlowWireError extends Error {}

function refuse(path: string, what: string): never {
  throw new FlowWireError(`flow snapshot: ${path} ${what}`);
}

function object(value: unknown, path: string): Record<string, unknown> {
  if (typeof value !== "object" || value === null || Array.isArray(value)) refuse(path, "is not an object");
  return value as Record<string, unknown>;
}

function array(value: unknown, path: string): unknown[] {
  if (!Array.isArray(value)) refuse(path, "is not a list");
  return value;
}

function integer(value: unknown, path: string, max: number): number {
  if (typeof value !== "number" || !Number.isSafeInteger(value) || value < 0 || value > max) {
    refuse(path, `is not an integer in 0..${max}`);
  }
  return value;
}

function nullableInteger(value: unknown, path: string, max: number): number | null {
  return value === null ? null : integer(value, path, max);
}

function text(value: unknown, path: string): string {
  if (typeof value !== "string") refuse(path, "is not text");
  return value;
}

function nullableText(value: unknown, path: string): string | null {
  return value === null ? null : text(value, path);
}

function oneOf<T extends string>(value: unknown, path: string, allowed: readonly T[]): T {
  if (typeof value !== "string" || !(allowed as readonly string[]).includes(value)) refuse(path, "has an unknown value");
  return value as T;
}

// Every one of the six keys must be present: `null` is a statement ("no
// layer states it"), a missing key is a deviation from the contract.
function flag(value: unknown, path: string): boolean | null {
  if (value !== null && typeof value !== "boolean") refuse(path, "is not true, false or null");
  return value as boolean | null;
}

function flags(value: unknown, path: string): FlowFlags {
  const raw = object(value, path);
  return {
    communication: flag(raw.communication, `${path}.communication`),
    read: flag(raw.read, `${path}.read`),
    write: flag(raw.write, `${path}.write`),
    transmit: flag(raw.transmit, `${path}.transmit`),
    update: flag(raw.update, `${path}.update`),
    readOnInit: flag(raw.readOnInit, `${path}.readOnInit`),
  };
}

function member(value: unknown, path: string): FlowMember {
  const raw = object(value, path);
  if (typeof raw.active !== "boolean") refuse(`${path}.active`, "is not a boolean");
  return {
    deviceId: integer(raw.deviceId, `${path}.deviceId`, U32),
    comObjectId: integer(raw.comObjectId, `${path}.comObjectId`, U32),
    direction: oneOf(raw.direction, `${path}.direction`, ["Send", "Receive"] as const),
    active: raw.active,
    flags: flags(raw.flags, `${path}.flags`),
  };
}

function idList(value: unknown, path: string): number[] {
  return array(value, path).map((id, i) => integer(id, `${path}[${i}]`, U32));
}

function diagnostics(value: unknown): FlowDiagnostics {
  const raw = object(value, "diagnostics");
  return {
    duplicateIndividualAddresses: array(raw.duplicateIndividualAddresses, "diagnostics.duplicateIndividualAddresses").map((entry, i) => {
      const e = object(entry, `duplicateIndividualAddresses[${i}]`);
      return { individualAddressRaw: integer(e.individualAddressRaw, "individualAddressRaw", U16), deviceIds: idList(e.deviceIds, "deviceIds") };
    }),
    ambiguousGroupAddresses: array(raw.ambiguousGroupAddresses, "diagnostics.ambiguousGroupAddresses").map((entry, i) => {
      const e = object(entry, `ambiguousGroupAddresses[${i}]`);
      return { gaRaw: integer(e.gaRaw, "gaRaw", U16), gaIds: idList(e.gaIds, "gaIds") };
    }),
    ambiguousDevices: idList(raw.ambiguousDevices, "diagnostics.ambiguousDevices"),
    danglingLinks: array(raw.danglingLinks, "diagnostics.danglingLinks").map((entry, i) => {
      const e = object(entry, `danglingLinks[${i}]`);
      return { comObjectId: integer(e.comObjectId, "comObjectId", U32), gaId: integer(e.gaId, "gaId", U32) };
    }),
    unknownDevices: array(raw.unknownDevices, "diagnostics.unknownDevices").map((entry, i) => {
      const e = object(entry, `unknownDevices[${i}]`);
      return { comObjectId: integer(e.comObjectId, "comObjectId", U32), deviceId: integer(e.deviceId, "deviceId", U32) };
    }),
    objectsWithoutFlags: idList(raw.objectsWithoutFlags, "diagnostics.objectsWithoutFlags"),
  };
}

/** Validates one `GET /api/bus/monitor/flow-snapshot` body; throws on any deviation. */
export function parseFlowSnapshot(value: unknown): FlowSnapshot {
  const raw = object(value, "body");
  const generation = text(raw.generation, "generation");
  if (!GENERATION.test(generation)) refuse("generation", "is not a canonical decimal");
  const truncated = object(raw.truncated, "truncated");
  return {
    serverIncarnation: text(raw.serverIncarnation, "serverIncarnation"),
    sessionId: integer(raw.sessionId, "sessionId", Number.MAX_SAFE_INTEGER),
    generation,
    status: oneOf(raw.status, "status", ["current", "historical", "unavailable"] as const),
    groupAddressStyle:
      raw.groupAddressStyle === null ? null : oneOf(raw.groupAddressStyle, "groupAddressStyle", ["Free", "TwoLevel", "ThreeLevel"] as const),
    devices: array(raw.devices, "devices").map((entry, i) => {
      const e = object(entry, `devices[${i}]`);
      return {
        deviceId: integer(e.deviceId, `devices[${i}].deviceId`, U32),
        installationId: nullableInteger(e.installationId, `devices[${i}].installationId`, U8),
        name: text(e.name, `devices[${i}].name`),
        individualAddressRaw: nullableInteger(e.individualAddressRaw, `devices[${i}].individualAddressRaw`, U16),
      };
    }),
    groups: array(raw.groups, "groups").map((entry, i) => {
      const e = object(entry, `groups[${i}]`);
      return {
        gaRaw: integer(e.gaRaw, `groups[${i}].gaRaw`, U16),
        gaId: integer(e.gaId, `groups[${i}].gaId`, U32),
        installationId: integer(e.installationId, `groups[${i}].installationId`, U8),
        name: text(e.name, `groups[${i}].name`),
        dpt: nullableText(e.dpt, `groups[${i}].dpt`),
        members: array(e.members, `groups[${i}].members`).map((m, j) => member(m, `groups[${i}].members[${j}]`)),
      };
    }),
    diagnostics: diagnostics(raw.diagnostics),
    truncated: {
      devices: integer(truncated.devices, "truncated.devices", Number.MAX_SAFE_INTEGER),
      groups: integer(truncated.groups, "truncated.groups", Number.MAX_SAFE_INTEGER),
      members: integer(truncated.members, "truncated.members", Number.MAX_SAFE_INTEGER),
      diagnostics: integer(truncated.diagnostics, "truncated.diagnostics", Number.MAX_SAFE_INTEGER),
    },
  };
}

/** What a monitor row says about the flow, read from its AR20 fields only. */
export type RowFlowFacts =
  | { kind: "facts"; sourceRaw: number; destinationRaw: number; observedAgeMs: number | null; flowGeneration: string }
  /** `SessionClosed`: no source, destination or generation; not traffic. */
  | { kind: "marker" }
  /** A server before AR20: no raw fields at all. */
  | { kind: "legacy" }
  /** Present but not what the contract states; never drawn. */
  | { kind: "malformed" };

export interface RowFlowFields {
  sourceRaw?: unknown;
  destinationRaw?: unknown;
  observedAgeMs?: unknown;
  flowGeneration?: unknown;
}

const isRaw16 = (value: unknown): value is number =>
  typeof value === "number" && Number.isInteger(value) && value >= 0 && value <= U16;

export function rowFlowFacts(row: RowFlowFields): RowFlowFacts {
  const { sourceRaw, destinationRaw, observedAgeMs, flowGeneration } = row;
  if (sourceRaw === undefined && destinationRaw === undefined && flowGeneration === undefined) return { kind: "legacy" };
  const age = observedAgeMs ?? null;
  if (age !== null && (typeof age !== "number" || !Number.isSafeInteger(age) || age < 0)) return { kind: "malformed" };
  if (sourceRaw === null && destinationRaw === null && (flowGeneration ?? null) === null) return { kind: "marker" };
  if (!isRaw16(sourceRaw) || !isRaw16(destinationRaw)) return { kind: "malformed" };
  if (typeof flowGeneration !== "string" || !GENERATION.test(flowGeneration)) return { kind: "malformed" };
  return { kind: "facts", sourceRaw, destinationRaw, observedAgeMs: age, flowGeneration };
}
