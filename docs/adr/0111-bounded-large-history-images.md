# ADR-0111: Bounded compression for large native history images

- Status: implemented locally; integration/publication are separate
- Status update (2026-10-13): on main since 2026-10-11, commit `4e0e874d` (the large-history codec that ADR-0112 was renumbered around). The line above is kept as written; it predates the integration.
- Date: 2026-10-11
- Scope: native history Save As, editing, recovery and versions

## Context

The native importer/store and editor-history image budgets are different. A
project with a large retained opaque context can be valid in native storage but
fail its first application Save As, Save or durable edit. Deleting prior undo
states cannot fix an oversized single initial context. Discarding source bytes
is not an acceptable remedy. Shared context already prevents copying unchanged
opaque bytes into every model state (ADR-0100); that mechanism remains intact.

The existing workspace resolves flate2 1.1.10. Its low-level Decompress API exposes
consumed/produced counts and explicit StreamEnd. StreamEnd verifies the zlib
checksum; merely receiving output or EOF does not establish a complete stream.
Primary sources inspected 2026-10-11:
https://docs.rs/flate2/1.1.10/flate2/struct.Decompress.html and
https://docs.rs/flate2/1.1.10/flate2/enum.Status.html.
SQLite serialization/deserialization retains a standalone native image:
https://sqlite.org/c3ref/serialize.html and
https://sqlite.org/c3ref/deserialize.html.

## Decision

Keep the outer history row format and semantic hash version unchanged. Add a
separately versioned **image codec** in infrastructure, without changing the
KNX core or the imported opaque data:

- Existing SQLite images of at most 64 MiB remain byte-for-byte unwrapped.
- Larger images use magic `KNXHST\0\x01`, an unsigned little-endian 64-bit exact
  decoded length, followed by exactly one zlib stream. The stored-image SHA-256
  covers the complete envelope. Semantic identity continues to cover native
  typed rows, not compressed bytes or SQLite page housekeeping.
- Model/stack/version stored images remain capped at 64 MiB. The single shared
  source context has a separate 256-MiB stored-image cap: retained binary data
  need not compress sufficiently to fit the per-model limit. Physical total
  stored-history payload remains capped at 512 MiB. Each decoded image has an
  independent 512-MiB ceiling. Cumulative decoded image charge is also capped
  at 512 MiB, counting each shared context once. Row/stored-byte limits precede
  fixed-size returned-header accounting before inflation. SQLite `substr` is not
  a guarantee that the engine avoids materializing the already bounded stored
  blob; this is not an exact heap/RSS ceiling. The official scalar-function and
  Column-opcode documentation distinguish BLOB `length` content-loading avoidance
  from general function arguments (inspected 2026-10-11):
  https://sqlite.org/lang_corefunc.html and https://sqlite.org/opcode.html.

  Declared lengths are resource evidence only; existing exact stream/hash checks
  still establish integrity. These are explicit support boundaries, not
  permission for unlimited imports.
- Refuse unknown codec versions, impossible or excessive declared lengths,
  decoded overflow, short output, missing StreamEnd/checksum failure and trailing
  bytes. Decode incrementally in bounded chunks; no allocation based solely on
  an untrusted declared length. Native schema/reference/nonrecursive admission
  still runs on the decoded bytes.
- Fail before publication when the decoded image or encoded image exceeds its
  limit. Do not trim history, discard opaque data or bypass durable editing.
- A per-image limit error must not recommend clearing history as its remedy.
  Total-history/entry-count limits keep their explicit history-management advice.

## Compatibility and consequences

No native table/model migration is required: image blobs already have their own
codec admission and integrity boundary. The previous reader refuses the new
magic rather than partially loading history; files with compressed history
require this or a newer build. Small-image files retain the legacy codec.
The new decoder reads both codecs, including a mixed history. This is forward
writing and backward reading, not backward readability by old builds.

Compression is not encryption, anonymization or a signature. Native projects
still include private retained material. Compression does not solve all large
project memory costs: constructing and admitting complete SQLite snapshots
still takes memory proportional to decoded content. Incompressible or larger
contexts can still be refused; budget coherence for all import sizes remains a
separate limitation. No hardware, ETS revision or full compatibility claim.

## Verification contract

Synthetic tests must first reproduce the prior 64-MiB refusal, then prove exact
large-context save/reopen/edit/undo/redo and version recovery in the application
path, with opaque/manufacturer values retained. Codec tests cover legacy reads,
unknown/truncated/concatenated/corrupt data, dishonest lengths, decoded limits
and incompressible stored limits using small test budgets. Private case replay
is opt-in and stores no case values in public fixtures or receipts.
