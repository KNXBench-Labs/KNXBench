# ADR-0107: Source-bound import integrity and completion vocabulary

Status: local implementation; unpublished.

## Decision

Keep archive extraction bound to the validated physical member identity. Decoded
names are display/lookup identities, not a reversible encoding into the ZIP
library's raw-byte index. Retain both original parsed project documents in
addition to structured fields and opaque fragments. Transport metadata-attribute
ownership explicitly in the mapper result. Metadata uses the full namespace
and local name at known slots, including audit traces; foreign lookalikes stay
retained and reported rather than overwriting typed information. Existing archive/path/CRC/byte limits
are unchanged; no manufacturer code executes.

The existing project parser stream records a fixed lexical observation vocabulary,
including events consumed while retaining unknown subtrees. Observations are not
semantic acceptance and are separate from legacy mapper-visit counts. Overrides
outside the declared active object tree are preserved in original source and
reported; they are not silently activated. Product diagnostics are read from
source-bound persisted evidence on initial import and retry and projected through
the existing import report. Baggage classification reuses the product adapter.

Support both documented root and recursively nested node object-tree lists within
an owning device; root and node lists describe different groups of active objects,
not competing precedence rules. The original structure remains preserved.
Unassigned placement reuses the same DeviceInstance grammar without fabricating a
physical address or line.

Primary evidence inspected: KNX Association *Project Schema23 v01.00.00*,
§1.2.4.6 (unassigned DeviceInstance type), §§1.2.5.21–23 (independent and node
object lists, recursive node children), §1.1.2.6 (completion vocabulary), and
PKWARE APPNOTE §4.4.4/Appendix D (filename flags/encoding). The downloaded schema
is official documentation, not a claim of authoritative XSD validation.

CompletionStatus also contains FinishedCommissioning, Tested and Locked. Add
these explicit values without guessing mappings to legacy values. Bump native
and normalized-model schema to 12 with an identity migration: older binaries
must refuse the new vocabulary instead of rewriting it to Undefined. Existing
rows/history are not rewritten or deleted. Audit current selective-import closure
for the scalar vocabulary-only version change. Unknown completion strings remain
reported and preserved, never mapped to another stated status.

## Constraints and acceptance

No full ETS compatibility, ETS export, signature verification or hardware support
claim. All public tests are independently generated synthetic witnesses. Private
sources and source-bound measurements remain outside Git; shared docs contain no
project-specific data. Acceptance requires parser/mapper/native roundtrip,
version/history migration, CLI/HTTP/UI reporting, hostile-archive regressions and
unmodified-original local verification. Research-only device extensions and
execution limitations remain separate follow-up scopes.
