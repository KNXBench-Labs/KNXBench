# CSV group-address exchange — design

**Task:** T12 in `docs/GAP_ANALYSIS_ETS.md`. Closes gap **C2** ("No CSV/Excel
group-address import or export").

**Goal:** let a user export the open project's group addresses to a CSV file,
edit that file in a spreadsheet, and import it back to create new addresses and
rename existing ones — a bulk-authoring workflow that does not require a full
`.knxproj` round trip.

---

## 1. The honesty problem, stated first

ETS has a "Export Group Addresses" feature. This design **does not claim
compatibility with it**, because nothing in this repository or in the KNX
Standard v3.0.0 corpus documents what that export actually looks like:

- All 179 documents of the extracted KNX Standard v3.0.0 corpus were searched
  for `csv`, `esf`, `OPC export`, and group-address export terminology. The two
  `csv` hits are incidental prose in a data-security test report and an RF
  application note. There is no standardized group-address exchange text format.
  The CSV export is an ETS *application* feature, not a KNX Association one.
- `OriginalData/` contains no `.csv` and no `.esf` sample. No `docs/` file had
  previously researched either format.

Consequences that bind every part of this design:

1. **KNXBench defines its own documented format**, "KNXBench group-address CSV
   v1", specified in §3. It is not presented anywhere — docs, UI, or commit
   message — as an ETS-compatible file.
2. **The importer is column-name-driven and separator-detecting** (§4), so that
   a file produced by some other tool has a fair chance of being read even
   though its exact header text could not be predicted in advance.
3. `.esf` (the legacy OPC export) is **out of scope**. It is absent from the
   standard corpus, no sample exists, and writing a parser against remembered
   syntax is exactly what CLAUDE.md's "do not invent technical facts" forbids.
   If it is ever wanted, it becomes its own task, gated on first obtaining a
   real file.

If a genuine ETS CSV export is supplied later, adding a compatibility profile to
the importer is a small change — the column-mapping layer already exists for it.
That is the intended upgrade path, and it is the only claim of ETS
interoperability this work is entitled to make.

## 2. What a group address actually is here

From `crates/knx-core/src/group.rs:41`:

```rust
pub struct GroupAddressEntry {
    pub id: GroupAddressId,
    pub source: SourceRef,
    pub name: String,
    pub address: GroupAddress,
    pub central: bool,
    pub unfiltered: bool,
    pub range: Option<GroupRangeId>,
}
```

Four facts drive the format design:

- **There is no datapoint type on a group address.** A DPT belongs to the
  communication objects linked to the address (`ComObjectInstance.dpt`,
  `device.rs:61`), several of which may disagree — `knx-etsproj` has a whole
  `infer::Conflict` type for that case. 194 of the 514 addresses in the
  reference project have no DPT at all. So a `DatapointType` column can be
  *derived* for export but can never be written back on import.
- **There is no description field.** The `.knxproj` schema has
  `GroupAddress/@Description` and `@Comment`
  (`Project Schema23 v01.00.00.md` §1.2.7.3); this domain model does not carry
  them yet, so the CSV cannot either.
- **`central` and `unfiltered` are real, editable fields** of the entry, so the
  CSV can carry them honestly in both directions.
- **The address style is project-wide**, not per address
  (`ProjectInfo::group_address_style`, `project.rs:152`): `Free`, `TwoLevel`,
  or `ThreeLevel`. `GroupAddress::parse`/`format` (`address.rs:104`/`:164`)
  already implement every string form. The CSV layer must call them and must
  not reimplement `x/y/z` handling.

Group address `0` is reserved for broadcast
(`03_05_01 Resources v01.10.01 AS.md` §3.4.2.1) and the project schema's
`Address` attribute is `[1…65535]`, so it is never a valid row value.

## 3. The format: KNXBench group-address CSV v1

- **Encoding:** UTF-8. Written with a BOM (Excel opens BOM-less UTF-8 as the
  system code page and mangles umlauts); read with or without one.
- **Line endings:** written CRLF, read either.
- **Separator:** written `,`. Read `,` or `;`, auto-detected (§4).
- **Quoting:** RFC 4180 — a field containing the separator, a quote, or a line
  break is `"`-quoted and embedded quotes are doubled.
- **Header row:** required. Columns are matched by name, case-insensitively,
  ignoring surrounding whitespace. Column order does not matter.

| Column | On export | On import |
| --- | --- | --- |
| `Address` | address in the project's own style, e.g. `1/2/3` | **required**, matched as the row's identity |
| `Name` | entry name | **required**, must be non-empty |
| `Central` | `true`/`false` | optional, applied |
| `Unfiltered` | `true`/`false` | optional, applied |
| `DatapointType` | derived, `DPST-1-1` form, empty when absent or contested | accepted, **not applied**, counted in the report |
| `MainGroup` | name of the containing main range, if any | accepted, **not applied**, counted in the report |
| `MiddleGroup` | name of the containing middle range, if any | accepted, **not applied**, counted in the report |

Booleans are written `true`/`false` and read case-insensitively as
`true`/`false`/`1`/`0`/`yes`/`no`; an empty value means `false` on create and
"leave unchanged" on update.

`DatapointType` is derived from the DPTs of every communication object linked to
the address: unanimous → that DPT; none → empty; disagreement → empty plus one
warning per contested address in the export report. Never a silent pick.

The three export-only columns exist because a spreadsheet of bare addresses and
names is much harder to work with than one that shows what each address is for.
They are read back and explicitly reported as not applied, rather than being
rejected (which would make this tool's own export un-importable) or ignored in
silence (which CLAUDE.md forbids).

## 4. Import semantics

**Matching.** A row is matched to an existing entry by its parsed address.
Names are not identities; two addresses may share a name.

**Per row, one of four outcomes:**

| Outcome | When |
| --- | --- |
| create | no entry has that address |
| update | an entry has that address and at least one applied column differs |
| unchanged | an entry has that address and nothing differs |
| error | see below |

**Errors (row-level):** address missing, unparseable, or out of range for the
project's style; address `0`; name missing or blank; a boolean column that is
neither empty nor a recognized truth value; an address that appears twice in
the same file.

**All-or-nothing.** If any row is an error, nothing is applied. The report names
every offending row so the user can fix the file and retry. Partial application
of a hand-edited spreadsheet is how people end up with half-renamed projects and
no idea which half.

**Import never deletes.** An address present in the project but absent from the
file is left alone. A CSV is an edit, not a replacement.

**Import never re-addresses.** Since the address is the match key, changing an
address in the spreadsheet reads as "create a new one", not "move the old one".
`docs/KNOWN_LIMITATIONS.md` records this; deleting and recreating is the
supported way to re-address, and the group-address view already does that.

**Range placement.** A newly created address is placed in the innermost existing
`GroupRange` whose `start..=end` contains it (`GroupRange::contains`,
`group.rs:28`) — middle range preferred over main range. If no range contains
it, it is created with `range: None` and the report says so. No ranges are
created by a CSV import.

**Unknown columns** are collected and reported by name, once per file.

**One undo step.** All commands are wrapped in a single `Command::Batch`
(`command.rs:217`), which already rolls back applied sub-commands if a later one
fails, so a CSV import is one `Ctrl+Z`, exactly like T9's bulk delete/move.

## 5. The one new command

`crates/knx-core/src/command.rs` today has `CreateGroupAddress` and
`DeleteGroupAddress` and nothing else that touches a `GroupAddressEntry`. There
is no way to rename one. Import therefore needs:

```rust
Command::UpdateGroupAddress {
    id: GroupAddressId,
    name: String,
    central: bool,
    unfiltered: bool,
}
```

Its inverse is the same variant carrying the previous three values — the pattern
`SetDeviceDescription` and `RenameGroupRange` already use. It fails with the
existing `CommandError::GroupAddressNotFound` for an unknown id, and, like
`RenameGroupRange` and `CreateGroupAddress`, it does not itself police empty
names: name validation lives in the layer that has a user to report to, which
for this feature is the CSV reader (§4). It deliberately does
**not** change the address or the range: the address is the identity the import
matches on, and moving an entry between ranges is a separate concern with its
own UI.

This command is also what a future inline rename in the group-address view will
use, so it is not CSV-specific plumbing.

## 6. Where the code lives

A new crate, `crates/knx-csv`, depending only on `knx-core`, the `csv` crate,
and `serde`. This follows the existing one-crate-per-external-format precedent
(`.knxproj` → `knx-etsproj`, product packages → `knx-productdb`) and keeps
`zip`/`quick-xml` out of the CSV compile unit. `xtask check-layering` gains the
matching rule: `knx-csv` must reach neither `knx-store` nor `knx-etsproj` nor
`knx-productdb`.

Its public surface is pure — text and `&Project` in, text and typed reports out.
It knows nothing of SQLite, `AppState`, or HTTP:

```rust
pub fn export_group_addresses(project: &Project) -> CsvExport;
pub fn parse_group_addresses(text: &str, style: GroupAddressStyle) -> ParsedCsv;
pub fn plan_import(project: &Project, parsed: &ParsedCsv) -> ImportPlan;

pub struct CsvExport { pub text: String, pub warnings: Vec<CsvWarning> }
pub struct ImportPlan { pub command: Option<Command>, pub report: CsvImportReport }

pub struct CsvImportReport {
    pub separator: char,
    pub rows_read: usize,
    pub created: usize,
    pub updated: usize,
    pub unchanged: usize,
    pub ignored_columns: Vec<IgnoredColumn>,
    pub problems: Vec<CsvProblem>,
}
pub struct CsvProblem { pub row: Option<usize>, pub severity: Severity, pub detail: String }
```

`ImportPlan::command` is `None` when there is nothing to do or when a row-level
error blocked the whole file; `report.problems` explains which.

The report deliberately mirrors `knx_etsproj::ImportReport`'s shape (counts,
problems with a location and a severity, a "has losses" question) so the session
log, the CLI, and the UI can treat it the way they already treat an import
report. `row` is the 1-based line number in the file including the header, so it
matches what a spreadsheet shows.

Orchestration stays outside: `apps/knx-server/src/domain.rs` for the HTTP path,
`apps/knx-cli` for the headless path, both calling the same three functions.

## 7. Surfaces

**Server** — two routes, both path-based, mirroring `/api/project/import` and
`/api/project/export` rather than inventing a new upload shape:

- `POST /api/group-addresses/csv-export` `{ path }` → `{ warnings }`, writes the
  file through the same `resolve_new_project_path` helper the `.knxproj` export
  uses.
- `POST /api/group-addresses/csv-import` `{ path }` → `{ tree, report }`.

Both push entries into the T11 session log via a new
`session_log::from_csv_import_report`, tagged `csv-import:*`/`csv-export`, and
neither calls `SessionLog::reset` — only opening a whole project does that.

**CLI** — two subcommands over a `.knxdb` store, in the existing hand-rolled
argument style (this CLI has no `clap`):

```
knx ga-export <store.knxdb> <out.csv>
knx ga-import <store.knxdb> <in.csv> [--dry-run]
```

`--dry-run` prints the report and writes nothing. Exit code 2 ("ran, but the
report has errors") is reused from the existing import path for a file that was
rejected row-wise.

**Web** — two buttons in the group-address view, using the existing
`pickSavePath`/`pickOpenPath` abstraction, a toast with the one-line summary,
and the detail in the Log tab that T11 already built. No new panel.

## 8. Testing

- `knx-csv` unit tests: separator detection, BOM, CRLF, quoted fields with
  embedded separators and quotes, unknown columns, missing required columns,
  every row-level error, boolean spellings, all three address styles.
- `knx-csv` plan tests over a hand-built `Project`: create, update, unchanged,
  duplicate-in-file, range placement by containment, and the all-or-nothing rule.
- A corpus-gated round trip: import the reference `.knxproj`, export its group
  addresses, re-parse, plan — and assert the plan is entirely `unchanged` with
  no problems. This is the real proof that the writer and the reader agree,
  including on names containing commas, quotes, and umlauts. Uses the standard
  `corpus_available()` skip guard so CI without `OriginalData/` stays green.
- Server tests for both routes including the rejected-file case.
- A CLI test for `--dry-run`.
- Frontend component tests for both buttons and the report toast.

## 9. Out of scope, and recorded as such

- ETS CSV compatibility (no sample; see §1).
- `.esf` / OPC export.
- Re-addressing an existing group address by CSV (§4).
- Importing datapoint types (§2) — they belong to communication objects.
- `Description`/`Comment` — not in the domain model yet.
- Creating or renaming group ranges from a CSV.
- Deleting addresses absent from the file.
