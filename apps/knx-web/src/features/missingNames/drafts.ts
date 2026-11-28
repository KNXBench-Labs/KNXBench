/** Pure draft state of the missing-name wizard: selection, names, duplicates and invalidation. */
/*
 * A draft is keyed by the stable `NameTargetId` and lives across pages of
 * one scan binding. Selection never auto-splits or trims to the cap: the
 * view refuses to submit more than `MAX_MISSING_NAME_SELECTION` (Q17).
 * Duplicate names are only warned about, never suffixed (Q13). A parent
 * change re-proposes unedited children and keeps manual edits (Q16); a
 * child on a page loaded only later starts from its re-proposal.
 */
import {
  codePoints, isBlankName, NAME_TARGET_KINDS, targetKey, type MissingNameEntry, type MissingNameProposal,
  type MissingNameReevaluation, type NameTargetId, type NameTargetKind,
} from "./client";

export type Recheck = "updated" | "kept";
export type Draft = Readonly<{
  proposal: MissingNameProposal;
  name: string;
  /** The user typed into this entry; a parent change never overwrites it (Q16). */
  edited: boolean;
  selected: boolean;
  /** Set when a parent change re-proposed (`updated`) or kept a manual edit (`kept`). */
  recheck: Recheck | null;
}>;
export type Drafts = ReadonlyMap<string, Draft>;
export type NameProblem = "blank" | "tooLong";
/** Re-evaluated proposals by target key, kept for children whose page is not loaded yet. */
export type Reproposals = ReadonlyMap<string, MissingNameProposal>;

/** Kinds whose name feeds child suggestions (`build_dependency_graph`); only they trigger reevaluation. */
export const PARENT_KINDS: ReadonlySet<NameTargetKind> = new Set(["project", "installation", "building_part", "area",
  "line", "group_range"]);

export const isParentKind = (target: NameTargetId) => PARENT_KINDS.has(target.kind);

/** Only a uniquely identified target can be renamed; duplicates and zero IDs stay inspect-only. */
export function isSelectable(proposal: MissingNameProposal): boolean {
  return proposal.candidate.identity_status.status === "unique";
}

/**
 * Adds drafts for newly seen page items and keeps every existing draft untouched.
 * A new item with a re-proposal starts from it, not from the scan's outdated suggestion.
 */
export function mergePage(drafts: Drafts, items: readonly MissingNameProposal[],
  reproposals: Reproposals = new Map()): Drafts {
  const next = new Map(drafts);
  for (const item of items) {
    const key = targetKey(item.candidate.target);
    if (next.has(key)) continue;
    const reproposed = reproposals.get(key);
    const proposal = reproposed ?? item;
    next.set(key, { proposal, name: proposal.proposed_name ?? "", edited: false, selected: false,
      recheck: reproposed ? "updated" : null });
  }
  return next;
}

export function nameProblem(draft: Draft): NameProblem | null {
  if (isBlankName(draft.name)) return "blank";
  if (codePoints(draft.name) > draft.proposal.candidate.max_name_codepoints) return "tooLong";
  return null;
}

export function setName(drafts: Drafts, key: string, name: string): Drafts {
  const draft = drafts.get(key);
  if (!draft || draft.name === name) return drafts;
  return new Map(drafts).set(key, { ...draft, name, edited: true, recheck: null });
}

export function setSelected(drafts: Drafts, keys: readonly string[], selected: boolean): Drafts {
  const next = new Map(drafts);
  for (const key of keys) {
    const draft = next.get(key);
    if (draft && isSelectable(draft.proposal) && draft.selected !== selected) next.set(key, { ...draft, selected });
  }
  return next;
}

export const selectedDrafts = (drafts: Drafts): Draft[] => [...drafts.values()].filter((draft) => draft.selected);

/** The batch as the user chose it; the server orders parents before children. */
export function selectedEntries(drafts: Drafts): MissingNameEntry[] {
  return selectedDrafts(drafts).map((draft) => ({ target: draft.proposal.candidate.target, name: draft.name }));
}

/** Selected parents with an admissible name: the draft context the server overlays for reevaluation. */
export function draftParents(drafts: Drafts): MissingNameEntry[] {
  return selectedDrafts(drafts)
    .filter((draft) => isParentKind(draft.proposal.candidate.target) && nameProblem(draft) === null)
    .map((draft) => ({ target: draft.proposal.candidate.target, name: draft.name }));
}

/**
 * Keys that deserve a duplicate warning: the server's page-local flag while the
 * suggestion is unchanged, or the same name on two selected targets of one kind.
 * The preview's scoped warnings stay authoritative; nothing is renamed or blocked.
 */
export function duplicateKeys(drafts: Drafts): ReadonlySet<string> {
  const keys = new Set<string>();
  const byName = new Map<string, string[]>();
  for (const [key, draft] of drafts) {
    if (draft.proposal.duplicate_label && draft.name === draft.proposal.proposed_name) keys.add(key);
    if (!draft.selected || isBlankName(draft.name)) continue;
    const bucket = `${draft.proposal.candidate.kind}\u0000${draft.name}`;
    byName.set(bucket, [...(byName.get(bucket) ?? []), key]);
  }
  for (const group of byName.values()) if (group.length > 1) group.forEach((key) => keys.add(key));
  return keys;
}

/** Applies a Q16 reevaluation: unedited children take the new suggestion, manual edits stay. */
export function applyReevaluation(drafts: Drafts, result: MissingNameReevaluation): Drafts {
  const proposals = new Map(result.proposals.map((proposal) => [targetKey(proposal.candidate.target), proposal]));
  const next = new Map(drafts);
  for (const target of result.invalidated_targets) {
    const key = targetKey(target);
    const draft = next.get(key);
    const proposal = proposals.get(key);
    if (!draft || !proposal) continue;
    next.set(key, draft.edited
      ? { ...draft, proposal, recheck: "kept" }
      : { ...draft, proposal, name: proposal.proposed_name ?? "", recheck: "updated" });
  }
  return next;
}

/** Remembers a reevaluation's proposals for later pages; the latest reevaluation of a target wins. */
export function keepReproposals(reproposals: Reproposals, result: MissingNameReevaluation): Reproposals {
  const next = new Map(reproposals);
  for (const target of result.invalidated_targets) next.delete(targetKey(target));
  for (const proposal of result.proposals) next.set(targetKey(proposal.candidate.target), proposal);
  return next;
}

export type KindGroup = Readonly<{ kind: NameTargetKind; items: readonly MissingNameProposal[] }>;

/** Page items grouped by object type in the fixed project order (Q7). */
export function groupByKind(items: readonly MissingNameProposal[]): KindGroup[] {
  return NAME_TARGET_KINDS
    .map((kind) => ({ kind, items: items.filter((item) => item.candidate.kind === kind) }))
    .filter((group) => group.items.length > 0);
}

/** Row key: the target plus its occurrence, so duplicate-ID occurrences render separately. */
export function occurrenceKey(proposal: MissingNameProposal): string {
  return `${targetKey(proposal.candidate.target)}#${proposal.candidate.occurrence_index}`;
}
