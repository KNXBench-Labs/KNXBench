# Product-database gap census: second collection

Date: 2026-10-17. Evidence class: bounded, local, read-only **declaration census**
plus the **production installer** (`knx products ingest`, release build of
`3e770524`) in throwaway databases. Not application evaluation, rendered
fidelity, ETS behaviour or device compatibility.

The owner added four source folders to the local product-database collection.
This census asks two questions: where does KNXBench fall short on this data, and
which further parameter and communication-object layouts the declarations
support. It complements the
[parameter-workspace layout census](2026-10-17-parameter-workspace-layout.md),
which read only top-level `.knxprod` files below a 16 MiB member limit and so
saw neither wrapped packages nor the large complete databases.

## Method

- Scanner (stdlib Python, kept in agent scratch, not in the repository): opens
  ZIPs in memory up to four levels deep, stream-parses every XML member with no
  size cap, refuses DTD/entity declarations, records element and attribute
  **presence**, and records values only for an allowlist of enumeration-like
  attributes (`Layout`, `UIHint`, `Access`, `Unit`, flags, sizes and similar).
  No archive was extracted into the repository, nothing was executed.
- Installer: each new top-level file under a `.knxprod` name (exactly the bytes
  the web upload hands to `install_package`), each inner `.knxprod` on its own,
  and the `--allow-large-package` profile where the standard profile refused.
  Legacy `.vd*` members went through `knx products inspect-legacy` with the
  owner's stored import password (never printed).
- All 145 original files were byte-identical after the scans and installs.
- Counting unit for programs: a distinct `ApplicationProgram/@Id`. "New" means
  the id occurs only in the four added folders; "old" means everything else in
  the collection, now scanned completely (the earlier census skipped 27 large
  members).

## Inventory

| Measure | Value |
|---|---:|
| New top-level files / bytes | 27 / 241 MB |
| Of those: wrapper ZIPs holding product packages | 24 |
| Distinct inner and direct `.knxprod` contents | 39 (2 also elsewhere in the collection, 3 duplicated inside the new set) |
| New distinct application programs | 546 (1 further id overlaps the old set) |
| New programs by schema 20 / 11 / 10 / 13 / unknown | 450 / 89 / 4 / 2 / 1 |
| New programs converted from pre-ETS4 data (`PreEts4Style`) | 194 |
| New `PeiProgram`s (all mask `MV-07B0`, no parameters or objects) | 107 (old set: 1) |
| New programs whose XML member exceeds 8 MiB | 92 (17 %; old set 61 of 508, 12 %) |
| Largest single program XML member | 146 MB |

Further member kinds seen: PDF manuals, `Thumbs.db`, `__MACOSX` resource forks,
an ETS app package (`.etsapp`), plug-in DLLs and vendor data files in `Baggages`.
They are reported here and not interpreted ([KL-134](../KNOWN_LIMITATIONS.md#catalogue-kl-134)).

## Installer outcomes

| Input | Result |
|---|---|
| 24 wrapper ZIPs | 22 refused as "product ZIP is missing knx_master.xml"; 1 refused as an unsafe member path (backslash separators, correct safety refusal); 1 refused by the size bounds |
| 39 inner `.knxprod` | 38 install with the standard profile; 1 complete manufacturer database needs `--allow-large-package` |
| 2 direct `.knxprod` | 1 installs; the other complete database needs `--allow-large-package` |
| Complete database A, large profile | installs: 20.7 s, peak RSS about 311 MiB, product database 918 MiB, 7,233 unknown constructs, 1.25 million translations captured |
| Complete database B, large profile | installs: 135 s, peak RSS about 725 MiB, product database **4.56 GB**, 16,031 unknown constructs, 5.99 million translations captured |
| 9 legacy `.vd5` members | 8 inspect cleanly; 1 complete ETS3 database refused: "legacy EX-IM value count exceeds the limit of 24000000" |
| 1 `.vd2` | refused (known VD2 boundary, [KL-128](../KNOWN_LIMITATIONS.md#catalogue-kl-128)) |

The web catalogue install uses only the standard profile (64 MiB per member,
256 MiB expanded), so neither complete database can be installed from the web
app ([KL-151](../KNOWN_LIMITATIONS.md#catalogue-kl-151) records that boundary).

## Implementation gaps

Ordered by the project's priorities (correctness and data integrity first).

**G1. Block- and separator-level `Access` is ignored.** Project Schema 23
v01.00.00 §1.1.2.1 (printed page 6) defines `Access_t` (`None`, `Read`,
`ReadWrite`) as the user's rights to view and modify parameters. New programs
put it on `ParameterBlock` 9,662 times in 352 programs (`None` 6,147, `Read`
2,884) and on `ParameterSeparator` 15,801 times; the old set has 1,507 block
occurrences. The Dynamic parser keeps it only in `extra`. Write authority
(`effective_access` in `apps/knx-server/src/domain.rs`) reads only the
parameter's and its reference's own `Access`. So a `ReadWrite` parameter inside
a `Read` block is offered as writable, and a `None` block appears as an ordinary
area. That a block's `Access` binds its contents is an inference from the
enumeration's meaning, not a documented inheritance rule; the conservative
direction (refuse writes below `Read`/`None` ancestors and mark the area) cannot
corrupt data.

**G2. Separators outside declared grids are never shown.** Activated
`ParameterSeparator` labels feed only group captions and grid cells
(`domain.rs`, `parameter_grid.rs`); the row view has no separator entries, and
`UIHint` is not projected. New programs declare 74,520 separators in 232
programs: `HorizontalRuler` 37,273, `Information` 6,948, `Headline` 5,233,
`Error` 1,430, the rest unhinted text. Manufacturer information and error texts,
which are typically conditional, therefore never reach the user. No limitation
entry mentions this.

**G3. Wrapper ZIPs are refused with a misleading message.** The usual vendor
download (a ZIP holding one or more `.knxprod`, sometimes with manuals) fails as
"missing knx_master.xml" although every inner package (bar one large database)
installs. A fix must name the inner packages, keep every path-safety check (the
backslash refusal stays), report non-package members instead of skipping them
silently, and leave the inner bytes unmodified.

**G4. Complete manufacturer databases are web-uninstallable and very large.**
See the outcomes table. The 4.56 GB result for a 92 MB package is a measured
scaling cost (5.99 million translation rows); any optimisation must start from
a per-table size measurement, not from this census.

**G5. Presentation readers stop at 8 MiB.** Grids, block captions and checked
editors read the retained source only up to `MAX_EDITOR_SOURCE_BYTES` (8 MiB),
so 92 new programs (17 %) report `sourceUnavailable` for them.

**G6. The checked time editor misreads packed units.** `TypeTime` in new
programs uses `PackedSecondsAndMilliseconds` (443), `Seconds` (401),
`Milliseconds` (242) and other packed units, with hints such as
`Duration_hhmmss` (547) and `Time_ssfff` (443); the old set had a single packed
unit. `chooseParameterEditor` returns `time-count` for every `TypeTime`, so a
packed value is labelled as a decimal count of a packed unit. The local schema
document does not define the packed units; until they are evidenced, packed
units should fall back (`unsupported-codec`).

**G7. Legacy budget.** The complete ETS3 `.vd5` exceeds the EX-IM value-count
budget (24 million). Raising it needs the same measured-budget treatment as
[KL-152](../KNOWN_LIMITATIONS.md#catalogue-kl-152).

**G8. Download side only.** 107 new `PeiProgram`s are refused by the download
planner's load-state-machine check; they carry no parameters or objects, so they
have no UI consequence.

Already documented and now weightier: `Parameter/@SuffixText` is preserved but
not displayed (146 new programs, 45,191 occurrences;
[KL-156](../KNOWN_LIMITATIONS.md#catalogue-kl-156)); `Rename`, `Repeat`,
`Button`, `IndentLevel`, `Inline` and `ShowInComObjectTree` stay unmodelled
(counts below). `ParameterRef/@DisplayOrder` (184 new programs, exactly the
pre-ETS4 conversions) is already honoured by the query layer: not a gap.

## Derivable parameter layouts

Evidence levels: **documented** (defined in a local KNX document), **structural**
(measured placement, meaning follows from the tree), **name-level** (attribute
name only; needs further evidence before a visible claim).

| Layout | Evidence | New-data basis | Notes |
|---|---|---|---|
| P1. Separator rows (headline, information, error, ruler) in the row view | structural | 74,520 separators, 232 programs | Closes G2. Hints become styles; unknown hints stay plain text. |
| P2. Access-aware areas (hidden or read-only blocks marked, writes refused) | documented enumeration, inferred scope | 9,662 blocks, 352 programs | Closes G1. Show `None` areas like `Access=None` parameters today: marked, not silently dropped. |
| P3. Inline sections: an `Inline` block renders inside its parent page instead of as its own register entry | structural, name-level | 891 blocks in 58 programs; in the six largest new packages their parents are a `when` (668) or a block (221), and 741 carry a declared `Grid`/`Table` | Today the innermost block becomes the area (`parameter_layout.rs`), so every inline sub-table becomes a separate area. |
| P4. Row indentation from `ParameterRefRef/@IndentLevel` | name-level | 60,774 in 48 programs (old: 5,466); values 1–5, mostly 1 | Presentation only, no write impact. |
| P5. Unit suffix after the value (`SuffixText`, `ParameterRef` override) | name-level, translated text | 45,191 in 146 programs | The display gap is already recorded next to KL-156. |
| P6. Conditional block captions from `Rename` | structural | 712, all inside a `when`, targeting blocks (696) or separators (16), 87 programs (old: 6) | Needs the evaluator's active branches; inactive renames must not show. |
| P7. Checkbox and float slider controls | name-level | `TypeNumber@UIHint=CheckBox` 161 types, `TypeFloat@UIHint=Slider` 36 | Same checked-editor rules as the integer slider. |
| P8. Side-by-side comparison of repeated module instances | structural (identity) | in the six largest new packages, 23 of 519 programs instantiate one `ModuleDef` at least twice (up to 480 times) | Keyed by `ModuleDef` parameter identity, never by captions; plain `Channel`s have distinct ids and stay out. |

Not derivable without new evidence: `Table` versus `Grid` styling,
`CollapseIfEmpty` (8,598 rows in new data), `Button` (14, script event
handlers, not executable), and icons/pictures beyond the existing unconverted
original.

## Derivable communication-object layouts

| Layout | Evidence | New-data basis | Notes |
|---|---|---|---|
| O1. Folders: objects grouped under `ParameterBlock`s with `ShowInComObjectTree` | documented: Project Schema 23 §1.2.5.21–23 (printed pages 51–52) defines the project's `GroupObjectTree` with `Node` types `Folder` ("ParameterBlock with ShowInComObjectTree") and `Channel` | 343 blocks in 8 new programs (old: 322 in 9); 3,591 object references inside such blocks in the six largest new packages | For imported projects, the exact folder tree is already in `GroupObjectTree`, which the importer reads for object ids only. |
| O2. Objects by parameter page | structural | 332,499 object references; in a three-package sample 77,233 sit below a block, 1,395 directly below a `Channel` and 218 below the `ChannelIndependentBlock` | "Objects on this page" from the Parameters tab and a group-by-page mode in the object table; references without a block keep a visible "no page" group. The layout reader already records `controls_com_objects`. |
| O3. Flag and size columns (C R W T U I, object size, priority) | documented attributes | `ObjectSize` and flags on every one of 58,507 new `ComObject`s | Flags are already in the object DTO; size and priority need an additive field. |
| O4. Module-instance comparison for objects | structural (identity) | the 23 programs from P8; `ComObject/@BaseNumber` 1,722 | Same identity rule as P8. |

## Suggested order

1. G1/P2 and G2/P1: correctness first, both small and server-led. Done
   (17 October 2026, [ADR-0139](../adr/0139-block-access-and-separators-reach-the-parameter-tab.md)).
2. G3 wrapper ZIPs: removes the most common install failure on real downloads.
3. G6: make packed time units fall back.
4. P3, P4, P5, O1, O3: presentation work on data the server already holds or
   can read from the retained source.
5. G4/G5 (size ceilings, database growth) with measurements first; P6, P8, O2,
   O4 after their contracts.

Detailed per-file results, program ids and the scanner stay in agent scratch;
no manufacturer bytes, texts or identifiers are recorded here.