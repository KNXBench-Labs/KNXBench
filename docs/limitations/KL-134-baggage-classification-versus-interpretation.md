# Baggage classification versus interpretation

[Current summary and ledger status](../KNOWN_LIMITATIONS.md#catalogue-kl-134)

This dossier preserves the technical detail and dated history. Earlier
headings, failures and lifting conditions are historical where later
updates supersede them; they are not new work assignments.

<!-- BEGIN retained catalogue record -->

## §134 Baggage is inventoried, not interpreted

**Update 2026-10-10.** The import report's wording is coarser than this
inventory: every manufacturer `Baggages/` member, images included, is reported
as "vendor plugin binary", while project-side ETS app packages and add-in data
are plain container entries. See §174.

**Limitation.** Since PDB-10 (schema v16, ADR-0042) every baggage
declaration is typed and resolved and every payload is classified, but
nothing acts on it: `InstallOnImport`, `TargetPath` destinations and
`FileInfo` timestamps are carried as raw lexemes, icons are not rendered,
manuals not opened, plug-in DLLs/MSIs not run, nested ZIP entries not
decompressed. Media classes cover the formats the corpus contains (PNG,
JPEG, GIF, BMP, PDF, ZIP, PE, OLE2, XML); anything else is `unknown`, not
guessed. A nested ZIP's expanded size is what its directory *declares*, not a
measured decompression. Installing still holds each member whole in memory
(≤ 64 MiB each; a 54.8 MB member measured 4.2× peak RSS growth). Baggage
*references* from application programs
(`Static/Extension/Baggage/@RefId`, 935 distinct in the corpus) are reported
as unknown constructs, not resolved against the inventory; whether the single
`Hardware/Product/Baggages` element is reported has not been probed. When a
`Baggages.xml` is both ingested standalone and carried in a package, a fresh
install records its index unknowns under that blob once per path while a v15
upgrade records them once, so unknown *row* counts (not the inventory or
report) can differ in that shape. The package corpus alone cannot produce
it; a project import followed by installing the same package can.

**Cost.** The user sees what a manufacturer ships and which files are
undeclared, but cannot preview them in KNXBench, and a package that relies on
a baggage file being installed on import gets no such installation.

**Why it is this way.** The Project Schema specifies only that each
`Baggage` is an external file; the corpus spells `InstallOnImport` as both
`false` and `0`. Acting on undocumented semantics, or opening untrusted
vendor payloads, would trade integrity and safety for convenience.

**Lifted when.** A specified source defines the declaration attributes, or a
concrete feature (icon display, manual links) needs a payload and brings its
own sandboxed viewer with tests.

**Alpha decision (user, 2026-10-04).** Accepted as an Alpha boundary.
<!-- END retained catalogue record -->
