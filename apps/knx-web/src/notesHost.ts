/** Notes host: snapshot identity and labelled target choices for the notes editor. */
// The notes editor owns its own preview/apply lifetime; this module only
// derives what the host must hand it: the snapshot identity from the
// authoritative ProjectTree (explicit snake_case → camelCase, never an
// invented counter) and labelled choices from the service's target catalogue
// (`GET /api/project/notes/targets`, `knx_app::note_targets`). Which entities
// may carry a note is the service's decision; this module adds only the
// translated kind word and keeps equal labels apart.
import type { ProjectTree } from "./bindings/ProjectTree";
import type { NotesContext, NoteTarget, NoteTargetOption, NoteTargetsView } from "./features/notes/client";
import { noteTargetKey } from "./features/notes/client";
import type { NoteTargetChoice } from "./features/notes/ProjectNotesEditor";
import { snapshotContextFromTree } from "./snapshotIdentity";

type EntityKind = Exclude<NoteTarget["kind"], "project">;
export type NoteTargetKindLabels = Readonly<Record<EntityKind, string>>;

/** `null` unless the tree carries a complete server/project/revision identity. */
export function notesContextFromTree(tree: ProjectTree): NotesContext | null {
  return snapshotContextFromTree(tree);
}

/**
 * The catalogue's targets when it belongs to the same server and project as
 * `context`, otherwise `null`. A different snapshot revision is accepted: the
 * service validates every target again on preview, so a catalogue that is a
 * revision behind can at worst offer a target that preview then refuses.
 */
export function catalogueForContext(view: NoteTargetsView, context: NotesContext): readonly NoteTargetOption[] | null {
  return view.serverIncarnation === context.serverIncarnation && view.projectIncarnation === context.projectIncarnation
    ? view.targets
    : null;
}

function describe(option: NoteTargetOption, labels: NoteTargetKindLabels): string {
  const own = option.parts.join(" ") || `#${option.target.id}`;
  const owner = option.owner.join(" ");
  return `${labels[option.target.kind]}: ${own}${owner ? ` · ${owner}` : ""}`;
}

/**
 * The catalogue's targets in its own order, labelled for the UI. The project
 * target is the editor's own choice and never taken from here. An ID listed
 * twice with different labels is withheld rather than guessed; distinct
 * entities with equal labels get their stable ID appended.
 */
export function noteTargetChoices(options: readonly NoteTargetOption[], labels: NoteTargetKindLabels): NoteTargetChoice[] {
  const seen = new Map<string, NoteTargetChoice>();
  const ambiguous = new Set<string>();
  for (const option of options) {
    if ((option.target as NoteTarget).kind === "project") continue;
    const key = noteTargetKey(option.target);
    const label = describe(option, labels);
    const earlier = seen.get(key);
    if (!earlier) seen.set(key, { target: option.target, label });
    else if (earlier.label !== label) ambiguous.add(key);
  }
  const unique = [...seen].filter(([key]) => !ambiguous.has(key)).map(([, choice]) => choice);
  const counts = new Map<string, number>();
  for (const choice of unique) counts.set(choice.label, (counts.get(choice.label) ?? 0) + 1);
  return unique.map((choice) => (counts.get(choice.label) ?? 0) > 1 && choice.target.kind !== "project"
    ? { ...choice, label: `${choice.label} · #${choice.target.id}` }
    : choice);
}
