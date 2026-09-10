# CSV group-address exchange implementation plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** export the open project's group addresses to a CSV file, edit it in a
spreadsheet, and import it back to create new addresses and rename existing ones.

**Architecture:** a new pure `knx-csv` crate owns the text format and the
create/update plan; `knx-core` gains the one missing `UpdateGroupAddress`
command; server, CLI and web stay thin callers.

**Tech Stack:** Rust, the `csv` crate, Axum, React, Vitest.

**Spec:** `docs/superpowers/specs/2026-09-10-csv-group-address-exchange-design.md`

## Global Constraints

- **No ETS-compatibility claim anywhere.** No verified ETS CSV export sample
  exists in this repository or in the KNX Standard v3.0.0 corpus. Docs, UI text,
  commit messages and code comments describe "KNXBench group-address CSV v1",
  never "ETS CSV". `.esf` is out of scope.
- `knx-csv` stays pure: text and `&Project` in, text and typed reports out. No
  SQLite, no `AppState`, no HTTP, no filesystem. It may depend only on
  `knx-core`, `csv`, `serde`.
- Never silently discard input. Every column the importer does not apply, and
  every column it does not recognize, is counted and named in the report.
- Address strings are parsed and formatted only through
  `knx_core::GroupAddress::parse`/`format`. Do not hand-roll `x/y/z` handling.
- Import is all-or-nothing and never deletes, never re-addresses, never creates
  group ranges.
- One CSV import is one `Command::Batch`, hence one undo step.
- Test-first: write the failing test, run it, then implement.
- Commit messages in this repository are written in the voice of Marvin, the
  manically depressed robot from *The Hitchhiker's Guide to the Galaxy* — gloomy
  and world-weary, while the technical content stays accurate and complete. No
  `Co-Authored-By` trailers; commit as `github@knxbench.com`.

---

### Task 1: The `knx-csv` crate and its reader

**Files:**
- Modify: `Cargo.toml` (workspace members + `[workspace.dependencies]`)
- Create: `crates/knx-csv/Cargo.toml`
- Create: `crates/knx-csv/src/lib.rs`
- Create: `crates/knx-csv/src/read.rs`
- Modify: `xtask/src/main.rs`

**Interfaces:**
- Produces: `parse_group_addresses(text: &str, style: GroupAddressStyle) -> ParsedCsv`
- Produces: `ParsedCsv { separator: char, rows: Vec<CsvRow>, ignored_columns: Vec<IgnoredColumn>, problems: Vec<CsvProblem> }`
- Produces: `CsvRow { line: usize, address: GroupAddress, name: String, central: Option<bool>, unfiltered: Option<bool> }`
- Produces: `CsvProblem { row: Option<usize>, severity: Severity, detail: String }`, `Severity { Error, Warning }`

- [ ] **Step 1: Write the failing reader tests.**

  In `read.rs`'s `#[cfg(test)] mod tests`, cover: a minimal comma file; the same
  file semicolon-separated; a UTF-8 BOM prefix; CRLF line endings; a quoted name
  containing the separator, a doubled quote, and an umlaut; all three
  `GroupAddressStyle` forms; header matching that is case-insensitive and
  whitespace-tolerant and order-independent; an unknown column collected by
  name; the three export-only columns (`DatapointType`, `MainGroup`,
  `MiddleGroup`) collected as ignored-but-recognized; a missing `Address`
  column and a missing `Name` column (each a file-level error); and one row
  error each for an unparseable address, an out-of-range address, address `0`,
  a blank name, and an unrecognized boolean. Assert `row` line numbers are
  1-based and count the header, so they match what a spreadsheet displays.

- [ ] **Step 2: Run the tests and watch them fail to compile — the crate does
  not exist yet.**

  Run: `cargo test -p knx-csv`

- [ ] **Step 3: Create the crate and implement the reader.**

  Add `crates/knx-csv` to the workspace `members` list and `csv = "1"` to
  `[workspace.dependencies]`. The new `Cargo.toml` depends on `knx-core`, `csv`
  and `serde`, all `.workspace = true`, and nothing else.

  Detect the separator by counting `,` versus `;` outside quotes on the header
  line, defaulting to `,` on a tie. Strip a leading BOM. Configure
  `csv::ReaderBuilder` with `flexible(false)` and the detected delimiter; it
  already handles RFC 4180 quoting and both line endings. Map header cells to
  known columns case-insensitively after trimming; collect everything else as
  `IgnoredColumn { name, reason }`, with distinct reasons for "recognized but
  export-only" and "unknown". Booleans accept `true`/`false`/`1`/`0`/`yes`/`no`
  case-insensitively, and an empty cell reads as `None` (meaning "unchanged"),
  not `Some(false)`.

  A missing `Address` or `Name` column, or a completely empty file, is a
  file-level `CsvProblem` with `row: None` and no rows returned.

- [ ] **Step 4: Add the layering rule.**

  In `xtask/src/main.rs`, add a check that `knx-csv` reaches none of
  `knx-store`, `knx-etsproj`, `knx-productdb`, following the existing
  `knx-productdb` check verbatim in shape and message style.

- [ ] **Step 5: Run the focused checks.**

  Run: `cargo test -p knx-csv && cargo run -p xtask -- check-layering && cargo deny check`

  `cargo deny check` must pass unchanged: `csv` and `csv-core` are
  MIT/Unlicense and MIT is already on `deny.toml`'s allowlist. If it does not
  pass, stop and report rather than editing `deny.toml` — the allowlist is
  policy (RESEARCH R6: no GPL crate in the runtime graph).

- [ ] **Step 6: Commit the reader.**

  Run: `git add Cargo.toml Cargo.lock crates/knx-csv xtask && git commit`

### Task 2: `Command::UpdateGroupAddress`

**Files:**
- Modify: `crates/knx-core/src/command.rs`

**Interfaces:**
- Produces: `Command::UpdateGroupAddress { id: GroupAddressId, name: String, central: bool, unfiltered: bool }`
- Inverse: the same variant carrying the entry's previous three values.

This task is independent of Task 1 and touches no file Task 1 touches.

- [ ] **Step 1: Write the failing command tests.**

  In `command.rs`'s test module, next to the existing `RenameGroupRange` tests:
  applying the command changes name/central/unfiltered on the matching entry and
  nothing else; the returned inverse restores all three exactly; an unknown id
  yields `CommandError::GroupAddressNotFound`; and a round trip through
  `CommandStack::do_command` + `undo` + `redo` lands where expected.

- [ ] **Step 2: Run them and watch them fail.**

  Run: `cargo test -p knx-core update_group_address`

- [ ] **Step 3: Implement the variant.**

  Add the variant next to `DeleteGroupAddress` (`command.rs:104`) and its
  `apply` arm next to the `RenameGroupRange` arm (`command.rs:938`), which is
  the closest existing shape: find the entry in
  `installations.first_mut()`, `std::mem::replace` each field, return the
  variant carrying the previous values. Do not touch `address` or `range`, and
  do not validate the name — that belongs to the layer with a user to report to
  (see the spec §5).

- [ ] **Step 4: Run the focused tests.**

  Run: `cargo test -p knx-core`

- [ ] **Step 5: Commit the command.**

  Run: `git add crates/knx-core && git commit`

### Task 3: The writer and the import plan

**Files:**
- Create: `crates/knx-csv/src/write.rs`
- Create: `crates/knx-csv/src/plan.rs`
- Modify: `crates/knx-csv/src/lib.rs`
- Create: `crates/knx-csv/tests/roundtrip.rs`

**Interfaces:**
- Produces: `export_group_addresses(project: &Project) -> CsvExport { text, warnings }`
- Produces: `plan_import(project: &Project, parsed: &ParsedCsv) -> ImportPlan { command: Option<Command>, report: CsvImportReport }`
- Consumes: Task 1's `ParsedCsv`, Task 2's `Command::UpdateGroupAddress`.

- [ ] **Step 1: Write the failing writer and plan tests.**

  Writer, over a hand-built `Project`: the header row and column order are
  exactly as the spec's table lists; addresses are formatted in the project's
  own style; a name containing the separator, a quote and a newline round-trips
  through the quoting; `DatapointType` is the unanimous DPT of the linked
  communication objects, empty when there are none, and empty **plus a warning**
  when they disagree; `MainGroup`/`MiddleGroup` carry the containing ranges'
  names; the output starts with a BOM and uses CRLF.

  Plan, over a hand-built `Project`: an unseen address plans a
  `CreateGroupAddress` whose `source` is `KB-GA-<id>` in both fields (matching
  `domain.rs:665`); a differing name plans an `UpdateGroupAddress`; an identical
  row counts as `unchanged` and plans nothing; a file where every row is
  unchanged yields `command: None`; a duplicate address inside one file is an
  error; **any** row-level error yields `command: None` with every other row
  still reported; a new address lands in the innermost containing range, or
  `range: None` with a warning when nothing contains it; and the counts
  (`created`/`updated`/`unchanged`/`rows_read`) add up.

- [ ] **Step 2: Run them and watch them fail.**

  Run: `cargo test -p knx-csv`

- [ ] **Step 3: Implement the writer and the planner.**

  The writer iterates `installations.first()`'s `group_addresses` in their
  existing order. The planner allocates ids with
  `project.ids.next_group_address_id()` in row order and wraps everything in a
  single `Command::Batch`. Neither function mutates the project.

  Range placement: choose the `GroupRange` with the narrowest `start..=end` that
  `contains` the address, so a middle range wins over the main range enclosing
  it.

- [ ] **Step 4: Write the corpus-gated round-trip test.**

  In `tests/roundtrip.rs`, guarded by the standard
  `if !corpus_available() { eprintln!("skip: OriginalData/ corpus not present (gitignored, local-only)"); return; }`
  pattern: import `OriginalData/DemoProjects/Unser Zuhause ets4 - 2025-12-15.knxproj`,
  export its group addresses, re-parse the text, plan the import against the
  same project, and assert the plan is entirely `unchanged` — zero created, zero
  updated, zero problems, `command: None`. Because `knx-csv` must not depend on
  `knx-etsproj`, put this test's `knx-etsproj` dependency in
  `[dev-dependencies]`, and confirm afterwards that `cargo run -p xtask --
  check-layering` still passes (it walks the normal dependency graph, not the
  dev one — verify rather than assume).

- [ ] **Step 5: Run the crate's whole suite.**

  Run: `cargo test -p knx-csv && cargo run -p xtask -- check-layering`

- [ ] **Step 6: Commit the writer and planner.**

  Run: `git add crates/knx-csv && git commit`

### Task 4: Server routes and session-log entries

**Files:**
- Modify: `apps/knx-server/Cargo.toml`
- Modify: `apps/knx-server/src/domain.rs`
- Modify: `apps/knx-server/src/routes.rs`
- Modify: `apps/knx-server/src/session_log.rs`
- Create: `apps/knx-server/tests/http_group_address_csv.rs`

**Interfaces:**
- Produces: `POST /api/group-addresses/csv-export` `{ path }` → `{ warnings }`
- Produces: `POST /api/group-addresses/csv-import` `{ path }` → `{ tree, report }`
- Produces: `session_log::from_csv_import_report(&CsvImportReport) -> Vec<LogEntry>`
- Consumes: Task 3's three public functions.

- [ ] **Step 1: Write the failing route tests.**

  In `http_group_address_csv.rs`, against a state holding a small in-memory
  project: export writes a file whose first data row matches the project;
  importing that same file reports all-unchanged; importing a file with one new
  and one renamed address returns `created: 1, updated: 1` and a `tree` that
  reflects both; importing a file with a bad row returns 400 with the row number
  in the body and leaves the project untouched; and both operations append
  session-log entries without clearing the ones already there.

- [ ] **Step 2: Run them and watch them fail.**

  Run: `cargo test -p knx-server --test http_group_address_csv`

- [ ] **Step 3: Implement the domain functions and routes.**

  Two `*_impl` functions in `domain.rs` modelled on `export_project` and
  `create_group_address_impl`: lock `state.project`, call the pure `knx-csv`
  function, apply the planned command through the existing `apply` helper, write
  or read the file through the same path-resolution helper `/api/project/export`
  uses. A rejected file is `ApiError::bad_request`, never a 500.

  `from_csv_import_report` follows `from_import_report`'s shape exactly: one
  `LogEntry` per problem in report order, `source: "csv-import:<kind>"`,
  `location` carrying `row <n>`, plus one summary entry. Do **not** call
  `SessionLog::reset` — only opening a whole project resets the log.

- [ ] **Step 4: Run the focused tests.**

  Run: `cargo test -p knx-server`

- [ ] **Step 5: Commit the HTTP surface.**

  Run: `git add apps/knx-server && git commit`

### Task 5: CLI subcommands

**Files:**
- Modify: `apps/knx-cli/Cargo.toml`
- Modify: `apps/knx-cli/src/main.rs`
- Create or modify: `apps/knx-cli/tests/cli_group_address_csv.rs`

**Interfaces:**
- Produces: `knx ga-export <store.knxdb> <out.csv>`
- Produces: `knx ga-import <store.knxdb> <in.csv> [--dry-run]`
- Consumes: Task 3's functions plus `knx_store::load_project`/`save_project`.

This task shares no file with Task 6 and may run alongside it.

- [ ] **Step 1: Write the failing CLI tests.**

  Build a tiny `.knxdb` in a temp dir, export it, assert the file's header and
  one data row; re-import it and assert the "nothing to do" summary; import a
  file with one bad row and assert exit code 2 and that the store is unchanged;
  and assert `--dry-run` prints the same report but leaves the store untouched.

- [ ] **Step 2: Run them and watch them fail.**

  Run: `cargo test -p knx-cli`

- [ ] **Step 3: Implement the two subcommands.**

  Follow the existing hand-rolled `parse_*_args` pattern — this CLI has no
  `clap` and is not getting one. Add both lines to the `USAGE` const. Reuse
  `EXIT_IMPORTED_WITH_ERRORS = 2` for a row-rejected file, and print the report
  as human-readable lines, not JSON.

- [ ] **Step 4: Run the focused tests.**

  Run: `cargo test -p knx-cli`

- [ ] **Step 5: Commit the CLI.**

  Run: `git add apps/knx-cli && git commit`

### Task 6: Web UI

**Files:**
- Modify: `apps/knx-web/src/api.ts`
- Modify: `apps/knx-web/src/api.test.ts`
- Create: `apps/knx-web/src/GroupAddressCsvButtons.tsx`
- Create: `apps/knx-web/src/GroupAddressCsvButtons.test.tsx`
- Modify: `apps/knx-web/src/App.tsx`

**Interfaces:**
- Produces: `exportGroupAddressCsv(path)` and `importGroupAddressCsv(path)` in `api.ts`
- Consumes: Task 4's two routes.

- [ ] **Step 1: Write the failing API and component tests.**

  `api.test.ts`: both helpers post the expected body to the expected URL and
  surface a 400 body as an error message, matching the file's existing
  conventions.

  `GroupAddressCsvButtons.test.tsx`, with `./api` and `./filePicker` mocked the
  way `CatalogBrowser.test.tsx` mocks its dependencies: the export button calls
  `pickSavePath` then the export helper and reports the warning count; the
  import button calls `pickOpenPath` then the import helper, hands the new tree
  to its `onTree` prop, and reports the created/updated/unchanged summary; a
  cancelled picker (null path) calls nothing; a rejected import surfaces the
  error and never calls `onTree`; both buttons are disabled without a project.

- [ ] **Step 2: Run them and watch them fail.**

  Run: `cd apps/knx-web && npm test -- --run api.test.ts`

- [ ] **Step 3: Implement the helpers and the two buttons.**

  Follow `exportProject`'s existing shape in `api.ts`. `GroupAddressCsvButtons`
  is a small presentational component taking `{ tree, onTree, onMessage }`,
  modelled on `BulkActionToolbar.tsx`; `App.tsx` renders it in the toolbar next
  to "Export to .knxproj…" and passes its existing tree setter and toast
  pusher. The detailed report stays in the Log tab that T11 already built; the
  toast is one line.

- [ ] **Step 4: Run the frontend checks.**

  Run: `cd apps/knx-web && npx tsc --noEmit && npm test -- --run && npm run build`

  Restore `apps/knx-web/dist/.gitkeep` if the build removed it.

- [ ] **Step 5: Commit the UI.**

  Run: `git add apps/knx-web && git commit`

### Task 7: Documentation reconciliation

**Files:**
- Modify: `docs/IMPORT_EXPORT.md`
- Modify: `docs/COMPATIBILITY.md`
- Modify: `docs/GAP_ANALYSIS_ETS.md`
- Modify: `docs/IMPLEMENTATION_STATUS.md`
- Modify: `docs/KNOWN_LIMITATIONS.md`
- Modify: `docs/ROADMAP.md` (only if it names T12 or C2)
- Modify: `.ai/CURRENT_STATE.md` (needs `git add -f`; a local uncommitted
  `.gitignore` edit lists `.ai/`)
- Create: `.ai/logs/2026-09-10_claude_csv_group_address_exchange.md`

- [ ] **Step 1: Write the format down where a user would look for it.**

  A new `docs/IMPORT_EXPORT.md` section specifying KNXBench group-address CSV
  v1: the column table, encoding/BOM/CRLF/separator rules, the four row
  outcomes, the all-or-nothing rule, and what import never does. State plainly
  that no verified ETS CSV export sample exists in the repository or in the KNX
  Standard v3.0.0 corpus, so this is KNXBench's own format and interoperability
  with ETS is **unverified** — and name the upgrade path (supply a real sample,
  add a column profile).

- [ ] **Step 2: Close C2 and T12 with evidence, not adjectives.**

  In `GAP_ANALYSIS_ETS.md`, rewrite the C2 row and strike through the T12
  backlog entry the way T10 and T11 were closed, naming the crate, the routes,
  the CLI subcommands and the test names. Every count you write down must be
  re-derived from the code at that moment (`cargo test -p knx-csv -- --list`,
  `grep -c`), not copied from a subagent's report.

- [ ] **Step 3: Record the limitations honestly.**

  New `KNOWN_LIMITATIONS.md` entries for: unverified ETS CSV interoperability;
  no re-addressing by CSV; datapoint types are export-only; no
  `Description`/`Comment` columns because the domain model has no such fields;
  no group-range creation; import never deletes; and the German-locale Excel
  hazard (Excel writes `;` and reads `,` depending on the OS list separator —
  the importer detects both, but a file saved from Excel may still surprise
  someone).

- [ ] **Step 4: Run every relevant gate.**

  Run: `cargo fmt --all --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && cargo run -p xtask -- check-layering && cargo deny check && cd apps/knx-web && npx tsc --noEmit && npm test -- --run && npm run build`

- [ ] **Step 5: Commit the documentation.**

  Run: `git add docs && git add -f .ai && git commit`

## Plan self-review

- **Spec coverage:** Task 1 the reader (§3, §4 errors), Task 2 the one missing
  command (§5), Task 3 the writer and the create/update plan (§3 export columns,
  §4 outcomes, §8 round trip), Task 4 the HTTP surface and session log (§7),
  Task 5 the CLI (§7), Task 6 the web buttons (§7), Task 7 the format
  documentation and the honesty statement (§1, §9).
- **Ordering:** Tasks 1 and 2 are independent of each other; 3 needs both; 4
  needs 3; 5 and 6 need 3 and 4 respectively and touch disjoint files; 7 needs
  everything.
- **Placeholder scan:** no task depends on an unverified external format. The
  one place where guessing would have been tempting — ETS's own column names —
  is resolved by defining our own format and saying so, per the spec §1.
- **Type consistency:** `ParsedCsv` is produced in Task 1 and consumed in Task 3
  only; `ImportPlan`/`CsvImportReport` are produced in Task 3 and consumed in
  Tasks 4, 5, 6; `Command::UpdateGroupAddress` is produced in Task 2 and
  constructed only by Task 3's planner.
