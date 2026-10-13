/** Strict client for the project Functions read and revision-bound edit routes (A4). */
// Contract: docs/contracts/project-functions.md. The read answers the
// Functions plus the exact project snapshot they were read under; every
// edit carries that snapshot back. Any unexpected shape is refused whole,
// so the UI never renders or edits half-understood data.
import { noteRefusal } from "../../api";
import type { FunctionNode } from "../../bindings/FunctionNode";
import type { FunctionMemberNode } from "../../bindings/FunctionMemberNode";

export type FunctionsContext = Readonly<{ serverIncarnation: string; projectIncarnation: number; snapshotRevision: number }>;
export type FunctionsRead = FunctionsContext & Readonly<{ functions: readonly FunctionNode[] }>;
export type FunctionsEdited = FunctionsRead & Readonly<{ createdId: number | null }>;

/** ETS `CompletionStatus` spellings the server accepts and reports. */
export const COMPLETIONS = ["Undefined", "Editing", "FinishedDesign", "FinishedCommissioning", "Tested", "Locked", "Accepted"] as const;
export type Completion = (typeof COMPLETIONS)[number];

/** Replaces all details at once (`update`), exactly like the server command. */
export type FunctionDetailsDraft = Readonly<{
  name: string; functionType: string | null; number: string | null; comment: string | null;
  description: string | null; completion: Completion;
}>;

export type FunctionAction =
  | Readonly<{ kind: "create"; location: number; details: FunctionDetailsDraft }>
  | Readonly<{ kind: "update"; id: number; details: FunctionDetailsDraft }>
  | Readonly<{ kind: "delete"; id: number }>
  | Readonly<{ kind: "move"; id: number; location: number }>
  | Readonly<{ kind: "addMember"; functionId: number; groupAddressId: number; name: string; role: string | null }>
  | Readonly<{ kind: "removeMember"; functionId: number; memberId: number }>;

export type FunctionsClient = Readonly<{
  read(signal?: AbortSignal): Promise<FunctionsRead>;
  edit(context: FunctionsContext, action: FunctionAction, signal?: AbortSignal): Promise<FunctionsEdited>;
}>;

export class FunctionsContractError extends Error {
  constructor() { super("Unsupported or malformed project Functions response"); this.name = "FunctionsContractError"; }
}

/** A refused request: `functionStale` (409), `functionRefused` (422) or another server answer. */
export type FunctionsRefusal = Error & { status: number; kind: string | null };

const UINT32_MAX = 0xffff_ffff;
const MAX_FUNCTIONS = 65_536;
const MAX_MEMBERS = 65_536;
/** Imported text is verbatim and may be long; only an absurd size is refused. */
const MAX_TEXT_BYTES = 64 * 1024;
const encoder = new TextEncoder();

function record(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}
function exactly(value: Record<string, unknown>, expected: readonly string[]): boolean {
  return Object.keys(value).length === expected.length && expected.every((key) => Object.hasOwn(value, key));
}
function id(value: unknown, max = UINT32_MAX): value is number {
  return typeof value === "number" && Number.isSafeInteger(value) && value >= 0 && value <= max;
}
function text(value: unknown): value is string {
  return typeof value === "string" && encoder.encode(value).length <= MAX_TEXT_BYTES;
}
function optionalText(value: unknown): value is string | null {
  return value === null || text(value);
}
function context(value: Record<string, unknown>): boolean {
  return typeof value.serverIncarnation === "string" && value.serverIncarnation.length > 0
    && encoder.encode(value.serverIncarnation).length <= 128 && id(value.projectIncarnation, Number.MAX_SAFE_INTEGER)
    && id(value.snapshotRevision, Number.MAX_SAFE_INTEGER);
}

function member(value: unknown): value is FunctionMemberNode {
  return record(value)
    && exactly(value, ["id", "sourceId", "groupAddressId", "address", "groupAddressName", "name", "role"])
    && id(value.id) && value.id > 0 && text(value.sourceId) && id(value.groupAddressId) && text(value.address)
    && text(value.groupAddressName) && text(value.name) && optionalText(value.role);
}

function functionNode(value: unknown): value is FunctionNode {
  if (!record(value) || !exactly(value, ["id", "sourceId", "installationId", "location", "name", "functionType",
    "number", "comment", "description", "completion", "members"])) return false;
  const location = value.location;
  return id(value.id) && value.id > 0 && text(value.sourceId) && id(value.installationId, 255)
    && record(location) && exactly(location, ["buildingPartId", "name", "kind"]) && id(location.buildingPartId)
    && text(location.name) && text(location.kind) && text(value.name) && optionalText(value.functionType)
    && optionalText(value.number) && optionalText(value.comment) && optionalText(value.description)
    && typeof value.completion === "string" && (COMPLETIONS as readonly string[]).includes(value.completion)
    && Array.isArray(value.members) && value.members.length <= MAX_MEMBERS && value.members.every(member);
}

/** Function and member ids are project-unique; a repeated one is a broken answer. */
function admitFunctions(value: unknown): readonly FunctionNode[] {
  if (!Array.isArray(value) || value.length > MAX_FUNCTIONS) throw new FunctionsContractError();
  const functions = new Set<number>();
  const members = new Set<number>();
  for (const item of value) {
    if (!functionNode(item) || functions.has(item.id)) throw new FunctionsContractError();
    functions.add(item.id);
    for (const entry of item.members) {
      if (members.has(entry.id)) throw new FunctionsContractError();
      members.add(entry.id);
    }
  }
  return value as readonly FunctionNode[];
}

export function admitFunctionsRead(value: unknown): FunctionsRead {
  if (!record(value) || !exactly(value, ["functions", "serverIncarnation", "projectIncarnation", "snapshotRevision"])
    || !context(value)) throw new FunctionsContractError();
  return { serverIncarnation: value.serverIncarnation as string, projectIncarnation: value.projectIncarnation as number,
    snapshotRevision: value.snapshotRevision as number, functions: admitFunctions(value.functions) };
}

export function admitFunctionsEdited(value: unknown): FunctionsEdited {
  if (!record(value) || !exactly(value, ["createdId", "functions", "serverIncarnation", "projectIncarnation", "snapshotRevision"])
    || !(value.createdId === null || (id(value.createdId) && value.createdId > 0))) throw new FunctionsContractError();
  const { createdId, ...rest } = value;
  return { ...admitFunctionsRead(rest), createdId: createdId as number | null };
}

/** Just the binding of a read, without the Functions themselves. */
export function contextOf(read: FunctionsContext): FunctionsContext {
  return { serverIncarnation: read.serverIncarnation, projectIncarnation: read.projectIncarnation, snapshotRevision: read.snapshotRevision };
}

export function sameFunctionsContext(a: FunctionsContext, b: FunctionsContext): boolean {
  return a.serverIncarnation === b.serverIncarnation && a.projectIncarnation === b.projectIncarnation
    && a.snapshotRevision === b.snapshotRevision;
}

/** `409 functionStale`: nothing was applied because the project moved since the read. */
export function isFunctionsStale(error: unknown): boolean {
  return record(error) && (error as { status?: unknown }).status === 409 && (error as { kind?: unknown }).kind === "functionStale";
}

/** `422 functionRefused`: the command refused the edit; its message is the server's own text. */
export function isFunctionsRefused(error: unknown): boolean {
  return record(error) && (error as { status?: unknown }).status === 422 && (error as { kind?: unknown }).kind === "functionRefused";
}

async function requestJson(transport: typeof fetch, path: string, payload: unknown, signal?: AbortSignal): Promise<unknown> {
  const response = await transport(path, {
    method: payload === undefined ? "GET" : "POST", credentials: "same-origin", signal,
    ...(payload === undefined ? {} : { headers: { "Content-Type": "application/json" }, body: JSON.stringify(payload) }),
  });
  if (!response.ok) {
    noteRefusal(path, response.status);
    const body = await response.json().catch(() => null) as unknown;
    const message = record(body) && typeof body.error === "string" ? body.error : `Project Functions request failed (${response.status})`;
    throw Object.assign(new Error(message), {
      status: response.status, kind: record(body) && typeof body.kind === "string" ? body.kind : null,
    }) as FunctionsRefusal;
  }
  return response.json() as Promise<unknown>;
}

export function createFunctionsClient(transport: typeof fetch = (...args) => fetch(...args)): FunctionsClient {
  return {
    read: async (signal) => admitFunctionsRead(await requestJson(transport, "/api/project/functions", undefined, signal)),
    edit: async (expected, action, signal) => admitFunctionsEdited(await requestJson(transport, "/api/project/functions/edit", {
      serverIncarnation: expected.serverIncarnation, projectIncarnation: expected.projectIncarnation,
      snapshotRevision: expected.snapshotRevision, action,
    }, signal)),
  };
}
