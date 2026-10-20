# Offline monitoring project export

Status: implemented candidate; focused App/CLI/HTTP/web checks executed, final
integrated acceptance and receiving-application acceptance remain separate.

## Purpose and boundaries

Export one current normalized installation for offline monitoring. The profile is
**`monitoring-sparse-v1`**, not a complete engineering interchange format. It is
not a native backup, an engineering roundtrip, an ETS writer, a device image or a
programming operation. Core and native/product schemas do not change. Application
services own mapping and validation; transports never reconstruct engineering rules.

The export also works for projects created in KNXBench without an imported archive.
Retained source XML never replaces current edited names, addresses, types, links,
topology or building Functions. A file-based export uses recovered native working
state, while the HTTP surface uses the current open in-memory project snapshot.
A CLI cannot see changes held only by another running process.

## Container and disclosure

One deterministic ZIP contains plain `project.json`, `export-report.json` and
`README.txt`. The plain object is not wrapped inside the report. The report binds
the exact JSON bytes by SHA-256; keeping the pair in one artifact prevents a report
from accidentally describing another file. The CLI publishes a new file atomically
and never overwrites an existing output, input store or catalogue.

No keys, passwords, raw manufacturer files, opaque binaries, attachment contents,
notes, installation instructions, native history or recovery state are included.
Names, engineering labels, device addresses and room names are intentional private
project content: this is not an anonymized support bundle. Files stay local; no
upload, shared runtime, bus session or receiver service is required by the exporter.

## Why the profile is sparse

A normalized project does not establish a group's security state and may not know
all six object flags. The complete external-shaped record requires booleans and
has no unknown sentinel. A fabricated `false` would turn missing knowledge into a
claim. Therefore:

- `group_addresses.*.data_secure` is omitted and reported, never guessed.
- Unknown object flags are omitted individually and reported; known flags remain
  JSON booleans, including an explicitly known `false`.
- Missing GA DPT is `null`; missing object DPT is `[]`; main-only types have
  `sub: null`. Missing information is not subtype zero or an inferred byte codec.
- Unknown descriptive strings use `""`, nullable metadata uses `null`, with
  scoped unavailable-data diagnostics. These conventions are not measured facts.
- Module/channel/functional-block/DPA metadata not supplied by the normalized
  export is explicitly unavailable. No guessed module offsets or channel names.

This is **not conformance to a complete external TypedDict**. A receiving tool
needs a separately tested adapter that handles the sparse fields. Neither JSON
nor ZIP upload support in any receiving application follows from this export.

## Mapping and identity

The object has eight root maps: `info`, `communication_objects`, `devices`,
`topology`, `locations`, `group_addresses`, `group_ranges`, `functions`.

| Domain | Monitoring projection |
| --- | --- |
| Installation | Exactly one selected installation; never merged address spaces |
| Device | Physical-address dictionary key; current name/description and resolved optional product data |
| Object | Device address plus stable local object ID; current text, number, DPTs, size, known flags and links |
| GA | Style-formatted address key, numeric raw address, stable local identifier, current name/type and reverse object links |
| Range | Stable local range key, raw inclusive bounds, direct GA members and nested child ranges |
| Area/line | Numeric area and local line keys, names, medium description and device references |
| Space | Sibling-name key, stable local space identifier, current kind/name/number, devices, nested spaces and Functions |
| Function | Stable local identifier, owning space identifier and address-keyed members with literal names/roles |

IDs are stable within the native project, not globally unique or guaranteed
identical across independent imports. Source identifiers are not required for new
projects. Duplicate dictionary keys must refuse, never silently overwrite.

GA declarations and object DPTs remain independent; any existing type inference
is reported as inference. Conflicting widths/types are not resolved by choosing a
convenient first candidate. A width-conflict outcome yields `null`, with the
declared and linked candidates reported; a same-width differing declaration may
remain effective. Line media map the exact source tokens `MT-0`/`MT-1`/`MT-2`/
`MT-5` to TP/PL/RF/IP descriptions; `MT-3` is not guessed to mean IP.
Cleared instance values are not resurrected from
program defaults. Current authored text wins over optional product translations.
Object link direction, activity and engineering-only state lack equivalent fields
and must be disclosed rather than silently presented as lossless.

An absent selector chooses the sole installation only; zero or multiple
installations refuse without an explicit valid selection. Selected devices are
the unique union of that installation's line, unassigned and building references;
repeated placement within one installation is valid. Cross-installation ownership
and globally orphaned devices refuse. Nonselected installations are excluded
scope, not counted as omissions. Functions belong to their location's installation.

A device without an individual address cannot be an address-keyed record: it and
its objects are omitted with explicit entity counts selected before pruning.
Addressed unassigned devices can remain represented. Omitted entities must not
leave dangling references. Duplicate numeric area keys/local line numbers, numbers
outside 0–15 and lines without exactly one area parent refuse. `RoomPart` has no
admitted target kind and refuses the whole export, not a silently omitted subtree.

Project metadata identifies KNXBench as producer. `xknxproject_version` is an empty
string, not a claim that another parser ran; native schema numbers are not
presented as ETS format versions. Missing GUID/source metadata is reported.

## Integrity, budgets and report

Validate IDs, address uniqueness, entity ownership, parent/child consistency,
cycles and every exported reference before publication. Structural corruption or
resource exhaustion refuses the whole export. No source mutation, writable native
migration, catalogue creation/reinstall or network operation is permitted.

The report is version 1, with profile, producer/version, installation selector,
exact project SHA-256, per-entity `selected` / `exported` / `omitted` counts,
ordered diagnostic rows and excluded data categories. Counters use entity units,
not an unqualified percentage of completeness; `selected = exported + omitted`.
A zero counter is a measured zero for this projection, not proof that historical
or opaque input contains nothing else. Missing historical context is unavailable.

Resource policies: at most 100,000 entities and 1,000,000 visited reference/member
entries (duplicates still consume admission), 32 MiB cumulative admitted text,
hierarchy depth 64, 64 MiB serialized project JSON and 64 MiB final ZIP. Generated
text, diagnostics and repeated output copies are admitted as well; serialization
uses bounded writers, not only a length check after an unlimited allocation.
These are projection admission limits, not a measured process RSS guarantee. Native working
history recovery uses the existing validated editor reader and can materialize
undo/redo history; constant-space loading is not promised.

## Adapters

CLI shape:

```text
knx monitor-export <project.knxdb> <new.zip>
    [--installation <id>] [--language <code>]
    [--product-db <path> | --no-product-db]
```

Read project and working history in a consistent read transaction, using existing
read-only/in-memory-migrated openers. Before either SQLite opener, the CLI admits
only quiescent standalone rollback-layout inputs: regular SQLite files with
write/read header versions `1/1` and no `-wal`, `-shm` or `-journal` entry at the
supplied path or resolved target. WAL headers refuse even without sidecars;
dangling catalogue links refuse rather than behaving like an absent catalogue.
A genuinely absent optional catalogue is allowed only without recovery sidecars.
The exporter never checkpoints, recovers, removes sidecars or converts journal
mode. Read-only SQLite flags alone do not establish side-effect-free WAL reads.
This is not protection against concurrent external writers or adversarial path
replacement between inspection and open. See
[the input-layout evidence](../research/2026-10-20-monitor-export-input-layout.md).

Optional catalogue reads never create or migrate the user's file. Build all
content before output creation; a unique sibling
temporary, file synchronization, no-clobber publication and directory synchronization
provide the publication path. An output race must leave the winner untouched.
If directory synchronization fails after no-clobber publication, the CLI reports
that the complete ZIP was published but durability is unconfirmed. It must not
claim no output was written or delete the published file as presumed rollback.

HTTP preview and download must bind server incarnation, project incarnation and
revision. Preview returns the report and a digest of the exact bundle; download
rebuilds and compares it, detecting catalogue-derived changes even without a
project edit. Stale binding or digest refuses. The web dialog discloses reduced
scope/private content and omissions before explicit local ZIP download. Neither
client selection nor a stale late response may replace the reviewed export.

## Required acceptance before delivery

1. Semantic RED before new behavior; API absence evidence kept separate.
2. Synthetic native project created without ETS, working-state edits and reopen.
3. Three address styles, installation selection, malformed/unknown data,
   addressless devices, DPT disagreements and missing catalogue context.
4. Duplicate keys, dangling/cross-owner references, hierarchy cycles and bounds.
5. Independent ZIP/hash/report readback and existing-output/source preservation.
6. Real CLI and HTTP execution; rendered UI contract and visual check if exposed.
7. Independent shape/reference validator and deliberately corrupted rejection
   controls, separate from receiver runtime interoperability.
8. Independent whole-feature review and maintained integrated package gate.

No receiving-application, ETS or hardware interoperability is claimed.
