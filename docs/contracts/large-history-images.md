# Large native history images

Local implementation, 2026-10-11; source integration and deployment are separate.

## Contract

- Retained source bytes and manufacturer references are not removed to make an
  import fit the application Save/Edit path.
- Model/stack/version images remain limited to **64 MiB stored**. The single
  shared source-context image has its own **256 MiB stored** budget. Each decoded
  image is bounded to **512 MiB**. Total physical history payload remains
  **512 MiB**, with at most 256 stack states and 256 named/save versions. A
  separate **512 MiB cumulative decoded** budget counts every physical model
  image and each shared context once. Fixed-size header preflight runs before
  inflating blobs; many small compressed images cannot bypass it. SQLite may
  materialize a stored blob internally for `substr`; the preceding stored-image
  and physical-total caps still apply. This is not a 16-byte heap/RSS guarantee.
- Images at or below the legacy 64-MiB threshold remain plain native SQLite.
  Larger images use the independently versioned envelope in
  [ADR-0111](../adr/0111-bounded-large-history-images.md). A large poorly
  compressible shared context can use its separate storage budget, without
  increasing the per-model limit or being duplicated for ordinary edits.
- Stored-byte SHA-256 and semantic native identity are separate. Changing the
  image representation does not change the semantic identity algorithm.
- Decode requires the exact declared length, a complete single zlib stream and
  no trailing bytes, then the existing native schema/integrity/reference checks.
  Unknown versions, malformed streams and excessive bounds are refused.
- A single-image refusal must not recommend clearing older undo/version entries.
  Image-limit refusals in history endpoints use `projectHistoryUnavailable`
  (HTTP 422). Existing Save/Edit endpoint error conventions are unchanged.
- History responses additionally expose `maxContextBytes` and
  `maxDecodedImageBytes` and `maxDecodedHistoryBytes`; the history dialog
  displays them in EN/DE. Older responses that omit any of these fields are accepted without inventing its value.
  Present malformed values refuse the complete view.
- Old readers do not support the new codec. Larger-context or compressed-history
  files require this reader or a compatible newer build. No ETS format changes.

## Verification

Public tests use fictional data only:

| Requirement | Test |
| --- | --- |
| Legacy bytes and bounded compressed roundtrip | `knx-store` image-codec unit tests |
| Unknown, short, corrupt, concatenated/trailing, length and budget refusal | image-codec negative tests |
| Cumulative expansion refused before application-level image materialization/inflation across all four tables (not an SQLite heap/RSS bound) | `project_history::decoded_budget_tests` |
| API/UI limits, legacy omission and malformed optional values | HTTP history and client/dialog tests; EN/DE browser history cases |
| Large exact snapshot, including retained bytes/references | `project_history::large_retained_snapshot_is_exact_without_raising_stored_history_limits` |
| Poorly compressible context, shared once after edit | `project_history::incompressible_shared_context_uses_its_own_budget_and_is_not_repeated_per_edit` |
| Save As/Open/Edit/Save/restart/Undo/Redo | server `http_large_project_history` synthetic test |
| User-facing refusal without impossible cleanup advice | server `project_history::image_limit_tests` |

The ignored private lifecycle test requires an explicit input and an explicitly
selected private evidence directory. It fails when selected without those
prerequisites; no silent skip is accepted as a replay. It asserts every HTTP
status, exact normalized model, retained opaque/context/manufacturer data and
unchanged original source. Failure bodies and source-bearing artifacts remain
outside tracked/public files. Execution of this opt-in is not general ETS
compatibility or hardware acceptance.

## Remaining limits

This is not universal coherence with the importer's larger archive budget.
Decoded contexts over the ceiling, overly large incompressible contexts, total
history exhaustion and unsupported native model states remain explicit refusals.
Constructing/admitting a native snapshot still uses memory proportional to the
materialized data. Compression neither anonymizes nor encrypts private content.
Early import-time write-capability preflight remains follow-up work.
