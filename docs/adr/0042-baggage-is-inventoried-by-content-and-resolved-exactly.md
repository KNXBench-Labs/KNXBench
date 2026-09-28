# ADR 0042: Baggage is inventoried by content and resolved exactly, never opened

Date: 2026-09-28
Status: Accepted
Session: 4 (manufacturer databases), goal item PDB-10
Amends: ADR-0035 (product package install evidence) for the baggage index

## Context

Goal item PDB-10 asks for a safe inventory of product-package *baggage*:
the files a manufacturer ships next to its XML (icons, manuals, plug-in
archives) under `M-XXXX/Baggages/`, declared by `M-XXXX/Baggages.xml`.

Until schema v15 the installer stored every baggage member byte-for-byte
(role `Baggage`, never parsed) and counted the `Baggage` elements of each
index, but typed nothing: every index got an `unsupported-baggage-index`
diagnostic and a `baggage_index / unsupported` count. Nothing said which
declaration named which file, what a file actually was, or whether a file
was declared at all.

What is specified: the KNX Project Schema (v3.0.0 §4.2) says only that each
`Baggage` is stored as an external file. Nothing in the available source
specifies `TargetPath`, `InstallOnImport` or `FileInfo` semantics.

What was measured (read-only, aggregate-only probe of the private corpus on
2026-09-28, recorded in `docs/PRODUCT_DATABASE_CORPUS.md`):

- 38 index files over 117 package instances, grammar exactly
  `KNX/ManufacturerData/Manufacturer/Baggages/Baggage/FileInfo`.
- 777 declarations; `Id`, `Name`, `TargetPath` and `FileInfo/@TimeInfo` on
  all, `InstallOnImport` on 129 with values `true`, `false` *and* `0`,
  `FileInfo/@Version` on 2.
- Every declaration resolves exactly (case-sensitive) to
  `M-XXXX/Baggages/<TargetPath>/<Name>`; 13 of 790 payloads are declared by
  nothing.
- Extensions lie: 35 BMPs are named `.png`, the 11 `.ai` files are PDFs, one
  `.dll` is a PE executable and one `.msi` an OLE2 compound file.
- 37 payloads are ZIPs (7,144 entries, largest 317,240 bytes expanded); none
  is encrypted and none contains a further ZIP.

The historical "1,728 baggage declarations" in the corpus inventory counts a
different unit (every `Baggage` element in every XML of every package
instance, including the 115-vs-117 duplicates and non-index elements), so
neither number is a pin for the other; the matrix pins the installer's own
totals.

## Decision

1. **Typed, uninterpreted declarations.** `parse::baggage` reads each
   declaration's `Id`, `Name`, `TargetPath`, `InstallOnImport`,
   `FileInfo/@TimeInfo` and `FileInfo/@Version` as raw lexemes. No
   boolean/date coercion: `InstallOnImport="0"` stays `"0"`. Any other
   attribute, a second `FileInfo`, or any element outside the grammar is an
   `ingest_unknown` row, reported through the install report like every other
   parser unknown.
2. **Exact resolution only.** A declaration resolves to the member
   `<index dir>/Baggages/<TargetPath>/<Name>` byte-for-byte, or it does not.
   No case folding, no extension guessing, no search. Missing `Name`, an
   absolute/empty/dot/backslashed `TargetPath` component, or a path that no
   member carries yields `invalid` or `missing` with a stated reason.
3. **Content, not names.** Each payload is classified by magic bytes
   (`MediaClass`: PNG, JPEG, GIF, BMP, PDF, ZIP, PE executable, OLE2
   compound, XML, empty, unknown). Weak two-byte magics are confirmed
   (BMP reserved words zero; PE `PE\0\0` at `e_lfanew`), so a text file
   starting `BM`/`MZ` stays `unknown`. The extension is kept only to *report*
   disagreement.
4. **Nested ZIPs: directory only.** A ZIP payload passes the same validator
   as a package (`preflight_zip` and `validate_central_directory`: entry
   budget, directory-size cap, no ZIP64 or multi-disk, only the
   end-of-directory record that ends the buffer, local/central agreement, no
   overlapping records, no repeated entry name). An empty archive (only its
   end-of-directory record) is read as zero entries. Its entry count, *declared* expanded size,
   encrypted-entry count (general-purpose flag bit 0) and `.zip`-named entry
   count are then taken from that checked metadata. The `zip` crate's
   archive reader is not used here: it tries every end-of-directory
   candidate, which is superlinear on hostile input (a 256 KiB payload of
   crafted records measured 8.3 s). No entry is decompressed; nothing is
   ever extracted, written to disk, rendered or executed. An unreadable
   directory is `unreadable`, not zero.
5. **Storage (schema v16).** `package_baggage_inventory` (status
   `measured`/`unavailable`), `package_baggage_payload` (one row per `Baggage`
   member: hash, size, class, nested facts, resolving-declaration count) and
   `package_baggage_declaration` (one row per declaration in document order:
   lexemes, resolution, member, reason). Reload re-parses every retained
   index blob, re-measures every payload from its retained blob and
   re-resolves; the stored rows must equal that result exactly, so a forged
   lexeme is a corruption error even when it leaves the resolution unchanged.
6. **Report shape.** The `baggage_index` `unsupported` count becomes `stored`
   (every declaration is kept typed). `unsupported-baggage-index` is retired;
   `unresolved-baggage-declaration` (grouped per index and reason) and
   `undeclared-baggage-payload` (one per member) name the actual gaps.
7. **Upgrade = fresh install.** v15 → v16 re-derives each package's inventory
   and index unknowns from its retained bytes with the install's own
   functions and rewrites the stored report into the fresh v16 shape,
   including the `package.unknown_count` a fresh install writes. A package
   whose bytes no longer parse, or whose stored v15 report no longer
   validates, gets an `unavailable` inventory and an `unavailable` report plus
   a recorded `InstallReportBackfillError`; only a database failure aborts the
   migration, so the database still opens.

## Consequences

- The UI/API can say *what* a manufacturer ships and *which* files nothing
  declares, without trusting a single extension.
- `InstallOnImport`, `TargetPath` destinations and `FileInfo` timestamps are
  carried but deliberately not acted on; acting on them needs a specified
  source first.
- Payload bytes stay opaque. Rendering an icon or opening a manual is a
  separate, later decision with its own sandboxing questions.
- Install still holds each member whole (≤ 64 MiB, `MAX_MEMBER_SIZE`); a
  55 MB program member measured 4.2× peak RSS growth, which the heavy
  `large_member_memory` test bounds at 8×. Streaming is not claimed.

## Alternatives considered

- **Case-insensitive or fuzzy resolution.** Rejected: the corpus resolves
  exactly, and guessing would hide real gaps.
- **Extracting nested archives into a sandbox.** Rejected: no consumer needs
  their contents yet, and the directory already answers "how big, how many,
  encrypted?".
- **Interpreting `InstallOnImport` as a boolean.** Rejected: the corpus uses
  both `false` and `0`, and no specification defines either.
