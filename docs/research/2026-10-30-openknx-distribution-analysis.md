# OpenKNX distribution data: capabilities, limits and next steps

**30 October 2026 · analysis, not an implementation or a publication.**
Code compared: `a6cc2ebe99fa2f7ac272bc119f05d98c8af82f44`.
Public English edition, 1 November 2026, of a German working note. The
collection, the original note and the detailed evidence stay private and
untracked; this edition keeps the findings, figures and boundaries and leaves
out local paths, evidence file names and hashes of local document copies.
Original archives and survey files were not modified.

## Status since this analysis

The work packages proposed below (OKX-01 to OKX-07) have since been delivered
in bounded form. Before implementation, the 32-row matrix was re-audited
against `72db561a4`, where every source file the matrix cites was
byte-identical to the compared code above. The
[capability reconciliation of 1 November 2026](../status/2026-11-01-openknx-capability-reconciliation.md)
says, row by row, what landed on `main`, which commits and tests back it, and
what is still blocked. The rest of this note is the analysis as it stood on
30 October 2026 and is not updated for later deliveries.

## Result in brief

KNXBench already has extensive product-data parsers, versioned storage,
Dynamic evaluation and parameter and object views. None of that needs to be
rebuilt. The OpenKNX release ZIPs, however, are **not packages that the
existing product-package importer accepts**. A successful run of a single
parser proves neither a complete import nor a correctly initialised
configuration, UI parity or programmability.

**No new defect in the supported native package path was found.** What the
analysis did establish is the input boundary of this distribution form, and
concrete semantic limits that are mostly documented already: in particular
assignments, calculations, manufacturer checks and event handlers that are not
executed. What is new is their source-bound classification, their scale and
testable implementation slices.

**First recommended implementation package:** an explicitly read-only XML
distribution adapter with a section and completeness report. Not a supposed
`.knxprod` import, and not a JavaScript runtime. It should make real
identities, references and source capabilities usable without pretending to
install, to be editable, or to have master data that is missing. This was
**an option for a separate assignment**, not an architecture change already
decided.

## Method and evidenced coverage

- All **37 original archives from 36 repositories** were re-examined against
  their hashes: 37 distinct product XMLs and application IDs, schema 20 × 36
  and schema 14 × 1. No ready-made `.knxprod` and no `knx_master.xml` in the
  examined set.
- 15,741 recursive members were fully read and hashed; 74 XML member hashes
  match the first survey. Product element counts were reproduced
  independently. Catalogue → product/Hardware2Program → application:
  **37/37** resolvable. Resolving the captured typed references is not a check
  of script logic or of external master definitions.
- Representatives: **IP-Router** (small, absolute segments and Property),
  **Dummy-Small** (object reference alternatives, no DPT attributes),
  **EnOcean 1.9** (schema 14), **RaumController** (largest XML),
  **AccessControl** (300 validations) and **NeoPixel** (largest script text,
  1,294 calculation declarations). Selection criteria and ties are recorded in
  the private evidence.
- Structural and targeted lexical and source-coordinate checks of all 37 XMLs;
  deeper source inspection of the six representatives. **No complete semantic
  check of every application.** Native component probes covered five of them;
  NeoPixel was not evaluated natively.
- 2,541 concrete XML witnesses were cross-checked with a second parser against
  the original bytes, locators and attributes. Twelve self-authored scanner
  controls cover, among other things, scopes, Unions, encoding, DTD and
  resource defences.
- The research probe used unchanged KNXBench library source with a **separate
  probe lock file**, not the exact production workspace build; some dependency
  versions differ. Runtime calls ran in isolated XDG profiles, explicit
  temporary databases and a separate network namespace. No bundled scripts,
  Producer, ETS libraries, firmware, bus or device operations were executed.

Every material detail finding carries the archive hash, the original member
path, the member hash, the application ID and the XML locator or attribute in
the private evidence. Observation, executed probe, documented semantics and
assumption are kept apart. Code paths carry revision, file hash and symbol or
line reference. A source check is not an executed UI test.

## Real component probes and their exact limit

| Input / call | Observation | Not established by it |
|---|---|---|
| 37 unchanged release ZIPs → `install_product_file` | 28 first refused with `MissingMaster`, 9 first with `UnsafeMember` because of backslashes; no installation | That the nine would otherwise be valid, or that one repair would fix every problem |
| Four unchanged product XMLs → `ingest_file` | Only `Catalog` each time: one source blob and one catalogue entry, no application or hardware; retry `Skipped` | A complete import of the merged document |
| Explicit public Hardware/Catalog/Static/Dynamic parsers | Typed data for five different applications; database checked after a clean close and a read-only reopen | Normal installer, full report/translation/baggage/identity orchestration or write authority |
| `resolve_values` + `evaluate` | Activations under raw, unscoped supplied/ref/default fallback values; IP-Router: two defaults changed, one named activation switches as expected | Initialisation/Assign/calculation semantics, a full state-space check or the parameter edit workflow |
| `load_program_code` on five original XMLs | Segments and procedures readable; unmodelled load steps explicitly retained | An executable load plan, a memory image or device support |
| Five independently authored mini-XML controls | Split Dynamic and value changes; catalogue-first boundary; low-level namespace boundary; complete broken XML refused | Production XSD conformance or arbitrary rollback, crash or migration properties |

The experimental parser composition in particular does **not** call the
private write-authority pass: an unmarked program must not gain edit
permission from it. Unknown constructs returned by the parsers were recorded
separately and not inserted into `ingest_unknown`: a zero there does **not**
mean "no unknown content" here. Specialised parsers can report legitimate
sibling sections handled by a neighbouring parser as unknown. Their group
counts must not be added up into a list of "missing features".

The reopen check establishes selected row counts and
`PRAGMA integrity_check = ok`. It does not establish full equality of all
content, report histories or byte-identical sources after reopening. The
temporary databases were removed afterwards; originals and measurements were
kept.

## Capability and gap summary

The full **32-row matrix** and its resolved witnesses are private evidence; 17
original witnesses were additionally re-checked in a targeted way. The summary below does
not replace them. Its current status is in the
[capability reconciliation](../status/2026-11-01-openknx-capability-reconciliation.md).

| Area | Already present | Verified remainder / classification |
|---|---|---|
| Identities/references | Separate product database, catalogue/hardware/application parsers, conflict and provenance mechanisms | No explicit multi-section path for this distribution; missing master and declared manufacturer identity stay separate |
| Types/defaults | Typed declarations, raw values, ref/parameter defaults, source provenance | A resolved fallback is not a computed initial state; not every stored type has an editor or byte codec |
| Memory/Unions | Own offsets, Memory/Property, Union assignment and conservative image limits | Active alternative, byte order, sentinel and initialisation rules and load effects must not be derived from occurrences |
| Dynamic/access | Bounded `choose/when`, module-scoped foundations, existing write authority and block access | `Assign` deliberately not executed; foreign calculations, checks and handlers not interpreted; `NoBranchMatched` alone is not an error |
| Module definitions | Parser and bounded nested module handling exist | 30 definitions, but **no instances, arguments or allocators** in the corpus: no new evidence for WP-07 instance arithmetic |
| Communication objects | Object/ref/Dynamic layers, DPT/flag/size resolution and object refresh | Native probes confirm partial paths, not effective DTO/UI/undo acceptance; Dummy without DPT is a counter-witness against blanket DPT assumptions |
| Calculations/checks/buttons | Source and unknown retention, existing calculation-related write refusal | 19,451 calculations, 303 validations, 33 scripts, 8,979 buttons: declarations, not executed functions |
| UI/texts | Workspace, grids, separators, indentation, caption/language/suffix resolution and checked partial editors | No new browser check; remaining layout forms one at a time under WP-05. Language blocks are not a translation oracle |
| Images/icons/help | Baggage inventory, media detection, fallback and report mechanisms | 116 declared resources mapped to their sources, 46 PNG/70 ZIP; the new distribution envelope is not native package baggage ownership |
| Segments/loading | Absolute and relative source models, conservative offline/app limits | 36 relative/1 absolute segment; **no Data/Mask blobs** in these declarations. Do not invent fill, relocation or complete images |

### Observations that are not yet a defect list

A direct check of explicit number ranges finds **7,141 parameter defaults**
and **100 ref overrides** outside their declared TypeNumber bounds. Examples:
EnOcean `4294967296` for `0..4294967295`; NeoPixel `0` for `1..64`. These are
declaration occurrences, not as many independent active wrong values.
Inactivity, deliberate sentinel values and unevaluated logic are unresolved.
**Do not correct, clamp or discard them.**

Likewise, **180** Union interval overruns under explicitly known widths and
offsets are candidates at first. Overlapping alternatives are precisely one
purpose of Unions; simultaneous active conflicts were not proven. No
completeness was pretended for unknown widths. The historical Manufacturer
Tool document explains ref layers and this Union purpose, but does not define
the whole modern schema 14/20 behaviour [K2].

### Resources and safe handling

The largest product XML is **75,763,660 bytes**. A supplementary RaumController
composition run took about **45.6 s** process time with **427,436 KiB** peak
RSS. The process covers more than parsing (migration, reading, evaluation,
code reader, output, reopen). A single debug run with a probe lock file is
**not a release benchmark**, a UI limit or evidence that a parser rewrite is
needed. Scanner and native measurements stay separate; RSS was taken per fresh
native child process.

The two `content.xml` encoding errors were reproduced unchanged. W3C XML 1.0
§4.3.3 requires a contradicting encoding declaration to be treated as fatal
when no qualifying external encoding information is present [1]. That does
not automatically make the separate product XML unreadable. The nine observed backslash
refusals are likewise no licence to loosen the native importer's path safety.

## Adapter decision: own input boundary, existing parsers

**Not sensible:** a second general KNX XML parser, or relabelled ZIPs with an
invented master, signatures or manufacturer registration.

**Sensible as a bounded option:** a separate distribution boundary that reuses
proven parsers and safety mechanisms. It has to know provenance and section
ownership, validate the complete original XML, validate namespaces and
references, retain unknown content and report an explicit
`not attempted / retained / processed / partially evaluated`. Repeating
`ingest_file` under another name does not work because of hash-bound parse
markers. Feeding the full text to every parser unchecked would also leave
translation, scope and unknown double-counting problems unsolved.

The first delivery should be a **useful read-only source inspection**. A later
data import would additionally need checked atomic orchestration, write
authority/identity/translation/baggage passes, a persistence and migration
contract, and a separate completeness and authenticity statement. Missing
master data must not be silently replaced by whatever master happens to be
present. Schema23 describes master data and its system definitions, but
excludes manufacturer dependencies and image generation logic from its own
project-format scope [K1]. No XSD, ETS or registration approval follows from
it.

The official Producer describes an ETS dependency and itself warns that its
checks do not guarantee ETS usability [2]. That is another reason not to use
it here as a validator or as an unnoticed conversion step. Its signing
implementation and usage rights were not examined in this analysis.

## Prioritised, bounded work packages

Each package was specified privately with problem, existing code, concrete
sources, approach, layers, tests, acceptance and open decisions. `OKX-*` are
research handles, not newly assigned ledger IDs or work approvals.

| Order | Proposal | Existing task / boundary |
|---|---|---|
| **1: OKX-01** | Read-only distribution adapter with section and completeness report; no pretended package installation | Option under WP-10; needs its own assignment |
| OKX-02 | Report the effect of unevaluated Assign/calculation/validation/handler dependencies explicitly | Refine PDB-02/PDB-03/R-DYNAMIC-01; no JavaScript execution |
| OKX-03 | Discriminate source-bound default and Union candidates; do not guess write semantics | WP-09/PRG-01; respect current owners and parallel work |
| OKX-04 | Explicit distribution baggage mapping and passive display | Reuse existing inventory/PDB-08; depends on the input contract |
| OKX-05 | Exactly one evidenced remaining layout form | WP-05; do not rebuild existing grids, separators or access rights |
| OKX-06 | Exactly one independently defined editor or codec case | WP-08/WP-09; Date lexical handling and Time/Slider already exist |
| OKX-07 | Owner-bound synthetic regression and resource acceptance | Cross-cutting part of each delivery, not an extra feature project |

Minimal own regressions: a combined XML with four sections; a split versus
monolith counter-probe; a foreign namespace with the same local name;
ref/default/empty-value precedence; two conditionally exclusive Union aliases;
an unevaluated Assign/validation declaration; object/ref DPT and flag
precedence; a baggage path collision and a BOM/encoding conflict. None of them
may take foreign product files, images or scripts as public fixtures. Existing
UI, storage and write tests are extended, not replaced by parser green.

## Provenance of the analysed data

- **Origin:** the official OpenKNX GitHub organisation [3]. 136 organisation
  repositories were listed; all 42 repositories with the `OAM-` prefix were
  checked for the first release position the public API listed.
- **Selection:** per repository, the assets of that release position,
  including releases marked as pre-release. That is neither a complete
  historical survey nor evidence of the latest stable version. Five `OAM-`
  repositories had no published release, and for one release no ZIP or
  `.knxprod` asset was selected.
- **Result:** 37 unchanged ZIP archives from 36 repositories, 249,836,553
  bytes in total; 8 of the archives come from pre-releases. Collected on
  30 October 2026.
- **Retrieval path:** direct GitHub download URLs were not fetched after a
  `robots.txt` check. The official public release-asset API returned the
  permitted redirects to the asset host. After the anonymous API limit was
  exhausted, the existing GitHub CLI login was used; no credentials are stored
  in the private manifest.
- **Integrity:** size and SHA-256 for 37/37 archives were recorded and
  re-verified after re-reading, and 29/29 upstream digests available at the
  time matched. The private manifest keeps
  original file names, release and asset IDs, URLs, timestamps, sizes, hashes
  and the repositories' licence metadata.
- **Rights:** a public download and repository licence metadata are not a
  blanket permission to redistribute or reuse every bundled file. The rights
  to bundled data, firmware and media were not cleared one by one. No OpenKNX
  file, image, script or extract is published with this note; the fixtures
  that its follow-up packages added are self-authored.
- **Execution boundary:** build scripts that reference OpenKNXproducer and the
  local generation of a `.knxprod` were read as text only. No Producer, ETS
  signer, PowerShell script or firmware was run; 507 script and executable
  file names were inventoried, not executed.

## Reproduction, limits and sources

The private evidence set contains the archive and XML survey with its twelve
controls, independent witness checking and deterministic aggregate
reproduction; the counts, concrete candidates and selection; the code audit
(32 paths from parser and storage to DTO and UI, with source hashes and
reconciliation against existing tasks, without claimed test runs); the
separately bounded native probe lanes; the methodology, measurement method and
independent review; and the inspected primary sources with retrieval and file
hashes. It is not published because it contains original member paths,
identifiers and extracts of third-party data.

No production code, native importer or signing fix, firmware or bus access,
publication of third-party data, or commit was part of the analysis. No
whole-workspace, browser, ETS, hardware or complete semantic acceptance is
claimed.

## Sources

[1] https://www.w3.org/TR/xml — W3C XML 1.0, Fifth Edition, §4.3.3;
retrieved and the passage checked on 30 October 2026.

[2] https://github.com/OpenKNX/OpenKNXproducer/blob/7f23a63ef641bb1c5501697af62d330181e3e23f/README.md —
OpenKNXproducer README at a pinned revision; sections "Features and
Arguments", "Tipps" and "Requirements". Retrieved through the official API and
read as text only.

[3] https://github.com/OpenKNX — official OpenKNX organisation on GitHub;
repository and release listing through the public GitHub API on
30 October 2026.

- **[K1]** KNX Association, *Project Schema23 v01.00.00*, 1 March 2024,
  §§1.1 and 2, printed pages 4–5. The identity hash of the inspected copy is
  kept in the private evidence.
- **[K2]** KNX Association, *Manufacturer Tool v01.00.01*, §§2.2–2.4, printed
  pages 5–6. The identity hash of the inspected copy is kept in the private
  evidence. A historical concept document, not a complete modern
  manufacturer-format contract.

### Closing the analysis

The research delivers a reproducible source and code comparison and bounded,
testable proposals. **Still open** at the time were the exact modern
manufacturer semantics, active defaults, Unions and initialisation, native
codecs, full layout fidelity and any real device effect. These points are not
quietly marked as solved and are not replaced by more element counts or
repaired packages.
