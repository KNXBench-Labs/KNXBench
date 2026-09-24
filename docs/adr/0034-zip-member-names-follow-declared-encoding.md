# ADR 0034: ZIP member names follow their declared encoding before safety checks

Date: 2026-09-23
Status: Accepted
Session: 7

## Context

A bounded local corpus contains eleven packages with ordinary non-ASCII
characters in ZIP member names encoded with the historical IBM code page 437
and general-purpose bit 11 unset. Rejecting every member whose raw bytes are not
UTF-8 blocks otherwise supported scheme-11/20 packages. Decoding with a lossy
UTF-8 replacement would be worse: distinct raw names could collapse and path
checks would inspect a value different from the retained member path.

PKWARE's ZIP APPNOTE Appendix D defines UTF-8 when general-purpose bit 11 is set
and the original IBM PC character set (commonly code page 437) when it is not.
APPNOTE 4.6.9 additionally defines the Info-ZIP Unicode path field (`0x7075`):
version 1 and a CRC of the header name decide whether its UTF-8 value applies.
The pinned `zip` crate implements CP437 decoding, but accepts unknown Unicode-
path versions and rejects stale CRCs rather than ignoring both as APPNOTE says.

## Decision

The preflight inspects every physical central record and its referenced local
header. Raw names, general-purpose flags and compression methods must agree;
two central records may not share or overlap local-file ranges. The declared
central-directory extent must be exact. CRCs and sizes must agree when bit 3 is
clear; with bit 3 set, zero local placeholders and a matching data descriptor
are required. A name with bit 11 set must be valid UTF-8. With bit 11 clear,
`zip::ZipFile::name` supplies the ZIP-standard CP437 decode.
The parser receives a bounded view that begins at the selected central
directory; metadata from an earlier EOCD is therefore unreachable. EOCD magic
inside central-entry or final-EOCD comments is hidden in that parser-only view,
so malformed selected metadata cannot make the ZIP library retry a comment as
another directory. The same byte sequence outside a complete comment range is
rejected rather than changing semantic member metadata. The parser's effective
central/local offsets, CRC and sizes must still match every preflight record
before extraction.

KNXBench parses each `0x7075` field itself. Exactly one version-1 field with a
matching raw-name CRC may override the decoded path and must contain valid
UTF-8. Unknown versions and stale CRCs are ignored as specified; a read-only
parser overlay neutralizes only their field identifiers so `zip` cannot
reinterpret them, without cloning or changing the complete archive. The
original remains stored byte-for-byte. Duplicate fields,
conflicting local/central fields, and conflicts with flagged UTF-8 are rejected.

Only after decoding does KNXBench reject empty names, NUL, backslashes, drive or
scheme separators, absolute paths, `.`/`..` components, symlinks, project
partitions and normalized file/directory name collisions. Extraction and the
stored `PackageMember.path` use that same checked Unicode value. Raw central
names are still checked for duplicates independently, so decoding support does
not weaken the anti-aliasing boundary. A hash-identical retry repeats this
preflight and extraction and checks the stored member index rather than letting
an older database bypass a strengthened boundary.
Decoded component-prefix collisions use an iterative component trie capped at
65,536 nodes. The cap bounds adversarial disjoint-depth allocation while shared
deep prefixes remain linear in the supplied path components and do not recurse
on insertion or destruction.
Central-directory and local-header metadata each have a 24 MiB aggregate cap,
and decoded paths have a 72 MiB aggregate cap (the three-byte worst-case
expansion of the central-name budget). These checks run before parser metadata
expansion. A validation pass streams and hashes one expanded member at a time;
only member identities remain resident. After all members pass, the atomic
ingest pass re-reads one member at a time instead of retaining all expanded
payloads or a second full decoded-path list.

## Alternatives considered

- **Require every raw name to be UTF-8.** Rejected: ZIP permits the historical
  encoding and eleven measured packages rely on it.
- **Decode invalid bytes lossily.** Rejected: replacement characters destroy
  identity and can create collisions.
- **Guess a locale-specific Windows code page.** Rejected: the archive carries
  no such declaration, and guessing would be nondeterministic.
- **Maintain a second CP437 table in KNXBench.** Rejected: the pinned ZIP parser
  already owns and tests that format primitive; duplicating the table creates a
  divergence risk without adding a security boundary.

## Consequences

UTF-8 and CP437 member names are retained as Unicode, while malformed flagged
UTF-8, traversal and post-normalization collisions remain hard failures.
Synthetic tests cover both encodings, local/central identity, Unicode-path
fallbacks and attacks. An ignored, environment-opt-in local regression inspects
physical central records and pins only the observed count plus one aggregate
SHA-256 commitment over sorted package identities, without publishing
individual fingerprints, manufacturer names, filenames or data. This is ZIP
filename compatibility, not a claim of complete product-database or ETS
compatibility.

Source: PKWARE, *ZIP File Format Specification*, Appendix D,
<https://pkware.cachefly.net/webdocs/casestudies/APPNOTE.TXT>.
