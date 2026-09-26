# ADR 0037: Preserve application-program catalogue metadata as source values

Date: 2026-09-24
Status: Accepted for PDB-7 implementation

## Context

The [product-database corpus follow-up](../PRODUCT_DATABASE_CORPUS.md) calls
out missing secure and version metadata on standalone product packages.
The read-only local corpus inventory observes eight relevant
attribute names on `ApplicationProgram`, not on hardware or a bus connection:

| Attribute | Observed XML occurrences (including duplicate package instances) |
| --- | ---: |
| `IsSecureEnabled` | 34 |
| `MaxSecurityGroupKeyTableEntries` | 34 |
| `MaxSecurityIndividualAddressEntries` | 32 |
| `MaxSecurityP2PKeyTableEntries` | 27 |
| `MaxTunnelingUserEntries` | 6 |
| `MaxUserEntries` | 6 |
| `MinEtsVersion` | 310 |
| `ReplacesVersions` | 140 |

The local inventory includes nested packages and exact duplicates, so these
are occurrence counts, not distinct programs. The inventory's aggregated
value-shape buckets (no raw source values printed) found 271 dotted-numeric
and 39 other `MinEtsVersion` occurrences; `ReplacesVersions` had 57
unsigned-decimal and 83 other occurrences. The classifications are purely
lexical, not KNX or ETS version semantics. Source bytes survive in
`source_file`, but `PROGRAM_ATTRS` does
not recognise these names, `application_program` has no dedicated columns,
and `query::programs` cannot return them. Successful package installation is
not proof of the fields' runtime meaning.

## Decision

PDB-7 adds a **versioned product-database migration**, not a project-store
migration. Store the eight values returned by the existing XML attribute
decoder (`Attrs::evidence_value`) as nullable `TEXT` on the
owning `application_program` row, and expose them through an explicit
`query::ProgramRow` source-value projection (`Option<String>` per field),
including the CLI `knx products show <program-id>` view. The projection is **raw-only**:
even `IsSecureEnabled` remains an uninterpreted XML lexeme, not `bool_flag`'s
validated runtime boolean. Consumers must not silently coerce malformed
boolean spelling into a security claim. `NULL` means absent/unavailable;
`Some("")` means an explicitly empty XML value. These strings are decoded and
normalized by the existing parser, not guaranteed byte-for-byte XML lexemes;
the original XML bytes remain in `source_file`. Do not parse version strings,
comma-separated replacement expressions or numeric capacities into invented
semantics. A future validated consumer may offer a separately named parsed
view while retaining the original lexemes.

A fresh ingest writes metadata only for the program row that wins the existing
content-hash/ID conflict rule. A losing duplicate may still be reported in
install evidence but never overwrites the winner's metadata. Unknown reporting
must no longer label successfully persisted unqualified attributes as unknown;
qualified lookalikes keep their distinct evidence and must not overwrite the
unqualified value. Atomic package rollback continues to cover metadata.

Migration from v12 must rederive metadata from retained `source_file` bytes for
**winning rows only**, matched by both program ID and `source_sha256`, as
per [ADR-0020](0020-migrations-may-rederive-from-stored-bytes.md). Do not
invent values for absent or malformed historical blobs. Use per-blob savepoints
and explicit parse-failure diagnostics, preserving a usable database. The
backfill reader must reject ordinary premature EOF with unclosed XML tags,
not just tokenizer errors, before committing any updates for that blob.
Retain pre-migration `ingest_unknown` rows as historical ingest evidence:
they aggregate sibling programs by xpath/name without program ID and cannot
be selectively retired on a successful winning-row backfill. New ingests
recognise the eight unqualified attributes, but migration does not rewrite
historical per-package encounter counters or claim that a migrated install
was measured again. A second open must not change values or diagnostics.

Catalogue queries (and a minimal CLI presentation) display source values as
metadata, not a security capability declaration. No KNXnet/IP, commissioning,
Data Secure keys or ETS-version compatibility decisions follow from these
attributes. Product-scheme acceptance and `.knxproj` project-schema support
remain independent.

## Verification before claiming completion

RED→GREEN synthetic fixtures must cover all eight fields; absent versus empty
and unusual version/replacement/boolean lexemes; a winning/losing program ID conflict;
qualified-attribute lookalikes; old-v12 migration with correct winner/source
binding and idempotent reopen; a malformed retained blob that cannot poison
other programs; atomic rollback after an earlier member write; and an opt-in
read-only corpus matrix that pins aggregate field counts without publishing
private identities or raw values. Full workspace gates and an independent
whole-branch review precede merge.
