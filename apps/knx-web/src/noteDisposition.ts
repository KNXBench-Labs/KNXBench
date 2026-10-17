/** Admits the note-disposition refusal and builds explicit choices (ADR-0138). */
// A delete that would strand project notes answers `409 noteDispositionRequired`
// with every affected note, the targets the delete removes and the snapshot the
// answer describes. This module admits that body strictly (a malformed one is
// treated as an ordinary error, never half-shown) and turns the user's choices
// into the wire form of `POST /api/project/delete-with-notes`. Which entity is
// a valid move target stays the server's decision: it re-checks every choice.
import type { NoteTarget } from "./features/notes/client";
import { noteTargetKey } from "./features/notes/client";
import type { NoteTargetChoice } from "./features/notes/ProjectNotesEditor";

/** The deletes the web app offers, in the apply route's wire form. */
export type Deletion =
  | { kind: "devices"; ids: number[] }
  | { kind: "groupAddresses"; ids: number[] }
  | { kind: "area" | "line" | "buildingPart" | "groupRange"; id: number };

export interface AffectedNote { id: number; title: string; target: NoteTarget }

export interface DispositionContext { serverIncarnation: string; projectIncarnation: number; snapshotRevision: number }

export interface DispositionRequired {
  /** The server's named reason in English, for the log and as a fallback. */
  message: string;
  notes: AffectedNote[];
  removedTargets: NoteTarget[];
  context: DispositionContext;
}

export type NoteChoice = { kind: "project" } | { kind: "move"; target: NoteTarget | null } | { kind: "delete" };

export type WireChoice = {
  note: number;
  action: { kind: "project" } | { kind: "move"; target: NoteTarget } | { kind: "delete" };
};

const ENTITY_KINDS = ["installation", "area", "line", "buildingPart", "device", "groupRange", "groupAddress", "comObject"];
const UINT32_MAX = 0xffff_ffff;

function record(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}
function id(value: unknown, max = UINT32_MAX): value is number {
  return typeof value === "number" && Number.isSafeInteger(value) && value >= 0 && value <= max;
}
function target(value: unknown): value is NoteTarget {
  if (!record(value)) return false;
  if (value.kind === "project") return Object.keys(value).length === 1;
  return Object.keys(value).length === 2 && typeof value.kind === "string" && ENTITY_KINDS.includes(value.kind)
    && id(value.id, value.kind === "installation" ? 255 : UINT32_MAX);
}
function note(value: unknown): value is AffectedNote {
  return record(value) && id(value.id) && value.id > 0 && typeof value.title === "string" && target(value.target);
}
function errorParts(e: unknown): { status: unknown; body: Record<string, unknown> } | null {
  if (!record(e)) return null;
  const { status, body } = e as { status?: unknown; body?: unknown };
  return record(body) ? { status, body } : null;
}

/** The structured refusal of a delete that would strand notes, or `null`. */
export function dispositionRequired(e: unknown): DispositionRequired | null {
  const parts = errorParts(e);
  if (!parts || parts.status !== 409) return null;
  const { body } = parts;
  if (body.kind !== "noteDispositionRequired" || typeof body.error !== "string") return null;
  if (!Array.isArray(body.notes) || body.notes.length === 0 || !body.notes.every(note)) return null;
  if (new Set(body.notes.map((entry) => entry.id)).size !== body.notes.length) return null;
  if (!Array.isArray(body.removedTargets) || !body.removedTargets.every(target)) return null;
  const { serverIncarnation, projectIncarnation, snapshotRevision } = body;
  if (typeof serverIncarnation !== "string" || serverIncarnation.length === 0
    || !id(projectIncarnation, Number.MAX_SAFE_INTEGER) || !id(snapshotRevision, Number.MAX_SAFE_INTEGER)) return null;
  return {
    message: body.error,
    notes: body.notes,
    removedTargets: body.removedTargets,
    context: { serverIncarnation, projectIncarnation, snapshotRevision },
  };
}

/** Whether an apply failed because the project changed since the refusal. */
export function isDispositionStale(e: unknown): boolean {
  const parts = errorParts(e);
  return parts?.status === 409 && parts.body.kind === "noteDispositionStale";
}

/** Every note starts on the choice that loses nothing: keep it on the project. */
export function initialChoices(notes: readonly AffectedNote[]): Map<number, NoteChoice> {
  return new Map(notes.map((entry) => [entry.id, { kind: "project" } as NoteChoice]));
}

/** Catalogue entities a note may move to: none the delete removes, never the project. */
export function moveOptions(choices: readonly NoteTargetChoice[], removed: readonly NoteTarget[]): NoteTargetChoice[] {
  const gone = new Set(removed.map(noteTargetKey));
  return choices.filter((choice) => choice.target.kind !== "project" && !gone.has(noteTargetKey(choice.target)));
}

/** The apply payload in note order, or `null` while a note is undecided. */
export function wireChoices(notes: readonly AffectedNote[], choices: ReadonlyMap<number, NoteChoice>): WireChoice[] | null {
  const wire: WireChoice[] = [];
  for (const entry of notes) {
    const choice = choices.get(entry.id);
    if (!choice) return null;
    if (choice.kind === "move") {
      if (!choice.target) return null;
      wire.push({ note: entry.id, action: { kind: "move", target: choice.target } });
    } else {
      wire.push({ note: entry.id, action: { kind: choice.kind } });
    }
  }
  return wire;
}
