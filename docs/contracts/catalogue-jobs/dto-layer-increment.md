# WP-04.3 stage D — strict serde wire DTO layer (P2/P3)

Claimed 22 October 2026. The second increment of WP-04.3 stage D
([design-readiness.md](design-readiness.md) stage D: owned job/lifetime API)
under [ADR-0150](../../adr/0150-cancellable-catalogue-jobs.md). Stage D is split
into separately claimed increments in the staged order. This increment delivers
the strict serde wire-DTO layer the protocol consumes, as private unwired
additions to the existing `catalogue_jobs` module. No production route, upload,
spool, status or drain becomes reachable.

## Scope of this increment

A new private module `catalogue_jobs::dto` (declared from the existing
`#[allow(dead_code)]` `catalogue_jobs`) holding:

- **Strict JSON admission** mirroring the P2 rules of
  [protocol-artifacts.md](protocol-artifacts.md): duplicate object keys are
  rejected at every nesting level, `deny_unknown_fields` on every struct,
  leading JSON BOM, invalid UTF-8, non-finite numbers and nesting deeper than 8
  levels refuse; trailing non-whitespace input refuses. Two passes over the
  bounded input: a dependency-free duplicate/depth walk, then the derived
  `Deserialize` (which rejects unknown fields, wrong nullability and closed
  enum tokens).

- **Wire DTOs and closed enums** (serde `rename_all = "camelCase"`) for the
  core protocol objects of P3 and the schema `$defs`: `Handle`, `Profile`,
  `Source`, `Progress`, `Lease`, `Failure`, `Commit`, `Cleanup`, `Backup`,
  `Artifact`, `Preview`, `Result`, `Apply`, `Status`, `Admission`, `Create`,
  `ApplyCommand`, `CancelCommand`, `CancelReply`, `DisposeReply`, `Error` and
  `CatalogueDrain`. `U64` serializes as a canonical decimal string and
  deserializes only from that grammar; `JobId`/`Incarnation`/`Sha256`
  serialize/deserialize as their exact lowercase-hex strings. `Capability`
  has no JSON form (header-only). Unknown enum tokens refuse (closed).

- **Semantic validation** of the P3 field/null/state contract: const fields
  (`schemaVersion == 1`, `policyVersion == "wp043-v1"`, `confirmed == true`,
  `growthCliffPossible == true`, `projection == "redactedCompleteV1"`,
  `context == null`, `disposition == "disposed"`), source-name grammar and byte
  bounds, profile/`allowLargePackage` consistency, and the complete
  `stateRules` cross-field table for `Status` (phase/gate/stopCause/commit/
  cleanup/backup/failure couplings per state).

- **Error mapping** from every closed `FailureKind` to its fixed HTTP status
  and fixed English message (the `errors` table of `wp043-protocol-policy.json`).

## Explicitly out of scope (later stage-D increments)

- `DetailQuery`, `Page` and the `PublicRecord` artifact-record types: those
  depend on the P6 raw/public artifact schema and arrive with the
  artifact/details increment.
- Upload/spool sealing, status snapshots, progress/lease counters and TTL.
- The ArtifactDir lease/inventory/reclamation and filesystem fence (P8).
- Standalone/desktop close/drain and the `EmbeddedServerHandle` lifetime (P9).
- Any production route wiring, Ready/apply or worker execution.

## Done when

- Malformed input is refused: duplicate keys (top-level and nested), unknown
  fields, wrong nullability, a JSON numeric `U64`, uppercase hex, overlong
  `U64`, non-finite numbers, a BOM, trailing input and depth > 8.
- Every DTO round-trips valid wire through `serde_json` and rejects invalid
  state through its `validate` method; each terminal `Status` state is
  discriminated for its required coupling.
- Focused tests and the owning `knx-server` suite pass; warning-denied Clippy,
  `cargo fmt` and header checks are clean; no production route becomes
  reachable and no quiet API, schema or response shape changes.

## References

- [ADR-0150](../../adr/0150-cancellable-catalogue-jobs.md)
- [design-readiness.md](design-readiness.md) (stage D ordering)
- [protocol-artifacts.md](protocol-artifacts.md) (P2/P3)
- [wp043-protocol.schema.json](wp043-protocol.schema.json) (field/null shapes)
- [wp043-protocol-policy.json](wp043-protocol-policy.json) (stateRules, errors)
