# ADR-0098: Offline AP1 sequence resolution is not download capability

- Date: 2026-10-09
- Status: accepted; implemented and locally verified, unpublished
- Scope: explicit `MV-07B0` `Load/ap1` declarative sequence inspection and maintainer evidence

## Context

The existing memory planner intentionally refuses non-memory-mapped masks. Expanding a mask template does not supply a parameter image, table substitutions, management executor, recovery coverage or hardware evidence. The user approved offline resolution and maintainer reporting, explicitly without new live eligibility.

Primary-source and bounded-corpus research plus the full behavioral contract are in [Offline procedure resolution](../OFFLINE_PROCEDURE_RESOLUTION.md). KNX Configuration Procedures §3.9.3 provides the System-B merge seam and AP1/AP2 split. Manufacturer-side XML metadata outside those documented rules remains observation, not execution semantics.

## Decision

Add a bounded read-only source resolver in `knx-productdb`, separate from `code`/`download_plan`. Read retained application/master bytes from their ownership in the exact analyzed package. Never select a global newest master or silently resolve ambiguity. Exact namespaces, canonical paths and unqualified identity attributes govern selection. Source hashes/positions identify the originals.

Explicit AP1 expansion preserves source order; optional absent fragments are accounted for, mandatory/duplicate/unused/nested/unknown constructs remain reported and retained. The result is a diagnostic type, never `MemoryDownloadPlan` or any protocol plan. `expanded` means declarative reconstruction only; `executable` is always false. The existing readiness taxonomy and executor entry points are unchanged.

Expose local source-bearing results additively through existing contribution analysis UI/API/CLI. Reduced reports clear these objects and retain value-free findings/counts; known-secret-bearing inputs withhold them locally too. Existing exact preview/consent/ZIP/manual issue handoff owns disclosure. No new uploader, telemetry or background scan.

No schema migration, new protocol capability, importer admission or input-limit increase. Serialize the diagnostic types using the existing workspace serde dependency; no new dependency version/framework. Output work is bounded independently of input admission and partial coverage remains explicit.

## Alternatives

- Extending the live memory planner: rejected, because the resolved sequence does not meet its image/execution/recovery contract.
- Reading an arbitrary installed master: rejected, because it breaks package source binding and hides conflicts.
- Recursively evaluating conditional/nested fragments: deferred until their selection semantics are independently documented; naming them is safer than guessing.
- New persistence tables: unnecessary for this single-input analysis; retained blobs remain authoritative.
- A second reporting service: unnecessary; existing disclosure/handoff already provides the safe boundary.

## Consequences

Users can inspect additional declarative sequences and share discoveries without confusing sequence expansion with a runnable plan. Missing evidence still limits resolution and optional context may be necessary for maintainer reproduction. Legacy, other masks, complete/AP2/partial variants, symbol/materialization and new live paths remain out of scope. The acceptance contract explicitly exercises malformed sources, limits, privacy and unchanged live refusal.
