# ADR-0113 — Durable historical import evidence in native passthrough

- Status: Accepted and implemented locally; source integration/publication separate
- Date: 2026-10-11

## Context

The native snapshot already retains versioned, hash-bearing passthrough rows and
shares them once across ordinary history states. An import report belongs to the
application/import adapter, not the KNX domain model. Native opening previously
had no full historical report and explicitly marked diagnostic counts unavailable.
Parallel work owns the product model/native schema migration; adding a competing
schema version or an external-format dependency to `knx-store` is inappropriate.

## Decision

Use a dedicated **application-owned** passthrough kind and exact reserved native
metadata identity for a version-1 import-evidence envelope. This is not an ETS
member or a source XML field. Preserve the complete serialized original report,
including uninterpreted findings, independently of the bounded operational log.
`knx-app` alone validates and interprets the envelope. `knx-store` continues to
persist generic opaque rows; no domain or native schema migration is needed.

Generation occurs after import/enrichment reporting and before replacing the
active server project. An encoding refusal fails the load without replacing its
current state. Native opening validates the envelope before publication; absent
legacy evidence stays unavailable, never fabricated. Duplicate primary records,
unknown envelope versions, invalid identity/hash/JSON/report-counter shape and
oversized evidence fail closed. Report bytes are bounded to 16 MiB per envelope.
The existing snapshot/context/history bounds also continue to apply.

Historical counts are derived from the original report using the same category
semantics as initial import. The full report can be retrieved explicitly from the
current authenticated project API; historical provenance is visible separately
from session operations and is **not** current compatibility, digital-signature
verification or new hardware observation. The complete report remains unchanged
through edit/save/restart/undo/redo. Selective import does not invent a complete
merged-project historical report from a source report.

## Consequences

- Existing snapshot history preserves the new metadata without new snapshot
  fields or fingerprint algorithms; legacy readers retain an unknown opaque row.
- Old native projects cannot recover reports they never stored.
- The report may contain private source names, attributes and identifiers. A
  native save or explicit local report download is not anonymization; no automatic
  external transmission is permitted.
- The reserved kind avoids matching arbitrary ETS files with a similar basename.
  Hashes provide corruption detection, not authenticity against deliberate native
  file alteration. Historical provenance names a recorded claim, not an oracle.
- Server import and CLI `import --store` reuse the application capture helper.
  Older CLI-only files and source-scoped selective timelines remain explicitly
  outside primary-report restoration; no missing evidence is manufactured.
- Any later dedicated table/migration must preserve these envelopes and retain
  unavailable/unsupported distinctions, not silently drop them.
