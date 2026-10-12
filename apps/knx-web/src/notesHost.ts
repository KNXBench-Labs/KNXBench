/** Notes host: snapshot identity and stable-ID target choices for the notes editor. */
// The notes editor owns its own preview/apply lifetime; this module only
// derives what the host must hand it from the authoritative ProjectTree:
// the snapshot identity (explicit snake_case → camelCase, never an invented
// counter) and the entities a note may be attached to, keyed by domain ID.
import type { ProjectTree } from "./bindings/ProjectTree";
import type { BuildingNode } from "./bindings/BuildingNode";
import type { DeviceNode } from "./bindings/DeviceNode";
import type { NotesContext, NoteTarget } from "./features/notes/client";
import { noteTargetKey } from "./features/notes/client";
import type { NoteTargetChoice } from "./features/notes/ProjectNotesEditor";
import { hasRenameContext } from "./rename";

type EntityKind = Exclude<NoteTarget["kind"], "project" | "comObject">;
export type NoteTargetKindLabels = Readonly<Record<EntityKind, string>>;

/** Choice order: the containers a reader meets first, then the bus objects. */
const KIND_ORDER: readonly EntityKind[] = [
  "installation", "area", "line", "buildingPart", "device", "groupRange", "groupAddress",
];

/** The note contract addresses installations by an 8-bit domain ID. */
const MAX_INSTALLATION_ID = 255;

/** `null` unless the tree carries a complete server/project/revision identity. */
export function notesContextFromTree(tree: ProjectTree): NotesContext | null {
  if (!hasRenameContext(tree)) return null;
  return {
    serverIncarnation: tree.server_incarnation!,
    projectIncarnation: tree.project_incarnation!,
    snapshotRevision: tree.snapshot_revision!,
  };
}

interface Candidate { target: NoteTarget; text: string }

function describe(prefix: string, id: number, parts: readonly (string | null | undefined)[]): string {
  const text = parts.filter((part): part is string => !!part && part.trim().length > 0).join(" ");
  return `${prefix}: ${text || `#${id}`}`;
}

function buildingParts(roots: readonly BuildingNode[]): BuildingNode[] {
  // Iterative pre-order walk: deep imported hierarchies must not overflow.
  const ordered: BuildingNode[] = [];
  const stack = [...roots].reverse();
  while (stack.length > 0) {
    const part = stack.pop()!;
    ordered.push(part);
    for (let index = part.children.length - 1; index >= 0; index--) stack.push(part.children[index]);
  }
  return ordered;
}

/**
 * Every projected entity a note may target, once per stable ID, in project
 * order grouped by kind. An ID the projection shows with two different
 * identities is ambiguous and therefore not offered; identical labels of
 * distinct entities get their ID appended instead of one hiding the other.
 * Communication objects are not fully projected in the tree and are not
 * offered; a stored object target stays visibly identified by the editor.
 */
export function noteTargetChoices(tree: ProjectTree, labels: NoteTargetKindLabels): NoteTargetChoice[] {
  const byKind = new Map<EntityKind, Candidate[]>(KIND_ORDER.map((kind) => [kind, []]));
  const add = (kind: EntityKind, id: number, parts: readonly (string | null | undefined)[]) => {
    byKind.get(kind)!.push({ target: { kind, id }, text: describe(labels[kind], id, parts) });
  };
  const device = (node: DeviceNode) => add("device", node.id, [node.address, node.name]);
  for (const installation of tree.installations) {
    if (Number.isInteger(installation.id) && installation.id >= 0 && installation.id <= MAX_INSTALLATION_ID) {
      add("installation", installation.id, [installation.name]);
    }
    for (const area of installation.topology) {
      add("area", area.id, [String(area.address), area.name]);
      for (const line of area.lines) add("line", line.id, [`${area.address}.${line.address}`, line.name]);
    }
    const parts = buildingParts(installation.buildings);
    for (const part of parts) add("buildingPart", part.id, [part.name]);
    for (const area of installation.topology) for (const line of area.lines) line.devices.forEach(device);
    for (const part of parts) part.devices.forEach(device);
    installation.unassigned.forEach(device);
    for (const range of installation.group_ranges) add("groupRange", range.id, [`${range.start}–${range.end}`, range.name]);
    for (const address of installation.group_addresses) add("groupAddress", address.id, [address.address, address.name]);
  }

  const unique: Candidate[] = [];
  for (const candidates of byKind.values()) {
    const seen = new Map<string, Candidate>();
    const ambiguous = new Set<string>();
    for (const candidate of candidates) {
      const key = noteTargetKey(candidate.target);
      const earlier = seen.get(key);
      if (!earlier) seen.set(key, candidate);
      else if (earlier.text !== candidate.text) ambiguous.add(key);
    }
    for (const [key, candidate] of seen) if (!ambiguous.has(key)) unique.push(candidate);
  }

  const counts = new Map<string, number>();
  for (const candidate of unique) counts.set(candidate.text, (counts.get(candidate.text) ?? 0) + 1);
  return unique.map(({ target, text }) => ({
    target,
    label: (counts.get(text) ?? 0) > 1 && target.kind !== "project" ? `${text} · #${target.id}` : text,
  }));
}
