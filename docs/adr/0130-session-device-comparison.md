# ADR-0130: Device comparison is bound, decoded, session-only evidence

- **Status:** Accepted (owner UI-feedback P4 decision); both packages delivered on main; source/publication evidence below
- **Date:** 2026-10-15
- **Decision record:** [UI feedback P1–P4](../goals/ui-feedback-p1-p4.md#p4--project-file-versus-real-devices-point-3)

## Publication update (15 October 2026)

Both owner-approved packages are delivered. Parameter implementation519eff4b
and separate inherited-corpus fixa6cdfd9d are ancestors of verified main17cfd5b7;
full/corpus runtime acceptance remains pinned to actual tested mergefacdd62e.
[Publication record](../evidence/p4-device-comparison-parameters-2026-10-15.json).
No new release, hardware verification or full ETS compatibility is implied.

## Context and evidence

P3 already defines device programming content and relevant changes in
`knx_core::commissioning::pending::ProgrammingContent`. Loaded flags express a
project belief, not a comparison revision. The old `compare_with_plan` checked
only mask and manufacturer, so an unrelated application looked like byte drift.
The partial-download precondition already identifies an application by five
bytes, using the Application Program Object's `PID_PROGRAM_VERSION`.

Primary sources rechecked locally, not newly tested on hardware:

- *Management Procedures*, `03_05_02` v02.01.02, §3.31.2,
  printed p.137: the absolute task record contains manufacturer, application
  number and application version as a five-byte identity. This is the existing
  Core task-record codec, not a new protocol interpretation.
- *Standardized Identifier Tables*, `03_07_03` v01.04.01, property identifier
  table, p.12: PID 13 is the five-byte application version property. The
  existing `properties::PID_PROGRAM_VERSION` also cites Resources §4.19's
  Application Program Object table.
- Existing verified 0701 identity read and table byte order are recorded in
  `partial_memory_download.rs`, `group_tables.rs` and RESEARCH §19. The
  current work uses simulators and intercepted browser traffic only.

Licensed source excerpts remain private. No vendor contact, new on-wire
service, access key, object-index discovery assumption or hardware authority.

## Decision

1. Reuse the existing full/partial download plan and gateway admission guards.
   Comparison is a read-only management session without write authorisation.
   A confirmed UI queue sends one bound request at a time; a gateway-busy
   refusal ends admission, while a device error is recorded and the queue
   continues. Cancel finishes the current bounded read and starts no more.
2. Check the complete application identity before reading plan memory. A valid
   differing five-byte response is Wrong application. Missing, conflicting or
   malformed identity cannot establish that claim and is Not comparable.
   Never turn a short prefix into an application match or mismatch.
3. Core maintains transient, monotone per-device comparison revisions using
   P3 content semantics. Do/undo/redo and known affected bus-write settlement
   invalidate old evidence; labels, DPT choices and manual loaded-flag marks
   are not programmed content. No project or product-database migration.
4. The server binds preparation on both sides to server/project incarnation,
   stable device ID and comparison revision. It refuses a stale reviewed
   target before connecting. A change during a read marks that result outdated.
   The web client admits only a result matching its reviewed identity/scope.
5. Tags belong to this mounted project session, not files or local storage.
   No comparison attempt means no tag. They describe an observation, never
   continuous monitoring or proof against external device changes.
6. The product adapter decodes only verified image placements and format:
   package 1's 0701 group-address and association tables, with each side's
   own TSAP mapping. Invalid counts, missing ranges and unread bytes stay
   unknown; decoding is bounded by declared capacity and segment extent.
   Every raw read range remains visible, even when also decoded.
7. Package 2 captures active parameter bit attribution after each successful
   existing image write, including the selected union field, source name/ref
   and exact segment/offset/bit/width. It never guesses ownership from a diff.
   The adapter projects named project/device values from exactly compared
   bytes using the existing writer's inverse. It rechecks attribution against
   the final plan, so later layout writes cannot silently repurpose its bits.
   Unsupported/negative patterns and incomplete coverage stay unknown with a
   localized reason, alongside every original raw range. This metadata is
   transient and does not change the database or native project format.

## Consequences and limits

UI contains no KNX table/parameter codec and no protocol code. Sessions are
volatile and scoped; gateway and device refusals remain distinguishable. The
read scope is exactly the plan's writable ranges, so masked/unwritten bytes
are not claimed read. A matching image with an unloaded part does not earn
Synced. Other masks remain raw-only at the decoder boundary; existing download
admission is not broadened. Numeric bit order and text encoding are inverses
of existing admitted writers, not new protocol claims. Negative signed project
values remain refused; unknown device patterns are not guessed. Neither package
promises continuous freshness, a new backup format or full ETS compatibility.

**Amendment (WP-18, owner decision 19 October 2026):** point 6's "remains
visible" now means *reachable without loss*. The result display opens with
differences only. A raw range proven fully read and equal may be hidden by
default, and *Show equal rows* reveals every original read range. No filter
hides a differing or unread byte. This changes the display only: the response,
the tags and Synced are unchanged, and the UI still has no KNX codec
([result display](../contracts/device-comparison.md#result-display-wp-18-owner-amendment-19-october-2026)).

[Behavioural contract](../contracts/device-comparison.md).
