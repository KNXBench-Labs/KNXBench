# ADR-0142: File-size limits follow the storage ceiling

Status: accepted.
Date: 2026-10-18.
Amends: [ADR-0082](0082-large-product-packages-are-a-cli-opt-in.md) (both package
profiles), the KL-152 evidence budgets, and the project-container budget raised
on 2026-10-10.

## Context

The owner asked that no size limit get in the way of large real data. Where a
specification defines a limit, KNXBench should use that limit plus 25 %
headroom.

Real local data (three product collections and six projects, 18 October 2026),
measured by member role over 196 product packages and 1,462 application
programs:

| Quantity | Largest measured | Former bound |
|---|---:|---:|
| Raw product file | 107.7 MiB | 256 MiB |
| Application program XML (one member) | 281.8 MiB | 64 MiB standard, 256 MiB large |
| Programs above 8 MiB / 64 MiB | 199 / 7 | 8 MiB editor and contribution scans |
| Expanded product package | 1.7 GiB | 256 MiB standard, 4 GiB large |
| Members per package | 444 | 4,096 |
| Legacy `.vd5` payload | about 371 MiB | 256 MiB |
| `.knxproj` file | 110.5 MiB | 64 MiB selective-import source |
| Scheme-evidence items in one program | more than 1,048,576 | 1,048,576 |

**No KNX specification limits the size of a product or project file.** The
standard limits that do apply:

- **[V] ZIP, APPNOTE 6.3.10.** A classic (non-ZIP64) archive counts entries in
  16-bit fields (§4.4.22) and stores sizes and offsets in 32-bit fields
  (§4.3.9.2, §4.4.1.4). KNXBench refuses ZIP64 product packages, so 65,535
  entries and 4 GiB − 1 per member are the format's own maxima. A headroom above
  a field's maximum cannot be reached by any conforming file.
- **[V] SQLite.** The bundled SQLite is built with the default
  `SQLITE_MAX_LENGTH` of 1,000,000,000 bytes, the largest single value it
  stores (checked in its amalgamation; enforced at run time, see Evidence).
  Every retained file, member, payload and history image is one such value. A
  larger file could be read but never retained.

## Decision

1. **One storage ceiling**, `knx_core::limits::MAX_RETAINED_BYTES` =
   1,000,000,000 bytes, bounds every single retained file or member:
   - raw product package input (and the web upload bodies for it);
   - a decoded package member (both profiles);
   - legacy file and payload;
   - the master-language, editor, layout and grid source scans, and the
     contribution scan of one XML member;
   - project container entries, manufacturer files, selective-import sources
     and payload downloads (including the web client's own cap);
   - project-history images and shared contexts.
   This is not a KNX limit: it is the most KNXBench can store. It is 3.4 × the
   largest measured member, so no spec headroom rule applies.
2. **ZIP format maxima** replace smaller structural caps. Product packages and
   contribution archives accept up to 65,535 members. ZIP metadata (central
   directory, local headers) is bounded by the input it lives in, and decoded
   names by three times that (a CP437 byte decodes to at most three UTF-8
   bytes). The member-path tree keeps a memory guard of 16 nodes per possible
   member (1,048,560), because a crafted 65,535-byte name could otherwise
   create unbounded nodes.
3. **Expansion budgets** (no format bounds them; they guard against
   decompression bombs) are `MAX_EXPANDED_ARCHIVE_BYTES` = 4 GiB: the standard
   product profile, project containers, contribution archives and the
   cumulative project history. That is 2.4 × the largest measured package. The
   explicit large profile (`--allow-large-package`) now differs only in its
   total, 16 GiB.
4. **Work budgets follow the input**, because every XML event or item consumes
   at least one input byte: the editor, grid and contribution event budgets and
   the scheme-evidence item budget equal their input bounds. The
   scheme-evidence byte estimate also charges the repeated ancestor path per
   item (KL-152 measured about 193 charged bytes per item), so it gets eight
   times the ceiling.
5. **Unchanged, on purpose:**
   - KNX wire-protocol limits.
   - RFC limits (DNS names and labels).
   - Text-field caps (notes, names, instruction documents, RTF texts,
     attribute lengths).
   - Caps on KNXBench's own generated records (import evidence, report
     catalogues, exports, session logs).
   - The procedure-resolution document tree (KL-152 neighbour): it builds a
     complete in-memory tree for the MV-07B0 contribution report, and a
     streaming reader, not a larger tree, is the right fix.
   - Nesting depths.

## Consequences

- Every measured real file installs with the default profile; the opt-in is only
  needed above 4 GiB expanded. Measured with the release CLI: the 1.7 GiB package
  takes 110 to 131 s, peak RSS 714 MiB and a 4.5 GB database; the package with
  the 281.8 MiB program takes 22 s, peak RSS 1.16 GB and a 687 MiB database.
- Large inputs cost memory and time in proportion: files are still read whole.
  A file above 1 GB needs chunked storage, which does not exist. It is refused
  with a typed size error, never truncated.
- Boundary tests use the constants, zero-filled buffers (mapped lazily) and
  declared-size ZIP headers, so no test allocates the bounds it checks.
- An older database may carry "unexamined for size" master-language markers;
  the next re-derivation examines those sources and clears the markers.

## Evidence

- `crates/knx-core/src/limits.rs`.
- `crates/knx-productdb/tests/zip_cap_boundaries.rs` (65,535 entries install;
  one more needs ZIP64 and is refused; inclusive member and total bounds of
  both profiles; a real 65 MiB member installs by default).
- `master_evidence_rederive.rs` and `parameter_editor.rs` (SQLite refuses a
  value one byte above the ceiling).
- `cli_large_package.rs`, `archive_member_integrity.rs` (4 GiB project budget).
- The private large-package run summarised above (counts only, private evidence
  directory).
