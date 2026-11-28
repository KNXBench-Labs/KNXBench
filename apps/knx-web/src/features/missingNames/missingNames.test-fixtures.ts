/** Typed missing-name proposal and scan fixtures for the wizard unit and component tests. */
import type { MissingNameProposal, MissingNameScan, NameTargetId, NameTargetKind } from "./client";

const EMPTY_CONTEXT = {
  installation_id: null, source_ets_id: null, building_part_kind: null, building_part_number: null,
  individual_address: null, topology_area_address: null, topology_line_address: null, group_address: null,
  group_range_span: null,
} as const;

export function target(kind: NameTargetKind, id: number): NameTargetId {
  return kind === "project" ? { kind: "project" } : { kind, id };
}

export function proposal(kind: NameTargetKind, id: number, name: string | null,
  overrides: Partial<{ duplicate_label: boolean; identity: MissingNameProposal["candidate"]["identity_status"];
    occurrence: number; max: number; origin: MissingNameProposal["provenance"]["origin"];
    gaps: MissingNameProposal["provenance"]["gaps"]; context: Partial<MissingNameProposal["candidate"]["context"]> }> = {},
): MissingNameProposal {
  return {
    candidate: {
      target: target(kind, id), kind, current_name: "", occurrence_index: overrides.occurrence ?? 0,
      identity_status: overrides.identity ?? { status: "unique" }, parents: [],
      max_name_codepoints: overrides.max ?? 255, context: { ...EMPTY_CONTEXT, ...overrides.context },
    },
    proposed_name: name,
    provenance: { origin: overrides.origin ?? (name === null ? "manual_required" : "automatic"),
      sources: name === null ? [] : [{ kind: "ets_id", source_target: null, value: "x" }],
      gaps: overrides.gaps ?? [], derived_from_automatic: false },
    duplicate_label: overrides.duplicate_label ?? false,
  } satisfies MissingNameProposal;
}

/** The server's wire shape of a scan response (the client re-nests the binding). */
export function scanBody(items: readonly MissingNameProposal[], options: Partial<{ complete: boolean; offset: number;
  found: number; hasMore: boolean; persistence: string; revision: number; readOnly: boolean }> = {}) {
  const complete = options.complete ?? true;
  const found = options.found ?? items.length;
  return {
    serverIncarnation: "srv", projectIncarnation: 3, snapshotRevision: options.revision ?? 7,
    persistence: options.persistence ?? "native-working-history", readOnly: options.readOnly ?? false,
    discovery: {
      completeness: complete ? "complete" : "incomplete",
      counts: { entities_scanned: 40, candidates_found_in_scan: found, total_candidates: complete ? found : null,
        total_by_kind: complete ? {} : null },
      offset: options.offset ?? 0, limit: 100, has_more_in_scan: options.hasMore ?? false,
    },
    items,
  };
}

export type ScanWire = ReturnType<typeof scanBody>;
export type { MissingNameScan };
