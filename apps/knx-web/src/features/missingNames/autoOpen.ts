/** WP-51.3b: pure rule for opening the missing-name wizard once per loaded project. */
import type { MissingNameScan } from "./client";

export type AutoOpenTree = Readonly<{ server_incarnation?: string; project_incarnation?: number }>;

/** Identifies one loaded project of one server run; a new project or restart yields a new key. */
export const autoOpenKey = (tree: AutoOpenTree): string =>
  `${tree.server_incarnation ?? ""}|${tree.project_incarnation ?? ""}`;

/** The single predicate: may the wizard open by itself for this key right now? */
export function shouldAutoOpen(state: Readonly<{ key: string; openedFor: string | null; overlayOpen: boolean;
  saving: boolean; readOnly?: boolean }>): boolean {
  // readOnly is reserved for Q17: the UI has no read-only signal today, so callers leave it false.
  if (state.readOnly) return false;
  if (state.overlayOpen || state.saving) return false;
  return state.openedFor !== state.key;
}

/** Remembers the key of the last automatic open; manual opens never call markOpened. */
export function createAutoOpenTracker() {
  let opened: string | null = null;
  return { openedFor: (): string | null => opened, markOpened(key: string): void { opened = key; } };
}

/**
 * True when the scan page holds at least one proposal. Uses `items` (typed, admitted page content)
 * rather than the scan-wide count, so an empty page never opens an empty wizard.
 */
export const hasEligibleCandidates = (scan: Pick<MissingNameScan, "items">): boolean => scan.items.length > 0;
